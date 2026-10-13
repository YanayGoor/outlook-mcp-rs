//! Pure (COM-free) logic behind the `user_is_required` / `user_is_optional`
//! event fields (issue #44): whether the calendar's owner is invited as a
//! required or an optional attendee, matching by address (never by display
//! name) and looking through distribution lists, including nested ones.
//!
//! The COM side (`client.rs`) only reads the raw data, an item's
//! `Recipients` and a list's `AddressEntry.Members`, and hands it to these
//! functions.

use std::collections::{HashMap, HashSet};

use crate::constants::{OL_RECIPIENT_OPTIONAL, OL_RECIPIENT_REQUIRED};

/// How deep nested distribution lists are followed (a list in a list in a
/// list is depth 3). Deeper members are not looked at.
pub const MAX_LIST_DEPTH: usize = 3;
/// Upper bound on the members read while expanding one top-level list,
/// nested lists included, so one huge list can't stall `list_events`.
pub const MAX_LIST_MEMBERS: usize = 5000;

/// One entry of an item's `Recipients` collection, or one member of a list.
#[derive(Debug, Clone, PartialEq)]
pub struct Attendee {
    /// `Recipient.Type`: 1 = required, 2 = optional, 3 = resource. Members
    /// of a list carry their list's type.
    pub kind: i32,
    /// `Recipient.Address` / `AddressEntry.Address`: the SMTP address, or
    /// the Exchange legacy DN for an Exchange entry.
    pub address: String,
    /// True for a distribution list (Exchange or personal contact group).
    pub is_list: bool,
}

/// The addresses that identify the calendar's owner (the current user, or
/// the `calendar_of` person): typically the Exchange legacy DN and the
/// primary SMTP address. Compared case-insensitively and trimmed.
#[derive(Debug, Clone, Default)]
pub struct Identity {
    addresses: HashSet<String>,
}

impl Identity {
    pub fn new<I: IntoIterator<Item = S>, S: AsRef<str>>(addresses: I) -> Self {
        Self {
            addresses: addresses
                .into_iter()
                .map(|a| a.as_ref().trim().to_lowercase())
                .filter(|a| !a.is_empty())
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.addresses.is_empty()
    }

    pub fn matches(&self, address: &str) -> bool {
        let a = address.trim().to_lowercase();
        !a.is_empty() && self.addresses.contains(&a)
    }
}

/// `user_is_required` / `user_is_optional` for one event. Exactly one is
/// true, or neither (the owner isn't found among the attendees, e.g. an
/// invite forwarded to them or a list that can't be expanded).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UserRole {
    pub user_is_required: bool,
    pub user_is_optional: bool,
}

/// Remembers which lists contain the owner, so a week of meetings sent to
/// the same list expands it once. Lives for one tool call.
#[derive(Debug, Default)]
pub struct ListCache {
    contains: HashMap<String, bool>,
}

/// The owner's role on one event.
///
/// - A personal appointment (no attendees) or a meeting the owner organizes
///   counts as required.
/// - Otherwise a direct match on a required (optional) recipient makes the
///   owner required (optional), as does membership of a required (optional)
///   distribution list. Required wins if both apply.
///
/// `members_of(list_address)` returns a list's members, or `None` if it
/// can't be expanded (e.g. a dynamic list, or no permission). It's called
/// only for lists that a direct match hasn't already decided.
pub fn user_role(
    is_meeting: bool,
    organizer_is_owner: bool,
    attendees: &[Attendee],
    owner: &Identity,
    cache: &mut ListCache,
    mut members_of: impl FnMut(&str) -> Option<Vec<Attendee>>,
) -> UserRole {
    if !is_meeting || organizer_is_owner {
        return UserRole { user_is_required: true, user_is_optional: false };
    }
    let direct = |kind: i32| {
        attendees.iter().any(|a| a.kind == kind && !a.is_list && owner.matches(&a.address))
    };
    let mut required = direct(OL_RECIPIENT_REQUIRED);
    let mut optional = !required && direct(OL_RECIPIENT_OPTIONAL);
    for kind in [OL_RECIPIENT_REQUIRED, OL_RECIPIENT_OPTIONAL] {
        if required || (optional && kind == OL_RECIPIENT_OPTIONAL) {
            break;
        }
        let in_list = attendees
            .iter()
            .filter(|a| a.kind == kind && a.is_list)
            .any(|a| list_contains(&a.address, owner, cache, &mut members_of));
        if in_list {
            if kind == OL_RECIPIENT_REQUIRED {
                required = true;
                optional = false;
            } else {
                optional = true;
            }
        }
    }
    UserRole { user_is_required: required, user_is_optional: optional }
}

/// Whether the list at `address` contains the owner, directly or through
/// nested lists (up to [`MAX_LIST_DEPTH`] and [`MAX_LIST_MEMBERS`]).
/// Answers are cached per list address; a list being expanded is never
/// re-entered, so a cycle of lists terminates.
pub fn list_contains(
    address: &str,
    owner: &Identity,
    cache: &mut ListCache,
    members_of: &mut impl FnMut(&str) -> Option<Vec<Attendee>>,
) -> bool {
    let mut budget = MAX_LIST_MEMBERS;
    let mut visiting = HashSet::new();
    contains_inner(address, owner, cache, members_of, 1, &mut budget, &mut visiting)
}

fn contains_inner(
    address: &str,
    owner: &Identity,
    cache: &mut ListCache,
    members_of: &mut impl FnMut(&str) -> Option<Vec<Attendee>>,
    depth: usize,
    budget: &mut usize,
    visiting: &mut HashSet<String>,
) -> bool {
    let key = address.trim().to_lowercase();
    if let Some(&hit) = cache.contains.get(&key) {
        return hit;
    }
    if depth > MAX_LIST_DEPTH || !visiting.insert(key.clone()) {
        return false;
    }
    let members = members_of(address).unwrap_or_default();
    let mut found = false;
    let mut complete = true;
    // Direct members first: the cheap answer for the common case.
    for m in &members {
        if *budget == 0 {
            complete = false;
            break;
        }
        *budget -= 1;
        if !m.is_list && owner.matches(&m.address) {
            found = true;
            break;
        }
    }
    if !found {
        for m in members.iter().filter(|m| m.is_list) {
            if *budget == 0 || depth == MAX_LIST_DEPTH {
                complete = false;
                break;
            }
            if contains_inner(&m.address, owner, cache, members_of, depth + 1, budget, visiting) {
                found = true;
                break;
            }
        }
    }
    visiting.remove(&key);
    // A partial "no" (cut off by a cap) isn't cached, so another top-level
    // list with fresh budget can still look.
    if found || complete {
        cache.contains.insert(key, found);
    }
    found
}

/// How many attendees an Outlook `RequiredAttendees` / `OptionalAttendees`
/// string names (`"; "`-separated display names).
pub fn attendee_count(attendees: &str) -> usize {
    attendees.split(';').filter(|s| !s.trim().is_empty()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "/o=ExchangeLabs/ou=Exchange/cn=Recipients/cn=me";

    fn me() -> Identity {
        Identity::new([ME, "Me@Example.com"])
    }

    fn person(kind: i32, address: &str) -> Attendee {
        Attendee { kind, address: address.into(), is_list: false }
    }

    fn list(kind: i32, address: &str) -> Attendee {
        Attendee { kind, address: address.into(), is_list: true }
    }

    fn no_lists(_: &str) -> Option<Vec<Attendee>> {
        panic!("no list should be expanded")
    }

    fn role(attendees: &[Attendee], lists: &[(&str, Vec<Attendee>)]) -> UserRole {
        let mut cache = ListCache::default();
        user_role(true, false, attendees, &me(), &mut cache, |a| {
            lists.iter().find(|(k, _)| *k == a).map(|(_, m)| m.clone())
        })
    }

    const REQ: UserRole = UserRole { user_is_required: true, user_is_optional: false };
    const OPT: UserRole = UserRole { user_is_required: false, user_is_optional: true };
    const NEITHER: UserRole = UserRole { user_is_required: false, user_is_optional: false };

    #[test]
    fn personal_appointment_and_own_meeting_are_required() {
        let mut cache = ListCache::default();
        assert_eq!(user_role(false, false, &[], &me(), &mut cache, no_lists), REQ);
        let others = [person(1, "a@x.com")];
        assert_eq!(user_role(true, true, &others, &me(), &mut cache, no_lists), REQ);
    }

    #[test]
    fn direct_match_by_address_caselessly() {
        assert_eq!(role(&[person(1, "a@x.com"), person(1, "ME@example.com")], &[]), REQ);
        assert_eq!(role(&[person(1, "a@x.com"), person(2, &ME.to_uppercase())], &[]), OPT);
        assert_eq!(role(&[person(1, "a@x.com"), person(3, "me@example.com")], &[]), NEITHER);
        assert_eq!(role(&[person(1, "Me")], &[]), NEITHER, "display names never match");
    }

    #[test]
    fn required_wins_over_optional() {
        let a = [person(2, "me@example.com"), person(1, ME)];
        assert_eq!(role(&a, &[]), REQ);
        let a = [person(2, "me@example.com"), list(1, "team@x.com")];
        assert_eq!(role(&a, &[("team@x.com", vec![person(1, ME)])]), REQ);
    }

    #[test]
    fn membership_through_a_list_takes_the_list_type() {
        let lists = [("team@x.com", vec![person(1, "a@x.com"), person(1, "me@example.com")])];
        assert_eq!(role(&[list(1, "team@x.com")], &lists), REQ);
        assert_eq!(role(&[list(2, "team@x.com")], &lists), OPT);
        assert_eq!(role(&[list(2, "other@x.com")], &lists), NEITHER, "unexpandable list");
    }

    #[test]
    fn nested_lists_are_followed_to_the_depth_cap() {
        let lists = [
            ("l1", vec![list(1, "l2")]),
            ("l2", vec![list(1, "l3")]),
            ("l3", vec![person(1, ME)]),
        ];
        assert_eq!(role(&[list(1, "l1")], &lists), REQ);
        let lists = [
            ("l0", vec![list(1, "l1")]),
            ("l1", vec![list(1, "l2")]),
            ("l2", vec![list(1, "l3")]),
            ("l3", vec![person(1, ME)]),
        ];
        assert_eq!(role(&[list(1, "l0")], &lists), NEITHER);
    }

    #[test]
    fn list_cycles_terminate() {
        let lists = [("a", vec![list(1, "b")]), ("b", vec![list(1, "a")])];
        assert_eq!(role(&[list(1, "a")], &lists), NEITHER);
    }

    #[test]
    fn a_list_is_expanded_once_per_cache() {
        let mut calls = 0;
        let mut cache = ListCache::default();
        let mut members = |_: &str| {
            calls += 1;
            Some(vec![person(1, ME)])
        };
        for _ in 0..3 {
            let r = user_role(true, false, &[list(2, "Team@x.com")], &me(), &mut cache, &mut members);
            assert_eq!(r, OPT);
        }
        let r = user_role(true, false, &[list(1, "team@X.com")], &me(), &mut cache, &mut members);
        assert_eq!(r, REQ);
        assert_eq!(calls, 1);
    }

    #[test]
    fn a_direct_match_skips_list_expansion() {
        let a = [person(1, ME), list(1, "team@x.com"), list(2, "team@x.com")];
        let mut cache = ListCache::default();
        assert_eq!(user_role(true, false, &a, &me(), &mut cache, no_lists), REQ);
    }

    #[test]
    fn member_budget_caps_a_huge_list_without_caching_a_partial_no() {
        let big: Vec<Attendee> =
            (0..MAX_LIST_MEMBERS + 10).map(|i| person(1, &format!("p{i}@x.com"))).collect();
        let mut with_me = big.clone();
        with_me.push(person(1, ME));
        let mut cache = ListCache::default();
        let mut members = |_: &str| Some(with_me.clone());
        assert!(!list_contains("huge", &me(), &mut cache, &mut members));
        assert!(!cache.contains.contains_key("huge"));
    }

    #[test]
    fn empty_identity_matches_nothing() {
        let nobody = Identity::new(["", "  "]);
        assert!(nobody.is_empty());
        assert!(!nobody.matches(""));
    }

    #[test]
    fn counts_attendees_in_outlook_strings() {
        assert_eq!(attendee_count(""), 0);
        assert_eq!(attendee_count("Ada Lovelace"), 1);
        assert_eq!(attendee_count("Ada Lovelace; Bob Smith; "), 2);
    }
}
