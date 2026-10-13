//! `find_conflicts` (issue #47): which events in a range overlap. Pure, so
//! it's tested without COM; the client only supplies the events.

use chrono::NaiveDateTime;
use serde::Serialize;

use super::types::EventSummary;
use super::{EventQuery, MAX_EVENT_COUNT};
use crate::error::ToolError;

/// Most events `find_conflicts` reads before giving up on a range (20
/// pages of `list_events`); narrow the range past that.
pub const MAX_CONFLICT_EVENTS: usize = 5000;

/// What `find_conflicts` scans and which events can conflict.
#[derive(Debug, Clone)]
pub struct ConflictQuery {
    pub start_after: Option<String>,
    pub start_before: Option<String>,
    pub calendar_of: Option<String>,
    /// `show_as` values that never conflict (normalized, e.g. ["free"]).
    pub treat_as_free: Vec<String>,
    pub include_declined: bool,
    pub include_all_day: bool,
}

/// A maximal set of events whose times overlap transitively.
#[derive(Debug, Clone, Serialize)]
pub struct ConflictGroup {
    /// The earliest start and latest end in the group.
    pub start: String,
    pub end: String,
    pub events: Vec<EventSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConflictResult {
    pub groups: Vec<ConflictGroup>,
    /// How many events in the range were checked (canceled ones excluded).
    pub events_checked: usize,
}

fn parse(value: Option<&str>) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value?, "%Y-%m-%dT%H:%M:%S").ok()
}

/// Whether an event takes part in conflict detection.
fn can_conflict(e: &EventSummary, q: &ConflictQuery) -> bool {
    !e.is_canceled
        && !q.treat_as_free.iter().any(|f| f.eq_ignore_ascii_case(&e.show_as))
        && (q.include_declined || e.my_response != "declined")
        && (q.include_all_day || !e.all_day)
}

/// Groups the events that overlap. Events that can't conflict (see
/// [`can_conflict`]) or have no readable start/end are ignored. Touching
/// events (one ends exactly when the next starts) don't overlap. Groups
/// come back in start order, events within a group too; events that
/// overlap nothing aren't returned.
pub fn conflict_groups(events: Vec<EventSummary>, q: &ConflictQuery) -> Vec<ConflictGroup> {
    let mut timed: Vec<(NaiveDateTime, NaiveDateTime, EventSummary)> = events
        .into_iter()
        .filter(|e| can_conflict(e, q))
        .filter_map(|e| Some((parse(e.start.as_deref())?, parse(e.end.as_deref())?, e)))
        .filter(|(start, end, _)| end > start)
        .collect();
    timed.sort_by_key(|(start, end, _)| (*start, *end));
    let mut groups: Vec<(NaiveDateTime, NaiveDateTime, Vec<EventSummary>)> = Vec::new();
    for (start, end, e) in timed {
        match groups.last_mut() {
            Some((_, group_end, members)) if start < *group_end => {
                *group_end = (*group_end).max(end);
                members.push(e);
            }
            _ => groups.push((start, end, vec![e])),
        }
    }
    let fmt = |t: NaiveDateTime| t.format("%Y-%m-%dT%H:%M:%S").to_string();
    groups
        .into_iter()
        .filter(|(_, _, members)| members.len() > 1)
        .map(|(start, end, events)| ConflictGroup { start: fmt(start), end: fmt(end), events })
        .collect()
}

/// Reads every event in the range through `list_events` (page by page,
/// canceled meetings skipped, user roles computed) and groups the conflicts.
pub fn find_conflicts(
    q: &ConflictQuery,
    mut list_events: impl FnMut(EventQuery) -> Result<Vec<EventSummary>, ToolError>,
) -> Result<ConflictResult, ToolError> {
    let mut events = Vec::new();
    loop {
        let page = list_events(EventQuery {
            start_after: q.start_after.clone(),
            start_before: q.start_before.clone(),
            calendar_of: q.calendar_of.clone(),
            include_canceled: false,
            user_role: true,
            count: MAX_EVENT_COUNT,
            offset: events.len() as i32,
            ..EventQuery::default()
        })?;
        let full = page.len() >= MAX_EVENT_COUNT as usize;
        events.extend(page);
        if !full {
            break;
        }
        if events.len() >= MAX_CONFLICT_EVENTS {
            return Err(ToolError::new(format!(
                "More than {MAX_CONFLICT_EVENTS} events in this range: narrow start_after / start_before."
            )));
        }
    }
    let events_checked = events.len();
    Ok(ConflictResult { groups: conflict_groups(events, q), events_checked })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: &str, start: &str, end: &str) -> EventSummary {
        EventSummary {
            id: id.into(), occurrence_id: None, subject: id.into(),
            start: Some(format!("2026-10-14T{start}:00")), end: Some(format!("2026-10-14T{end}:00")),
            location: String::new(), organizer: String::new(), all_day: false, is_recurring: false,
            is_meeting: true, is_canceled: false, categories: vec![], show_as: "busy".into(),
            my_response: "accepted".into(), user_is_required: true, user_is_optional: false,
            required_count: 2, optional_count: 0, required_attendees: String::new(),
            optional_attendees: String::new(),
        }
    }

    fn q() -> ConflictQuery {
        ConflictQuery {
            start_after: None, start_before: None, calendar_of: None,
            treat_as_free: vec!["free".into()], include_declined: false, include_all_day: false,
        }
    }

    fn ids(groups: &[ConflictGroup]) -> Vec<Vec<String>> {
        groups.iter().map(|g| g.events.iter().map(|e| e.id.clone()).collect()).collect()
    }

    #[test]
    fn overlapping_events_group_transitively() {
        let events = vec![
            ev("c", "10:30", "11:30"),
            ev("a", "09:00", "10:00"),
            ev("b", "09:30", "10:45"),
            ev("lone", "12:00", "13:00"),
            ev("d", "14:00", "15:00"),
            ev("e", "14:15", "14:30"),
        ];
        let groups = conflict_groups(events, &q());
        assert_eq!(ids(&groups), vec![vec!["a", "b", "c"], vec!["d", "e"]]);
        assert_eq!(groups[0].start, "2026-10-14T09:00:00");
        assert_eq!(groups[0].end, "2026-10-14T11:30:00");
        assert_eq!(groups[1].end, "2026-10-14T15:00:00");
    }

    #[test]
    fn back_to_back_events_do_not_conflict() {
        let events = vec![ev("a", "09:00", "10:00"), ev("b", "10:00", "11:00")];
        assert!(conflict_groups(events, &q()).is_empty());
    }

    #[test]
    fn free_declined_canceled_and_all_day_events_are_skipped_by_default() {
        let base = || ev("busy", "09:00", "10:00");
        let mut free = ev("free", "09:00", "10:00");
        free.show_as = "Free".into();
        let mut declined = ev("declined", "09:00", "10:00");
        declined.my_response = "declined".into();
        let mut canceled = ev("canceled", "09:00", "10:00");
        canceled.is_canceled = true;
        let mut all_day = ev("all_day", "00:00", "23:59");
        all_day.all_day = true;
        for other in [free.clone(), declined.clone(), canceled.clone(), all_day.clone()] {
            assert!(conflict_groups(vec![base(), other], &q()).is_empty());
        }
        let mut wide = q();
        wide.include_declined = true;
        wide.include_all_day = true;
        wide.treat_as_free = vec![];
        for other in [free, declined, all_day] {
            assert_eq!(conflict_groups(vec![base(), other], &wide).len(), 1);
        }
        assert!(conflict_groups(vec![base(), canceled], &wide).is_empty(), "canceled never conflicts");
    }

    #[test]
    fn events_without_times_are_ignored() {
        let mut no_start = ev("x", "09:00", "10:00");
        no_start.start = None;
        assert!(conflict_groups(vec![ev("a", "09:00", "10:00"), no_start], &q()).is_empty());
    }

    #[test]
    fn find_conflicts_pages_through_list_events() {
        let page = MAX_EVENT_COUNT as usize;
        let all: Vec<EventSummary> = (0..page + 3)
            .map(|i| ev(&format!("e{i}"), &format!("{:02}:00", i % 24), &format!("{:02}:30", i % 24)))
            .collect();
        let mut queries = Vec::new();
        let result = find_conflicts(&q(), |eq| {
            queries.push((eq.offset, eq.include_canceled, eq.user_role));
            let from = (eq.offset as usize).min(all.len());
            Ok(all[from..(from + eq.count as usize).min(all.len())].to_vec())
        })
        .unwrap();
        assert_eq!(queries, vec![(0, false, true), (page as i32, false, true)]);
        assert_eq!(result.events_checked, page + 3);
        // Same slot every 24 events: every hour holds a conflict.
        assert_eq!(result.groups.len(), 24);
    }

    #[test]
    fn find_conflicts_refuses_a_huge_range() {
        let page = vec![ev("a", "09:00", "10:00"); MAX_EVENT_COUNT as usize];
        let err = find_conflicts(&q(), |_| Ok(page.clone())).unwrap_err().to_string();
        assert!(err.contains("narrow"), "{err}");
    }
}
