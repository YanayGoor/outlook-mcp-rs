pub mod client;
pub mod com;
pub mod fake;
pub mod types;

use crate::error::ToolError;
use serde_json::Value;
use types::*;

/// All filters for `list_emails`. All optional except `folder`/`count`
/// (which the server fills with defaults). Supplying several ANDs them.
#[derive(Debug, Clone)]
pub struct EmailQuery {
    pub query: Option<String>,
    pub folder: String,
    pub count: i32,
    pub unread_only: bool,
    pub from: Option<String>,
    pub category: Option<String>,
    pub received_after: Option<String>,
    pub received_before: Option<String>,
    pub since_days: Option<i32>,
    pub has_attachments: Option<bool>,
    pub flagged: bool,
    pub high_importance: bool,
}

/// All changes `update_email` can apply to one existing email. Every field
/// except `email_id` is optional; supplying several applies all of them.
/// State changes are applied first and `move_to` last (Move changes the
/// EntryID, so it must come after everything that addresses the item by id).
#[derive(Debug, Clone, Default)]
pub struct EmailUpdate {
    pub email_id: String,
    pub move_to: Option<String>,
    pub mark_read: Option<bool>,
    pub flag: Option<String>,               // "follow_up" | "complete" | "clear"
    pub add_categories: Option<Vec<String>>,
    pub remove_categories: Option<Vec<String>>,
    pub importance: Option<String>,         // "low" | "normal" | "high"
}

/// One image to embed in an HTML body as a hidden Content-ID attachment
/// (`send_email` / `create_draft` `inline_images`). The body references it
/// as `<img src="cid:CONTENT_ID">`. Also the tool-layer argument type, so
/// the field docs below are the public schema descriptions.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct InlineImage {
    /// Content-ID the HTML body references as <img src="cid:CONTENT_ID">, e.g. "logo".
    pub content_id: String,
    /// Local image file path. Give exactly one of `path` or `data_base64`.
    #[serde(default)]
    pub path: Option<String>,
    /// Base64-encoded image bytes (a "data:image/png;base64," prefix is
    /// accepted). Give exactly one of `path` or `data_base64`.
    #[serde(default)]
    pub data_base64: Option<String>,
    /// Attachment file name; defaults to the path's file name, or
    /// CONTENT_ID plus an extension.
    #[serde(default)]
    pub filename: Option<String>,
    /// MIME type like "image/png"; guessed from the file name or the data
    /// when omitted.
    #[serde(default)]
    pub mime_type: Option<String>,
}

/// Where a validated inline image's bytes come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineImageSource {
    /// An existing local file, attached as-is.
    Path(String),
    /// Decoded base64 bytes; the COM layer writes them to a temp file first,
    /// since `Attachments.Add` only takes a path.
    Data(Vec<u8>),
}

/// An [`InlineImage`] after [`validate_inline_images`]: normalized
/// content id, resolved bytes source, safe filename and MIME type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedInlineImage {
    pub content_id: String,
    pub source: InlineImageSource,
    pub filename: String,
    pub mime_type: String,
}

/// All changes `update_task` can apply to one existing task. Every field
/// except `task_id` is optional; supplying several applies all of them.
/// `mark_complete: Some(true)` replaces the retired standalone
/// `complete_task` tool (`= update_task` with `mark_complete: true`);
/// `Some(false)` reopens a completed task, filling the "can't reopen" gap
/// the old `complete_task` had no way to close. `mark_complete` is applied
/// *last*, after every other field write (`MarkComplete()`/reopen both set
/// `PercentComplete` themselves) — so combining `percent_complete` with
/// `mark_complete: Some(false)` in one call silently resets
/// `percent_complete` to 0 regardless of the value supplied; set it in a
/// separate call afterward if a specific non-zero value should stick.
#[derive(Debug, Clone, Default)]
pub struct TaskUpdate {
    pub task_id: String,
    pub mark_complete: Option<bool>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub due_date: Option<String>,
    pub start_date: Option<String>,
    pub importance: Option<String>,
    pub add_categories: Option<Vec<String>>,
    pub remove_categories: Option<Vec<String>>,
    pub percent_complete: Option<i32>,
    pub reminder_time: Option<String>,
}

/// All changes `update_note` can apply to one existing note. Every field
/// except `note_id` is optional; supplying several applies all of them.
#[derive(Debug, Clone, Default)]
pub struct NoteUpdate {
    pub note_id: String,
    pub body: Option<String>,
    pub add_categories: Option<Vec<String>>,
    pub remove_categories: Option<Vec<String>>,
    pub color: Option<String>,
}

/// All filters for `list_events`. Every field is optional; supplying several
/// ANDs them. `start_date`/`end_date` bound the (recurrence-expanded) scan;
/// the rest filter the streamed events client-side. `calendar_of` (an
/// email/name) opens another person's shared calendar instead of your own.
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub query: Option<String>,                 // text match on subject + location
    pub category: Option<String>,
    pub show_as: Option<String>,               // "free"|"tentative"|"busy"|"out_of_office"|"working_elsewhere"
    pub my_response: Option<String>,           // "organizer"|"accepted"|"declined"|"tentative"|"not_responded"
    pub attendees: Option<Vec<String>>,        // match events where ANY listed person participates
    pub attendee_role: Option<String>,         // "required"|"optional"|"any" (default "any")
    pub meetings_only: bool,
    pub all_day: Option<bool>,
    pub calendar_of: Option<String>,
}

/// All filters for `list_tasks`. Every field is optional except
/// `include_completed`; supplying several ANDs them. `include_completed`
/// drives a server-side `Restrict`; the rest filter the streamed tasks
/// client-side (there's no established DASL text-search path for the Tasks
/// folder in this codebase, unlike email's `@SQL` queries — same approach
/// `EventQuery`'s `query`/`category` already use). `query` matches either
/// the subject or the real task body, read per-item — same as `NoteQuery`'s
/// `query` below.
#[derive(Debug, Clone, Default)]
pub struct TaskQuery {
    pub include_completed: bool,
    pub category: Option<String>,
    pub importance: Option<String>,
    pub query: Option<String>, // text match on subject OR body
}

/// All filters for `list_notes`. Both fields optional; supplying both ANDs
/// them. A note's *only* content is its body (it has no separate subject),
/// so `note_matches` reads the real body text to match `query` — the same
/// approach `TaskQuery`'s `query` above now uses alongside its subject.
#[derive(Debug, Clone, Default)]
pub struct NoteQuery {
    pub category: Option<String>,
    pub query: Option<String>,
}

/// All inputs for `create_event`. `required_attendees`/`optional_attendees`
/// are the two invite tiers Outlook shows a meeting organizer; any attendee
/// in either tier makes the item a meeting. `send` (default true in the
/// tool layer) controls whether a meeting is actually sent to attendees or
/// merely saved for later review — see `create_event_status` below for the
/// resulting status string.
#[derive(Debug, Clone)]
pub struct CreateEventInput {
    pub subject: String,
    pub start: String,
    pub end: String,
    pub body: Option<String>,
    pub location: Option<String>,
    pub required_attendees: Option<Vec<String>>,
    pub optional_attendees: Option<Vec<String>>,
    pub all_day: bool,
    pub reminder_minutes: Option<i32>,
    pub categories: Option<Vec<String>>,
    pub show_as: Option<String>,
    pub send: bool,
    pub recurrence: Option<RecurrenceInput>,
}

/// All changes `update_event` can apply to one existing calendar event. Every
/// field except `event_id` is optional; supplying several applies all of
/// them. There is no `move_to` — events don't change folders in this API —
/// so, unlike `EmailUpdate`, field application order is cosmetic, not a
/// correctness constraint. Adding either attendee tier converts a personal
/// appointment into a meeting. `send_update` (no default here — the tool
/// layer defaults it to `true`) controls whether a meeting's edits are
/// delivered to attendees or applied quietly to your own copy only; a
/// personal (non-meeting) appointment always just saves, regardless.
#[derive(Debug, Clone)]
pub struct EventUpdate {
    pub event_id: String,
    pub subject: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub location: Option<String>,
    pub body: Option<String>,
    pub all_day: Option<bool>,
    pub reminder_minutes: Option<i32>,
    pub show_as: Option<String>,
    pub add_categories: Option<Vec<String>>,
    pub remove_categories: Option<Vec<String>>,
    pub add_required_attendees: Option<Vec<String>>,
    pub add_optional_attendees: Option<Vec<String>>,
    pub remove_attendees: Option<Vec<String>>,
    pub send_update: bool,
    pub recurrence: Option<RecurrenceInput>,
    pub clear_recurrence: bool,
}

/// One recurrence pattern for `create_event`/`update_event`. `pattern`
/// selects which of the other fields matter: `"weekly"` requires
/// `days_of_week`; `"monthly"` requires `day_of_month`; `"yearly"` derives
/// its month/day from the event's own start date (no field needed);
/// `"daily"` needs nothing extra. At most one of `until`/`occurrences` may
/// be set; if neither is set the series has no end date.
#[derive(Debug, Clone)]
pub struct RecurrenceInput {
    pub pattern: String,
    pub interval: Option<i32>,
    pub days_of_week: Option<Vec<String>>,
    pub day_of_month: Option<i32>,
    pub until: Option<String>,
    pub occurrences: Option<i32>,
}

pub trait OutlookClient: Send + Sync {
    fn list_folders(&self) -> Result<Vec<FolderInfo>, ToolError>;
    fn list_emails(&self, q: EmailQuery) -> Result<Vec<EmailSummary>, ToolError>;
    fn get_email(&self, email_id: String, prefer_html: bool)
        -> Result<EmailDetail, ToolError>;
    fn send_email(&self, to: Vec<String>, subject: String, body: String,
        cc: Option<Vec<String>>, bcc: Option<Vec<String>>, html: bool,
        attachments: Option<Vec<String>>, inline_images: Option<Vec<InlineImage>>)
        -> Result<Value, ToolError>;
    fn create_draft(&self, to: Vec<String>, subject: String, body: String,
        cc: Option<Vec<String>>, bcc: Option<Vec<String>>, html: bool,
        attachments: Option<Vec<String>>, inline_images: Option<Vec<InlineImage>>)
        -> Result<Value, ToolError>;
    fn reply_email(&self, email_id: String, body: String, reply_all: bool,
        html: bool, send: bool, attachments: Option<Vec<String>>)
        -> Result<Value, ToolError>;
    fn update_email(&self, u: EmailUpdate) -> Result<Value, ToolError>;
    fn delete_email(&self, email_id: String) -> Result<Value, ToolError>;

    fn list_events(&self, q: EventQuery) -> Result<Vec<EventSummary>, ToolError>;
    fn get_event(&self, event_id: String) -> Result<EventDetail, ToolError>;
    fn create_event(&self, input: CreateEventInput) -> Result<Value, ToolError>;
    fn respond_to_meeting(&self, event_id: String, response: String,
        comment: Option<String>, send: bool) -> Result<Value, ToolError>;
    fn update_event(&self, u: EventUpdate) -> Result<Value, ToolError>;
    fn delete_event(&self, event_id: String, send_cancellation: bool) -> Result<Value, ToolError>;
    fn check_availability(&self, input: CheckAvailabilityInput) -> Result<AvailabilityResult, ToolError>;

    fn list_attachments(&self, email_id: String)
        -> Result<Vec<AttachmentInfo>, ToolError>;
    fn save_attachments(&self, email_id: String, save_dir: String,
        attachment_names: Option<Vec<String>>) -> Result<Vec<Value>, ToolError>;

    fn list_tasks(&self, q: TaskQuery) -> Result<Vec<TaskSummary>, ToolError>;
    fn create_task(&self, subject: String, body: Option<String>,
        due_date: Option<String>, importance: String, categories: Option<Vec<String>>,
        start_date: Option<String>, reminder_time: Option<String>) -> Result<Value, ToolError>;
    fn update_task(&self, u: TaskUpdate) -> Result<Value, ToolError>;
    fn delete_task(&self, task_id: String) -> Result<Value, ToolError>;

    fn list_notes(&self, q: NoteQuery) -> Result<Vec<NoteSummary>, ToolError>;
    fn get_note(&self, note_id: String) -> Result<NoteDetail, ToolError>;
    fn create_note(&self, body: String, categories: Option<Vec<String>>, color: Option<String>) -> Result<Value, ToolError>;
    fn update_note(&self, u: NoteUpdate) -> Result<Value, ToolError>;
    fn delete_note(&self, note_id: String) -> Result<Value, ToolError>;
}

/// The status string `create_event` returns: `"meeting_sent"` (attendees +
/// send), `"meeting_saved"` (attendees + no send), or `"saved"` (no
/// attendees, regardless of `send` — there's nothing to send or withhold).
pub fn create_event_status(has_attendees: bool, send: bool) -> &'static str {
    match (has_attendees, send) {
        (true, true) => "meeting_sent",
        (true, false) => "meeting_saved",
        (false, _) => "saved",
    }
}

/// Resolves `r.pattern` to its `OlRecurrenceType` id and checks the fields
/// each pattern requires. Called first by both `create_event`'s and
/// `update_event`'s real-COM recurrence-writing code, before any COM call is
/// made, so a bad `recurrence` object fails fast with a clear message.
pub fn validate_recurrence(r: &RecurrenceInput) -> Result<i32, ToolError> {
    let recurrence_type = crate::friendly::recurrence_pattern_to_id(&r.pattern).ok_or_else(|| {
        ToolError::new(format!(
            "invalid recurrence.pattern {:?}: expected \"daily\", \"weekly\", \"monthly\", or \"yearly\"",
            r.pattern
        ))
    })?;
    if r.pattern.eq_ignore_ascii_case("weekly")
        && !r.days_of_week.as_ref().is_some_and(|d| !d.is_empty())
    {
        return Err(ToolError::new(
            "recurrence.days_of_week is required for a \"weekly\" pattern",
        ));
    }
    if r.pattern.eq_ignore_ascii_case("monthly") && r.day_of_month.is_none() {
        return Err(ToolError::new(
            "recurrence.day_of_month is required for a \"monthly\" pattern",
        ));
    }
    if r.occurrences.is_some() && r.until.is_some() {
        return Err(ToolError::new(
            "recurrence: specify at most one of \"until\" or \"occurrences\", not both",
        ));
    }
    Ok(recurrence_type)
}

/// The `RecurrenceType`-dependent `Interval` value Outlook's COM object model
/// expects. Every pattern except `"yearly"` passes the user's `interval`
/// straight through ("every N days/weeks/months"). `"yearly"` is the one
/// exception: Outlook's `RecurrencePattern.Interval` is documented as being
/// in **months** for `olRecursYearly`, and must be a multiple of 12 — "every
/// 1 year" is `Interval = 12`, "every 2 years" is `Interval = 24`. Called by
/// `apply_recurrence` (`client.rs`) right after `validate_recurrence`.
pub fn com_recurrence_interval(r: &RecurrenceInput) -> i32 {
    let interval = r.interval.unwrap_or(1);
    if r.pattern.eq_ignore_ascii_case("yearly") {
        interval * 12
    } else {
        interval
    }
}

/// Rejects an `EventUpdate` that sets both `recurrence` and `clear_recurrence`
/// — Outlook has no single COM call that means "replace recurrence and clear
/// it," so exactly one (or neither) must be set. Called by both
/// `update_event` implementors before either field is applied.
pub fn validate_recurrence_update(u: &EventUpdate) -> Result<(), ToolError> {
    if u.recurrence.is_some() && u.clear_recurrence {
        return Err(ToolError::new(
            "cannot set recurrence and clear_recurrence in the same update_event call",
        ));
    }
    Ok(())
}

/// Inverse of [`com_recurrence_interval`]: converts a COM `Interval` value
/// read back from `RecurrencePattern` into the user-facing "every N
/// years/months/..." value. Only `olRecursYearly` (`OL_RECURS_YEARLY`) needs
/// unwinding, since it's the only pattern `com_recurrence_interval` scales.
/// Called by `recurrence_info` (`client.rs`) right after reading `Interval`.
pub fn friendly_recurrence_interval(recurrence_type: i32, com_interval: i32) -> i32 {
    if recurrence_type == crate::constants::OL_RECURS_YEARLY {
        com_interval / 12
    } else {
        com_interval
    }
}

/// All inputs for `check_availability`. `treat_as_free` decides which raw
/// statuses count as "free" when computing `common_free` — it never changes
/// what a person's own `slots` report (those always show the true status).
#[derive(Debug, Clone)]
pub struct CheckAvailabilityInput {
    pub people: Vec<String>,
    pub start: String,
    pub end: String,
    pub interval_minutes: i32,
    pub treat_as_free: Vec<String>,
}

/// Parses Outlook's raw `Recipient.FreeBusy` status-code string (one ASCII
/// digit per `interval_minutes`-sized slot: `'0'` free, `'1'` tentative,
/// `'2'` busy, `'3'` out-of-office, `'4'` working-elsewhere — the exact
/// `OlBusyStatus` numbering `friendly::busy_status_word` already maps) into
/// timestamped slots starting at `start`. `FreeBusy` returns a string
/// covering a much longer range than the caller's `[start, end)` window (it
/// has no `end` parameter), so callers must compute `max_slots` themselves
/// — `(end - start) / interval_minutes`, rounded up — and this function
/// truncates to it. Any digit outside 0-4 (Outlook shouldn't produce one,
/// but the string could be malformed) falls back to `"busy"`, the same
/// catch-all `busy_status_word` uses.
pub fn parse_freebusy_slots(
    raw: &str,
    start: &chrono::NaiveDateTime,
    interval_minutes: i32,
    max_slots: usize,
) -> Vec<AvailabilitySlot> {
    raw.chars()
        .take(max_slots)
        .enumerate()
        .map(|(i, ch)| {
            let code = ch.to_digit(10).map(|d| d as i32).unwrap_or(crate::constants::OL_BUSY);
            let slot_start = *start + chrono::Duration::minutes(i as i64 * interval_minutes as i64);
            let slot_end = slot_start + chrono::Duration::minutes(interval_minutes as i64);
            AvailabilitySlot {
                start: slot_start.format("%Y-%m-%dT%H:%M:%S").to_string(),
                end: slot_end.format("%Y-%m-%dT%H:%M:%S").to_string(),
                status: crate::friendly::busy_status_word(code).to_string(),
            }
        })
        .collect()
}

/// The windows where every **resolved** person's status is in
/// `treat_as_free` (case-insensitive). Unresolved people are skipped
/// entirely — they neither block nor contribute to a common-free window.
/// Assumes all resolved people's `slots` share the same slot boundaries
/// (true whenever they were built from the same `start`/`interval_minutes`,
/// which `check_availability` always uses); intersects only over the
/// shortest `slots` length present, so a person whose raw string was
/// unexpectedly short doesn't panic the lookup.
pub fn common_free(people: &[PersonAvailability], treat_as_free: &[String]) -> Vec<FreeWindow> {
    let resolved: Vec<&PersonAvailability> = people.iter().filter(|p| p.resolved).collect();
    if resolved.is_empty() {
        return Vec::new();
    }
    let treat_lower: Vec<String> = treat_as_free.iter().map(|s| s.to_lowercase()).collect();
    let min_len = resolved.iter().map(|p| p.slots.len()).min().unwrap_or(0);

    let mut windows = Vec::new();
    let mut run_start: Option<usize> = None;
    for i in 0..min_len {
        let all_free = resolved
            .iter()
            .all(|p| treat_lower.contains(&p.slots[i].status.to_lowercase()));
        if all_free {
            run_start.get_or_insert(i);
        } else if let Some(s) = run_start.take() {
            windows.push(FreeWindow {
                start: resolved[0].slots[s].start.clone(),
                end: resolved[0].slots[i - 1].end.clone(),
            });
        }
    }
    if let Some(s) = run_start {
        windows.push(FreeWindow {
            start: resolved[0].slots[s].start.clone(),
            end: resolved[0].slots[min_len - 1].end.clone(),
        });
    }
    windows
}


// ---- inline images (send_email / create_draft) --------------------------

/// `"<cid:logo>"` / `"cid:logo"` / `"logo"` -> `"logo"`.
pub fn normalize_content_id(raw: &str) -> String {
    let mut cid = raw.trim();
    if cid.len() >= 2 && cid.starts_with('<') && cid.ends_with('>') {
        cid = cid[1..cid.len() - 1].trim();
    }
    if cid.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("cid:")) {
        cid = cid[4..].trim();
    }
    cid.to_string()
}

/// Strictly decode inline-image base64, accepting an optional
/// `data:<mime>;base64,` prefix and embedded whitespace/newlines.
pub fn decode_inline_base64(raw: &str, cid: &str) -> Result<Vec<u8>, ToolError> {
    use base64::Engine as _;
    let mut text = raw.trim();
    if text.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("data:")) {
        match text.split_once(',') {
            Some((header, rest)) if header.to_ascii_lowercase().contains(";base64") => text = rest,
            _ => {
                return Err(ToolError::new(format!(
                    "inline image {cid:?}: data URI must be base64-encoded"
                )))
            }
        }
    }
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let data = base64::engine::general_purpose::STANDARD
        .decode(compact.as_bytes())
        .map_err(|_| ToolError::new(format!("inline image {cid:?}: data_base64 is not valid base64")))?;
    if data.is_empty() {
        return Err(ToolError::new(format!("inline image {cid:?}: data_base64 is empty")));
    }
    Ok(data)
}

/// Magic-number sniffing for base64 images given without filename/mime_type.
pub fn sniff_image_mime(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if data.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// MIME type from a file name's extension (common image types only).
pub fn mime_from_filename(filename: &str) -> Option<&'static str> {
    let ext = std::path::Path::new(filename).extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" | "jpe" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "tif" | "tiff" => "image/tiff",
        "ico" => "image/x-icon",
        _ => return None,
    })
}

/// File extension (with the dot) for a MIME type, or `""` if unknown.
pub fn extension_for_mime(mime: &str) -> &'static str {
    match mime.to_ascii_lowercase().as_str() {
        "image/png" => ".png",
        "image/jpeg" | "image/jpg" => ".jpg",
        "image/gif" => ".gif",
        "image/webp" => ".webp",
        "image/bmp" => ".bmp",
        "image/svg+xml" => ".svg",
        "image/tiff" => ".tif",
        "image/x-icon" | "image/vnd.microsoft.icon" => ".ico",
        _ => "",
    }
}

/// Validate every inline image FIRST (so a bad one fails before any COM
/// item is created, sent or saved) and normalize each to a
/// [`ValidatedInlineImage`]. Requires `html`; each entry needs a non-empty
/// content id (unique, case-insensitively) and exactly one of
/// `path` (must exist) / `data_base64` (must decode). The MIME type comes
/// from `mime_type`, else the filename extension, else the data's magic
/// bytes, else `application/octet-stream`.
pub fn validate_inline_images(images: &[InlineImage], html: bool)
    -> Result<Vec<ValidatedInlineImage>, ToolError> {
    if !html {
        return Err(ToolError::new(
            "inline_images requires html=true (reference them in the HTML body as <img src=\"cid:CONTENT_ID\">).",
        ));
    }
    let non_empty = |v: &Option<String>| v.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let mut out = Vec::with_capacity(images.len());
    let mut seen = std::collections::HashSet::new();
    for img in images {
        let cid = normalize_content_id(&img.content_id);
        if cid.is_empty() {
            return Err(ToolError::new("inline image is missing a content_id"));
        }
        if !seen.insert(cid.to_lowercase()) {
            return Err(ToolError::new(format!("duplicate inline image content_id: {cid:?}")));
        }
        let mut filename = non_empty(&img.filename);
        let source = match (non_empty(&img.path), img.data_base64.as_deref().filter(|s| !s.trim().is_empty())) {
            (Some(path), None) => {
                if !std::path::Path::new(&path).is_file() {
                    return Err(ToolError::new(format!("inline image not found: {path}")));
                }
                if filename.is_none() {
                    filename = std::path::Path::new(&path)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned());
                }
                InlineImageSource::Path(path)
            }
            (None, Some(data)) => InlineImageSource::Data(decode_inline_base64(data, &cid)?),
            _ => {
                return Err(ToolError::new(format!(
                    "inline image {cid:?}: give exactly one of 'path' or 'data_base64'"
                )))
            }
        };
        let mime_type = non_empty(&img.mime_type)
            .or_else(|| filename.as_deref().and_then(mime_from_filename).map(str::to_string))
            .or_else(|| match &source {
                InlineImageSource::Data(d) => sniff_image_mime(d).map(str::to_string),
                InlineImageSource::Path(_) => None,
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let filename = filename.unwrap_or_else(|| format!("{cid}{}", extension_for_mime(&mime_type)));
        out.push(ValidatedInlineImage {
            content_id: cid,
            source,
            filename: com::safe_filename(&filename),
            mime_type,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{
        com_recurrence_interval, common_free, create_event_status, friendly_recurrence_interval,
        parse_freebusy_slots, validate_recurrence, validate_recurrence_update, EventUpdate, RecurrenceInput,
    };
    use crate::outlook::types::{AvailabilitySlot, FreeWindow, PersonAvailability};

    #[test]
    fn create_event_status_covers_all_three_outcomes() {
        assert_eq!(create_event_status(true, true), "meeting_sent");
        assert_eq!(create_event_status(true, false), "meeting_saved");
        assert_eq!(create_event_status(false, true), "saved");
        assert_eq!(create_event_status(false, false), "saved");
    }

    fn recurrence(pattern: &str) -> RecurrenceInput {
        RecurrenceInput {
            pattern: pattern.to_string(), interval: None, days_of_week: None,
            day_of_month: None, until: None, occurrences: None,
        }
    }

    #[test]
    fn validate_recurrence_accepts_daily_with_no_extra_fields() {
        assert_eq!(validate_recurrence(&recurrence("daily")).unwrap(), 0);
    }

    #[test]
    fn validate_recurrence_rejects_unknown_pattern() {
        assert!(validate_recurrence(&recurrence("biweekly")).is_err());
    }

    #[test]
    fn validate_recurrence_requires_days_of_week_for_weekly() {
        assert!(validate_recurrence(&recurrence("weekly")).is_err());
        let mut r = recurrence("weekly");
        r.days_of_week = Some(vec!["monday".to_string()]);
        assert_eq!(validate_recurrence(&r).unwrap(), 1);
    }

    #[test]
    fn validate_recurrence_requires_day_of_month_for_monthly() {
        assert!(validate_recurrence(&recurrence("monthly")).is_err());
        let mut r = recurrence("monthly");
        r.day_of_month = Some(15);
        assert_eq!(validate_recurrence(&r).unwrap(), 2);
    }

    #[test]
    fn validate_recurrence_rejects_both_until_and_occurrences() {
        let mut r = recurrence("daily");
        r.until = Some("2099-01-01".to_string());
        r.occurrences = Some(5);
        assert!(validate_recurrence(&r).is_err());
    }

    #[test]
    fn com_recurrence_interval_multiplies_yearly_by_12() {
        assert_eq!(com_recurrence_interval(&recurrence("yearly")), 12); // default interval 1 -> 12
        let mut r = recurrence("yearly");
        r.interval = Some(2);
        assert_eq!(com_recurrence_interval(&r), 24); // every 2 years -> 24 months

        assert_eq!(com_recurrence_interval(&recurrence("daily")), 1);
        let mut r = recurrence("weekly");
        r.interval = Some(3);
        assert_eq!(com_recurrence_interval(&r), 3); // unchanged for non-yearly
    }

    #[test]
    fn friendly_recurrence_interval_divides_yearly_by_12() {
        assert_eq!(
            friendly_recurrence_interval(crate::constants::OL_RECURS_YEARLY, 12),
            1
        );
        assert_eq!(
            friendly_recurrence_interval(crate::constants::OL_RECURS_YEARLY, 24),
            2
        );
        assert_eq!(
            friendly_recurrence_interval(crate::constants::OL_RECURS_DAILY, 1),
            1
        );
        assert_eq!(
            friendly_recurrence_interval(crate::constants::OL_RECURS_WEEKLY, 3),
            3
        );
    }

    fn event_update() -> EventUpdate {
        EventUpdate {
            event_id: "event-1".to_string(),
            subject: None, start: None, end: None, location: None, body: None,
            all_day: None, reminder_minutes: None, show_as: None,
            add_categories: None, remove_categories: None,
            add_required_attendees: None, add_optional_attendees: None, remove_attendees: None,
            send_update: false,
            recurrence: None, clear_recurrence: false,
        }
    }

    #[test]
    fn validate_recurrence_update_rejects_recurrence_and_clear_recurrence_together() {
        let mut u = event_update();
        u.recurrence = Some(recurrence("daily"));
        u.clear_recurrence = true;
        assert!(validate_recurrence_update(&u).is_err());
    }

    #[test]
    fn validate_recurrence_update_accepts_recurrence_only() {
        let mut u = event_update();
        u.recurrence = Some(recurrence("daily"));
        assert!(validate_recurrence_update(&u).is_ok());
    }

    #[test]
    fn validate_recurrence_update_accepts_clear_recurrence_only() {
        let mut u = event_update();
        u.clear_recurrence = true;
        assert!(validate_recurrence_update(&u).is_ok());
    }

    #[test]
    fn validate_recurrence_update_accepts_neither() {
        assert!(validate_recurrence_update(&event_update()).is_ok());
    }

    fn dt(s: &str) -> chrono::NaiveDateTime {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap()
    }

    #[test]
    fn parse_freebusy_slots_maps_codes_to_words_and_times() {
        // "02143" = free, busy, tentative, working_elsewhere, out_of_office
        let slots = parse_freebusy_slots("02143", &dt("2099-01-01T09:00:00"), 30, 5);
        assert_eq!(slots.len(), 5);
        assert_eq!(slots[0], AvailabilitySlot {
            start: "2099-01-01T09:00:00".to_string(),
            end: "2099-01-01T09:30:00".to_string(),
            status: "free".to_string(),
        });
        assert_eq!(slots[1].status, "busy");
        assert_eq!(slots[1].start, "2099-01-01T09:30:00");
        assert_eq!(slots[1].end, "2099-01-01T10:00:00");
        assert_eq!(slots[2].status, "tentative");
        assert_eq!(slots[3].status, "working_elsewhere");
        assert_eq!(slots[4].status, "out_of_office");
    }

    #[test]
    fn parse_freebusy_slots_truncates_to_max_slots() {
        // Outlook's raw FreeBusy string commonly covers a much longer range
        // than the caller's requested [start, end) window.
        let slots = parse_freebusy_slots("000000000000", &dt("2099-01-01T09:00:00"), 30, 3);
        assert_eq!(slots.len(), 3);
    }

    #[test]
    fn parse_freebusy_slots_treats_unrecognized_digit_as_busy() {
        let slots = parse_freebusy_slots("9", &dt("2099-01-01T09:00:00"), 30, 1);
        assert_eq!(slots[0].status, "busy");
    }

    fn avail(person: &str, resolved: bool, statuses: &[&str]) -> PersonAvailability {
        let mut slots = Vec::new();
        for (i, s) in statuses.iter().enumerate() {
            let start = dt("2099-01-01T09:00:00") + chrono::Duration::minutes(i as i64 * 30);
            slots.push(AvailabilitySlot {
                start: start.format("%Y-%m-%dT%H:%M:%S").to_string(),
                end: (start + chrono::Duration::minutes(30)).format("%Y-%m-%dT%H:%M:%S").to_string(),
                status: s.to_string(),
            });
        }
        PersonAvailability { person: person.to_string(), resolved, slots }
    }

    #[test]
    fn common_free_intersects_only_where_everyone_is_free() {
        let people = vec![
            avail("alice", true, &["free", "free", "busy"]),
            avail("bob", true, &["free", "busy", "busy"]),
        ];
        let windows = common_free(&people, &["free".to_string()]);
        assert_eq!(windows, vec![FreeWindow {
            start: "2099-01-01T09:00:00".to_string(),
            end: "2099-01-01T09:30:00".to_string(),
        }]);
    }

    #[test]
    fn common_free_merges_contiguous_free_slots_into_one_window() {
        let people = vec![avail("alice", true, &["free", "free", "busy", "free"])];
        let windows = common_free(&people, &["free".to_string()]);
        assert_eq!(windows, vec![
            FreeWindow { start: "2099-01-01T09:00:00".to_string(), end: "2099-01-01T10:00:00".to_string() },
            FreeWindow { start: "2099-01-01T10:30:00".to_string(), end: "2099-01-01T11:00:00".to_string() },
        ]);
    }

    #[test]
    fn common_free_respects_custom_treat_as_free() {
        let people = vec![avail("alice", true, &["tentative"])];
        assert_eq!(common_free(&people, &["free".to_string()]), vec![]);
        assert_eq!(
            common_free(&people, &["free".to_string(), "tentative".to_string()]).len(),
            1
        );
    }

    #[test]
    fn common_free_ignores_unresolved_people() {
        let people = vec![
            avail("alice", true, &["free"]),
            avail("bob", false, &[]),
        ];
        assert_eq!(common_free(&people, &["free".to_string()]).len(), 1);
    }

    #[test]
    fn common_free_empty_when_no_one_resolved() {
        let people = vec![avail("alice", false, &[])];
        assert_eq!(common_free(&people, &["free".to_string()]), vec![]);
    }
}

#[cfg(test)]
mod inline_image_tests {
    use super::{
        decode_inline_base64, extension_for_mime, mime_from_filename, normalize_content_id,
        sniff_image_mime, validate_inline_images, InlineImage, InlineImageSource,
    };

    /// A 1x1 transparent PNG.
    const PNG_1X1_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

    fn b64(cid: &str, data: &str) -> InlineImage {
        InlineImage { content_id: cid.into(), data_base64: Some(data.into()), ..Default::default() }
    }

    fn temp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("outlook-mcp-rs-unit-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn err(images: &[InlineImage], html: bool) -> String {
        validate_inline_images(images, html).unwrap_err().to_string()
    }

    #[test]
    fn normalize_content_id_strips_brackets_and_cid_prefix() {
        assert_eq!(normalize_content_id("logo"), "logo");
        assert_eq!(normalize_content_id("  <logo> "), "logo");
        assert_eq!(normalize_content_id("cid:logo"), "logo");
        assert_eq!(normalize_content_id("CID: logo"), "logo");
        assert_eq!(normalize_content_id("<cid:logo>"), "logo");
        assert_eq!(normalize_content_id("<>"), "");
        assert_eq!(normalize_content_id("cid:"), "");
    }

    #[test]
    fn decode_accepts_data_uri_prefix_and_whitespace() {
        let plain = decode_inline_base64(PNG_1X1_B64, "x").unwrap();
        let uri = decode_inline_base64(&format!("data:image/png;base64,{PNG_1X1_B64}"), "x").unwrap();
        let spaced = format!(" {}\n{} \r\n", &PNG_1X1_B64[..20], &PNG_1X1_B64[20..]);
        assert_eq!(plain, uri);
        assert_eq!(plain, decode_inline_base64(&spaced, "x").unwrap());
        assert_eq!(sniff_image_mime(&plain), Some("image/png"));
    }

    #[test]
    fn decode_rejects_invalid_or_empty_data() {
        assert!(decode_inline_base64("not base64!!", "x").unwrap_err().to_string().contains("not valid base64"));
        // Missing padding is rejected (strict decoding).
        assert!(decode_inline_base64("aGk", "x").is_err());
        assert!(decode_inline_base64("data:image/png,aGk=", "x").unwrap_err().to_string().contains("data URI"));
        assert!(decode_inline_base64("data:image/png;base64", "x").is_err());
        assert!(decode_inline_base64("data:image/png;base64,", "x").unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn sniff_and_extension_helpers() {
        assert_eq!(sniff_image_mime(b"\xff\xd8\xff\xe0rest"), Some("image/jpeg"));
        assert_eq!(sniff_image_mime(b"GIF89a..."), Some("image/gif"));
        assert_eq!(sniff_image_mime(b"GIF87a..."), Some("image/gif"));
        assert_eq!(sniff_image_mime(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff_image_mime(b"RIFF"), None);
        assert_eq!(sniff_image_mime(b"hello"), None);
        assert_eq!(mime_from_filename("a.PNG"), Some("image/png"));
        assert_eq!(mime_from_filename("a.jpeg"), Some("image/jpeg"));
        assert_eq!(mime_from_filename("noext"), None);
        assert_eq!(mime_from_filename("a.txt"), None);
        assert_eq!(extension_for_mime("image/jpeg"), ".jpg");
        assert_eq!(extension_for_mime("application/octet-stream"), "");
    }

    #[test]
    fn requires_html() {
        assert!(err(&[b64("logo", PNG_1X1_B64)], false).contains("html=true"));
    }

    #[test]
    fn rejects_empty_content_id() {
        assert!(err(&[b64("  <cid:> ", PNG_1X1_B64)], true).contains("missing a content_id"));
    }

    #[test]
    fn requires_exactly_one_source() {
        let neither = InlineImage { content_id: "a".into(), ..Default::default() };
        assert!(err(&[neither], true).contains("exactly one"));
        let path = temp_file("both.png", b"x");
        let both = InlineImage {
            content_id: "a".into(),
            path: Some(path.to_string_lossy().into_owned()),
            data_base64: Some(PNG_1X1_B64.into()),
            ..Default::default()
        };
        assert!(err(&[both], true).contains("exactly one"));
        // Empty strings count as absent.
        let empty = InlineImage { content_id: "a".into(), path: Some("".into()), data_base64: Some(" ".into()), ..Default::default() };
        assert!(err(&[empty], true).contains("exactly one"));
    }

    #[test]
    fn rejects_missing_path() {
        let img = InlineImage { content_id: "a".into(), path: Some("/definitely/not/here.png".into()), ..Default::default() };
        assert!(err(&[img], true).contains("inline image not found"));
    }

    #[test]
    fn rejects_bad_base64() {
        assert!(err(&[b64("a", "@@@")], true).contains("not valid base64"));
    }

    #[test]
    fn rejects_duplicate_ids_case_insensitively() {
        let e = err(&[b64("Logo", PNG_1X1_B64), b64("<cid:LOGO>", PNG_1X1_B64)], true);
        assert!(e.contains("duplicate"), "{e}");
    }

    #[test]
    fn base64_image_gets_sniffed_mime_and_default_filename() {
        let v = validate_inline_images(&[b64("<cid:chart>", PNG_1X1_B64)], true).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].content_id, "chart");
        assert_eq!(v[0].mime_type, "image/png");
        assert_eq!(v[0].filename, "chart.png");
        assert!(matches!(&v[0].source, InlineImageSource::Data(d) if d.starts_with(b"\x89PNG")));
    }

    #[test]
    fn unknown_data_falls_back_to_octet_stream() {
        let v = validate_inline_images(&[b64("blob", "aGVsbG8=")], true).unwrap();
        assert_eq!(v[0].mime_type, "application/octet-stream");
        assert_eq!(v[0].filename, "blob");
    }

    #[test]
    fn explicit_mime_and_filename_win_and_filename_is_sanitized() {
        let img = InlineImage {
            content_id: "x".into(),
            data_base64: Some(PNG_1X1_B64.into()),
            filename: Some("my:pic.gif".into()),
            mime_type: Some("image/custom".into()),
            ..Default::default()
        };
        let v = validate_inline_images(&[img], true).unwrap();
        assert_eq!(v[0].mime_type, "image/custom");
        assert_eq!(v[0].filename, "my_pic.gif");
        // Filename extension beats sniffing when no mime_type is given.
        let img = InlineImage { content_id: "y".into(), data_base64: Some(PNG_1X1_B64.into()), filename: Some("pic.gif".into()), ..Default::default() };
        assert_eq!(validate_inline_images(&[img], true).unwrap()[0].mime_type, "image/gif");
        // Unsafe characters in a cid-derived filename are sanitized too.
        let v = validate_inline_images(&[b64("a/b", PNG_1X1_B64)], true).unwrap();
        assert_eq!(v[0].content_id, "a/b");
        assert_eq!(v[0].filename, "a_b.png");
    }

    #[test]
    fn path_image_uses_file_name_and_extension_mime() {
        let path = temp_file("photo.JPG", b"\xff\xd8\xff");
        let img = InlineImage { content_id: "p".into(), path: Some(path.to_string_lossy().into_owned()), ..Default::default() };
        let v = validate_inline_images(&[img], true).unwrap();
        assert_eq!(v[0].filename, "photo.JPG");
        assert_eq!(v[0].mime_type, "image/jpeg");
        assert_eq!(v[0].source, InlineImageSource::Path(path.to_string_lossy().into_owned()));
    }

    #[test]
    fn empty_list_is_ok() {
        assert!(validate_inline_images(&[], true).unwrap().is_empty());
    }
}
