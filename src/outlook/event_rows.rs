//! Output shaping for `list_events` (issue #43): which fields each event row
//! carries. Pure, so it's tested without COM.

use serde_json::Value;

use crate::error::ToolError;

/// Every field a `list_events` row can carry, in output order.
pub const EVENT_FIELDS: &[&str] = &[
    "id", "subject", "start", "end", "location", "organizer", "all_day", "is_recurring",
    "is_meeting", "is_canceled", "categories", "show_as", "my_response", "user_is_required",
    "user_is_optional", "required_count", "optional_count", "required_attendees",
    "optional_attendees",
];

/// `compact: true`: enough to triage a calendar, without the attendee strings.
pub const COMPACT_EVENT_FIELDS: &[&str] = &[
    "id", "subject", "start", "end", "organizer", "is_recurring", "my_response", "show_as",
    "user_is_required", "user_is_optional", "required_count", "optional_count",
];

/// Fields that need a walk of each event's `Recipients` (and maybe list
/// expansion); `list_events` skips that work when none of them is asked for.
pub const USER_ROLE_FIELDS: &[&str] = &["user_is_required", "user_is_optional"];

/// The field set `list_events` returns, from its `fields` / `compact`
/// parameters: `None` means every field (the default). Names are trimmed
/// and lowercased; duplicates are dropped; an unknown name is an error that
/// lists the valid ones. Passing both `fields` and `compact` is an error.
pub fn event_fields(fields: Option<Vec<String>>, compact: bool)
    -> Result<Option<Vec<String>>, ToolError> {
    match (fields, compact) {
        (Some(_), true) => Err(ToolError::new(
            "Pass either fields or compact, not both: compact is a preset field list.",
        )),
        (None, true) => Ok(Some(COMPACT_EVENT_FIELDS.iter().map(|f| f.to_string()).collect())),
        (None, false) => Ok(None),
        (Some(fields), false) => {
            let mut out: Vec<String> = Vec::new();
            for f in fields {
                let f = f.trim().to_lowercase();
                if !EVENT_FIELDS.contains(&f.as_str()) {
                    return Err(ToolError::new(format!(
                        "Unknown field {f:?} in fields: valid fields are {}.",
                        EVENT_FIELDS.join(", ")
                    )));
                }
                if !out.contains(&f) {
                    out.push(f);
                }
            }
            if out.is_empty() {
                return Err(ToolError::new(format!(
                    "fields must name at least one field: {}.",
                    EVENT_FIELDS.join(", ")
                )));
            }
            Ok(Some(out))
        }
    }
}

/// Whether the rows need `user_is_required` / `user_is_optional` computed.
pub fn wants_user_role(fields: Option<&[String]>) -> bool {
    fields.is_none_or(|f| f.iter().any(|f| USER_ROLE_FIELDS.contains(&f.as_str())))
}

/// Keeps only `fields` in each row object of `rows` (a JSON array); `None`
/// leaves the rows unchanged.
pub fn select_fields(rows: Value, fields: Option<&[String]>) -> Value {
    let Some(fields) = fields else { return rows };
    match rows {
        Value::Array(items) => Value::Array(
            items.into_iter().map(|row| select_row(row, fields)).collect(),
        ),
        other => select_row(other, fields),
    }
}

fn select_row(row: Value, fields: &[String]) -> Value {
    match row {
        Value::Object(mut map) => {
            map.retain(|k, _| fields.iter().any(|f| f == k));
            Value::Object(map)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn default_is_every_field() {
        assert_eq!(event_fields(None, false).unwrap(), None);
        assert!(wants_user_role(None));
    }

    #[test]
    fn compact_is_the_preset() {
        let f = event_fields(None, true).unwrap().unwrap();
        assert_eq!(f, s(COMPACT_EVENT_FIELDS));
        assert!(!f.contains(&"required_attendees".to_string()));
        assert!(wants_user_role(Some(&f)));
    }

    #[test]
    fn fields_are_normalized_and_validated() {
        let f = event_fields(Some(s(&[" Subject", "id", "subject"])), false).unwrap().unwrap();
        assert_eq!(f, s(&["subject", "id"]));
        assert!(!wants_user_role(Some(&f)));
        let e = event_fields(Some(s(&["subject", "body"])), false).unwrap_err().to_string();
        assert!(e.contains("\"body\"") && e.contains("user_is_required"), "{e}");
        assert!(event_fields(Some(vec![]), false).is_err());
        assert!(event_fields(Some(s(&["id"])), true).is_err());
    }

    #[test]
    fn every_field_is_a_known_name() {
        for f in COMPACT_EVENT_FIELDS.iter().chain(USER_ROLE_FIELDS) {
            assert!(EVENT_FIELDS.contains(f), "{f}");
        }
    }

    #[test]
    fn select_fields_keeps_only_the_named_keys() {
        let rows = json!([{"id": "a", "subject": "s", "location": "l"}, {"id": "b"}]);
        assert_eq!(select_fields(rows.clone(), None), rows);
        let f = s(&["id", "subject"]);
        assert_eq!(select_fields(rows, Some(&f)), json!([{"id": "a", "subject": "s"}, {"id": "b"}]));
        assert_eq!(select_fields(json!({"id": "a", "x": 1}), Some(&f)), json!({"id": "a"}));
    }
}
