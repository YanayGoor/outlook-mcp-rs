use std::sync::Arc;

use outlook_mcp_rs::outlook::fake::{FakeOutlookClient, EMAIL_ID};
use outlook_mcp_rs::server::{
    CheckAvailabilityParams, CreateDraftParams, CreateEventParams, CreateNoteParams, CreateTaskParams,
    DeleteEmailParams, DeleteEventParams, DeleteNoteParams, DeleteTaskParams, GetEmailParams,
    GetEventParams, GetInlineImageParams, GetNoteParams, ListAttachmentsParams,
    EmptyDeletedItemsParams,
    ListEmailsParams, ListEventsParams, ListNotesParams, ListTasksParams, OutlookMcpServer,
    RecurrenceParams, ReplyEmailParams, RespondToMeetingParams, SaveAttachmentsParams,
    SendEmailParams, UpdateDraftParams, UpdateEmailParams, UpdateEventParams, UpdateNoteParams, UpdateTaskParams,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use serde_json::{json, Value};

fn result_json(result: &CallToolResult) -> Value {
    let text = result.content[0]
        .as_text()
        .expect("expected text content")
        .text
        .clone();
    serde_json::from_str(&text).unwrap()
}

#[tokio::test]
async fn list_folders_records_call() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server.list_folders().await.unwrap();
    assert_eq!(fake.calls(), vec![("list_folders".to_string(), json!({}))]);
}

#[tokio::test]
async fn list_emails_passes_arguments() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: ListEmailsParams = serde_json::from_value(json!({
        "folder": "sent", "count": 5, "offset": 15, "unread_only": true
    }))
    .unwrap();
    server.list_emails(Parameters(params)).await.unwrap();
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "list_emails");
    assert_eq!(args["folder"], "sent");
    assert_eq!(args["count"], 5);
    assert_eq!(args["offset"], 15);
    assert_eq!(args["unread_only"], true);
}

#[tokio::test]
async fn list_emails_uses_defaults() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: ListEmailsParams = serde_json::from_value(json!({})).unwrap();
    server.list_emails(Parameters(params)).await.unwrap();
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "list_emails");
    assert_eq!(args["folder"], "inbox");
    assert_eq!(args["count"], 10);
    assert_eq!(args["offset"], 0);
    assert_eq!(args["unread_only"], false);
    assert!(args["to"].is_null());
}

#[tokio::test]
async fn list_emails_forwards_recipient_filter() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: ListEmailsParams = serde_json::from_value(json!({
        "folder": "sent", "to": "ada@x.com", "from": "me@x.com"
    }))
    .unwrap();
    server.list_emails(Parameters(params)).await.unwrap();
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "list_emails");
    assert_eq!(args["folder"], "sent");
    assert_eq!(args["to"], "ada@x.com");
    assert_eq!(args["from"], "me@x.com");
}

#[tokio::test]
async fn list_emails_forwards_query_and_filters() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: ListEmailsParams = serde_json::from_value(json!({
        "query": "invoice", "from": "ada@x.com", "category": "Work",
        "since_days": 30, "has_attachments": true, "flagged": true, "high_importance": true
    }))
    .unwrap();
    server.list_emails(Parameters(params)).await.unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["query"], "invoice");
    assert_eq!(args["from"], "ada@x.com");
    assert_eq!(args["category"], "Work");
    assert_eq!(args["since_days"], 30);
    assert_eq!(args["has_attachments"], true);
    assert_eq!(args["flagged"], true);
    assert_eq!(args["high_importance"], true);
}

#[tokio::test]
async fn list_emails_forwards_hebrew_query_unchanged() {
    // Issue #2: the non-ASCII query must reach the client intact (the
    // DASL/fallback handling lives in the COM client).
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: ListEmailsParams = serde_json::from_value(json!({
        "query": "מייל שיקוף Q3"
    }))
    .unwrap();
    server.list_emails(Parameters(params)).await.unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["query"], "מייל שיקוף Q3");
}

#[tokio::test]
async fn list_emails_returns_categories() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: ListEmailsParams = serde_json::from_value(json!({})).unwrap();
    let result = server.list_emails(Parameters(params)).await.unwrap();
    let json = result_json(&result);
    assert_eq!(json[0]["categories"], serde_json::json!(["Work"]));
}

#[tokio::test]
async fn get_email_returns_body() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .get_email(Parameters(GetEmailParams {
            email_id: EMAIL_ID.to_string(),
            prefer_html: false,
            max_body_chars: None,
        }))
        .await
        .unwrap();
    assert_eq!(result_json(&result)["body"], "Hi there");
}

#[tokio::test]
async fn get_email_includes_item_type() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .get_email(Parameters(GetEmailParams {
            email_id: EMAIL_ID.to_string(),
            prefer_html: false,
            max_body_chars: None,
        }))
        .await
        .unwrap();
    assert_eq!(result_json(&result)["item_type"], "email");
    assert_eq!(result_json(&result)["is_meeting"], false);
}

// ---- Non-ASCII (Hebrew) round trip, issue #3 ----------------------------

const HE_SUBJECT: &str = "מייל שיקוף";
const HE_SENDER: &str = "עדה לאבלייס";
const HE_BODY: &str = "סיכום עשייה\nשורה שנייה";

/// The raw text of a tool result: the exact JSON string rmcp puts on the wire.
fn result_text(result: &CallToolResult) -> String {
    result.content[0].as_text().expect("expected text content").text.clone()
}

/// Hebrew must appear as literal UTF-8 in the JSON, never as `\u05xx` escapes.
fn assert_raw_utf8(text: &str, expected: &[&str]) {
    for s in expected {
        assert!(text.contains(s), "{s:?} not found verbatim in {text}");
    }
    assert!(!text.contains("\\u05"), "unexpected \\u escape in {text}");
}

#[tokio::test]
async fn list_emails_returns_hebrew_text_verbatim() {
    let fake = Arc::new(FakeOutlookClient::new());
    fake.set_email_text(HE_SUBJECT, HE_SENDER, HE_BODY);
    let server = OutlookMcpServer::new(fake.clone());
    let params: ListEmailsParams = serde_json::from_value(json!({})).unwrap();
    let result = server.list_emails(Parameters(params)).await.unwrap();
    let text = result_text(&result);
    assert_raw_utf8(&text, &[HE_SUBJECT, HE_SENDER]);
    let json = result_json(&result);
    assert_eq!(json[0]["subject"].as_str().unwrap().as_bytes(), HE_SUBJECT.as_bytes());
    assert_eq!(json[0]["sender"].as_str().unwrap().as_bytes(), HE_SENDER.as_bytes());
}

#[tokio::test]
async fn get_email_returns_hebrew_text_verbatim() {
    let fake = Arc::new(FakeOutlookClient::new());
    fake.set_email_text(HE_SUBJECT, HE_SENDER, HE_BODY);
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .get_email(Parameters(GetEmailParams {
            email_id: EMAIL_ID.to_string(),
            prefer_html: false,
            max_body_chars: None,
        }))
        .await
        .unwrap();
    // The newline in the body is escaped as `\n` in JSON; check the line.
    assert_raw_utf8(&result_text(&result), &[HE_SUBJECT, HE_SENDER, "סיכום עשייה"]);
    let json = result_json(&result);
    assert_eq!(json["subject"].as_str().unwrap().as_bytes(), HE_SUBJECT.as_bytes());
    assert_eq!(json["sender"].as_str().unwrap().as_bytes(), HE_SENDER.as_bytes());
    assert_eq!(json["body"].as_str().unwrap().as_bytes(), HE_BODY.as_bytes());
}

#[tokio::test]
async fn hebrew_arguments_reach_the_client_unchanged() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: ListEmailsParams = serde_json::from_value(json!({
        "query": HE_SUBJECT, "from": HE_SENDER, "category": "סיכום עשייה"
    }))
    .unwrap();
    server.list_emails(Parameters(params)).await.unwrap();
    server
        .create_draft(Parameters(CreateDraftParams {
            to: vec!["a@example.com".to_string()],
            subject: HE_SUBJECT.to_string(),
            body: HE_BODY.to_string(),
            cc: None,
            bcc: None,
            html: false,
            attachments: None,
            inline_images: None,
        }))
        .await
        .unwrap();
    let calls = fake.calls();
    assert_eq!(calls[0].1["query"], HE_SUBJECT);
    assert_eq!(calls[0].1["from"], HE_SENDER);
    assert_eq!(calls[0].1["category"], "סיכום עשייה");
    assert_eq!(calls[1].1["subject"], HE_SUBJECT);
    assert_eq!(calls[1].1["body"], HE_BODY);
}

/// End to end over the same newline-delimited JSON-RPC codec that rmcp's
/// stdio transport uses (an in-memory duplex pipe stands in for
/// stdin/stdout): Hebrew in the request reaches the client, and Hebrew in the
/// result leaves the server as raw UTF-8 bytes.
#[tokio::test]
async fn hebrew_round_trips_as_raw_utf8_over_the_stdio_codec() {
    use rmcp::ServiceExt;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let fake = Arc::new(FakeOutlookClient::new());
    fake.set_email_text(HE_SUBJECT, HE_SENDER, HE_BODY);
    let server = OutlookMcpServer::new(fake.clone());
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    let (read_half, mut write_half) = tokio::io::split(client_io);
    let mut reader = BufReader::new(read_half);

    // Reads raw bytes up to the next response carrying `id`.
    async fn read_response<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R, id: u64) -> Vec<u8> {
        loop {
            let mut line = Vec::new();
            assert!(reader.read_until(b'\n', &mut line).await.unwrap() > 0, "server closed");
            let msg: Value = serde_json::from_slice(&line).unwrap();
            if msg["id"] == id {
                return line;
            }
        }
    }

    let requests = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "utf8-test", "version": "0"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
            "name": "list_emails", "arguments": {"query": HE_SUBJECT}}}),
    ];
    for (i, req) in requests.iter().enumerate() {
        // serde_json writes the Hebrew as raw UTF-8, like a real client would.
        let mut bytes = serde_json::to_vec(req).unwrap();
        bytes.push(b'\n');
        write_half.write_all(&bytes).await.unwrap();
        if i == 0 {
            read_response(&mut reader, 1).await;
        }
    }
    let line = read_response(&mut reader, 2).await;

    // Strict UTF-8 decode of the wire bytes; the tool result is JSON text
    // nested in a JSON string, so its Hebrew is still raw, not `\u` escaped.
    let wire = String::from_utf8(line).expect("server output must be valid UTF-8");
    assert_raw_utf8(&wire, &[HE_SUBJECT, HE_SENDER]);
    let msg: Value = serde_json::from_str(&wire).unwrap();
    let emails: Value =
        serde_json::from_str(msg["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(emails[0]["subject"], HE_SUBJECT);
    assert_eq!(emails[0]["sender"], HE_SENDER);
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "list_emails");
    assert_eq!(args["query"], HE_SUBJECT);
}

// ---- Every text field of every tool, issue #31 ----------------------------

/// A JSON-RPC client over the same newline-delimited codec rmcp's stdio
/// transport uses (an in-memory duplex pipe stands in for stdin/stdout), so
/// assertions run against the exact bytes a real client would read.
struct WireClient {
    reader: tokio::io::BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    writer: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    next_id: u64,
}

impl WireClient {
    async fn start(server: OutlookMcpServer) -> Self {
        use rmcp::ServiceExt;
        let (client_io, server_io) = tokio::io::duplex(1024 * 1024);
        tokio::spawn(async move {
            if let Ok(running) = server.serve(server_io).await {
                let _ = running.waiting().await;
            }
        });
        let (read_half, writer) = tokio::io::split(client_io);
        let mut client =
            WireClient { reader: tokio::io::BufReader::new(read_half), writer, next_id: 1 };
        client
            .request("initialize", json!({
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": "utf8-test", "version": "0"}}))
            .await;
        client.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).await;
        client
    }

    async fn send(&mut self, msg: &Value) {
        use tokio::io::AsyncWriteExt;
        // serde_json writes non-ASCII as raw UTF-8, like a real client would.
        let mut bytes = serde_json::to_vec(msg).unwrap();
        bytes.push(b'\n');
        self.writer.write_all(&bytes).await.unwrap();
    }

    /// Sends a request and returns its response as strictly decoded UTF-8
    /// wire text plus the parsed message.
    async fn request(&mut self, method: &str, params: Value) -> (String, Value) {
        use tokio::io::AsyncBufReadExt;
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})).await;
        loop {
            let mut line = Vec::new();
            assert!(self.reader.read_until(b'\n', &mut line).await.unwrap() > 0, "server closed");
            let wire = String::from_utf8(line).expect("server output must be valid UTF-8");
            let msg: Value = serde_json::from_str(&wire).unwrap();
            if msg["id"] == id {
                return (wire, msg);
            }
        }
    }
}

/// Hebrew passed as every free-text argument.
const HE_ARG: &str = "שלום עולם";

/// One tool call: the Hebrew arguments it takes, and the result fields
/// (JSON pointers) that must contain the given text verbatim.
struct Utf8Case {
    tool: &'static str,
    args: Value,
    out: Vec<(&'static str, &'static str)>,
}

fn utf8_cases() -> Vec<Utf8Case> {
    use outlook_mcp_rs::outlook::fake::{EVENT_ID, HE_TEXT, NOTE_ID, TASK_ID};
    let h = HE_ARG;
    let case = |tool, args, out| Utf8Case { tool, args, out };
    vec![
        case("list_folders", json!({}), vec![("/0/name", HE_TEXT), ("/0/path", HE_TEXT)]),
        case("list_emails",
            json!({"query": h, "folder": h, "from": h, "to": h, "category": h}),
            vec![("/0/subject", HE_TEXT), ("/0/sender", HE_TEXT), ("/0/sender_email", HE_TEXT),
                 ("/0/to", HE_TEXT), ("/0/categories/0", HE_TEXT)]),
        case("get_email", json!({"email_id": EMAIL_ID, "prefer_html": true}),
            vec![("/subject", HE_TEXT), ("/sender", HE_TEXT), ("/sender_email", HE_TEXT),
                 ("/to", HE_TEXT), ("/cc", HE_TEXT), ("/bcc", HE_TEXT), ("/body", HE_TEXT),
                 ("/html_body", HE_TEXT), ("/attachments/0", HE_TEXT), ("/categories/0", HE_TEXT),
                 ("/meeting/location", HE_TEXT), ("/meeting/organizer", HE_TEXT),
                 ("/meeting/required_attendees", HE_TEXT),
                 ("/meeting/optional_attendees", HE_TEXT)]),
        case("send_email",
            json!({"to": [h], "subject": h, "body": h, "cc": [h], "bcc": [h]}),
            vec![("/to", h), ("/subject", h)]),
        case("create_draft",
            json!({"to": [h], "subject": h, "body": h, "cc": [h], "bcc": [h]}),
            vec![("/subject", h)]),
        case("reply_email", json!({"email_id": EMAIL_ID, "body": h, "send": false}), vec![]),
        case("update_email",
            json!({"email_id": EMAIL_ID, "move_to": h, "add_categories": [h],
                   "remove_categories": [h]}),
            vec![]),
        case("update_draft",
            json!({"draft_id": EMAIL_ID, "subject": h, "body": h, "to": [h], "cc": [h],
                   "bcc": [h]}),
            vec![]),
        case("delete_email", json!({"email_id": EMAIL_ID}), vec![]),
        case("empty_deleted_items", json!({"confirm": true}), vec![]),
        case("list_events",
            json!({"query": h, "category": h, "attendees": [h], "calendar_of": h}),
            vec![("/0/subject", HE_TEXT), ("/0/location", HE_TEXT), ("/0/organizer", HE_TEXT),
                 ("/0/categories/0", HE_TEXT), ("/0/required_attendees", HE_TEXT),
                 ("/0/optional_attendees", HE_TEXT)]),
        case("get_event", json!({"event_id": EVENT_ID}),
            vec![("/subject", HE_TEXT), ("/location", HE_TEXT), ("/organizer", HE_TEXT),
                 ("/categories/0", HE_TEXT), ("/required_attendees", HE_TEXT),
                 ("/optional_attendees", HE_TEXT), ("/body", HE_TEXT)]),
        case("create_event",
            json!({"subject": h, "start": "2026-06-10T10:00:00", "end": "2026-06-10T11:00:00",
                   "body": h, "location": h, "required_attendees": [h],
                   "optional_attendees": [h], "categories": [h], "send": false}),
            vec![("/subject", h)]),
        case("respond_to_meeting",
            json!({"event_id": EVENT_ID, "response": "accept", "comment": h, "send": false}),
            vec![]),
        case("update_event",
            json!({"event_id": EVENT_ID, "subject": h, "location": h, "body": h,
                   "add_categories": [h], "remove_categories": [h],
                   "add_required_attendees": [h], "add_optional_attendees": [h],
                   "remove_attendees": [h], "send_update": false}),
            vec![]),
        case("delete_event", json!({"event_id": EVENT_ID, "send_cancellation": false}), vec![]),
        case("check_availability",
            json!({"people": [h], "start": "2026-06-10T09:00:00", "end": "2026-06-10T17:00:00"}),
            vec![("/people/0/person", h)]),
        case("list_attachments", json!({"email_id": EMAIL_ID}),
            vec![("/0/filename", HE_TEXT), ("/1/filename", HE_TEXT)]),
        case("save_attachments",
            json!({"email_id": EMAIL_ID, "save_dir": "C:\\תיקייה", "attachment_names": [h]}),
            // Expect only the Hebrew part: the wire escapes the backslash twice.
            vec![("/0/filename", HE_TEXT), ("/0/saved_to", "תיקייה")]),
        case("get_inline_image",
            json!({"email_id": EMAIL_ID, "content_id": "logo@example", "context_lines": 2}),
            vec![("/filename", HE_TEXT), ("/context", HE_TEXT)]),
        case("list_tasks", json!({"category": h, "query": h}),
            vec![("/0/subject", HE_TEXT), ("/0/categories/0", HE_TEXT)]),
        case("create_task", json!({"subject": h, "body": h, "categories": [h]}),
            vec![("/subject", h)]),
        case("update_task",
            json!({"task_id": TASK_ID, "subject": h, "body": h, "add_categories": [h],
                   "remove_categories": [h]}),
            vec![]),
        case("delete_task", json!({"task_id": TASK_ID}), vec![]),
        case("list_notes", json!({"category": h, "query": h}),
            vec![("/0/subject", HE_TEXT), ("/0/categories/0", HE_TEXT)]),
        case("get_note", json!({"note_id": NOTE_ID}),
            vec![("/subject", HE_TEXT), ("/body", HE_TEXT), ("/categories/0", HE_TEXT)]),
        case("create_note", json!({"body": h, "categories": [h]}), vec![]),
        case("update_note",
            json!({"note_id": NOTE_ID, "body": h, "add_categories": [h], "remove_categories": [h]}),
            vec![]),
        case("delete_note", json!({"note_id": NOTE_ID}), vec![]),
    ]
}

/// Issue #31: for every tool, Hebrew in each text argument reaches the
/// Outlook client unchanged, and Hebrew in each text field of the result
/// leaves the server as literal UTF-8 (never `\u` escaped, never re-encoded
/// to a code page) over the stdio codec.
#[tokio::test]
async fn every_tool_round_trips_hebrew_in_every_text_field_over_the_wire() {
    use outlook_mcp_rs::outlook::fake::HE_TEXT;

    let fake = Arc::new(FakeOutlookClient::new());
    fake.set_hebrew_text();
    let mut wire = WireClient::start(OutlookMcpServer::new(fake.clone())).await;

    // Every advertised tool has a case, so a new tool can't skip this check.
    let (_, listed) = wire.request("tools/list", json!({})).await;
    let mut advertised: Vec<String> = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    advertised.sort();
    let cases = utf8_cases();
    let mut covered: Vec<String> = cases.iter().map(|c| c.tool.to_string()).collect();
    covered.sort();
    assert_eq!(covered, advertised, "add a utf8_cases() entry for each new tool");

    for case in cases {
        let (raw, msg) = wire
            .request("tools/call", json!({"name": case.tool, "arguments": case.args}))
            .await;
        assert!(msg["error"].is_null(), "{}: JSON-RPC error {}", case.tool, msg["error"]);
        let result = &msg["result"];
        assert_ne!(result["isError"], true, "{}: tool error {result}", case.tool);

        // The wire carries non-ASCII as raw UTF-8: no `\uXXXX` escape for
        // Hebrew or for the emoji's surrogate pair.
        assert!(
            !raw.contains("\\u05") && !raw.to_ascii_lowercase().contains("\\ud83d"),
            "{}: unexpected \\u escape in {raw}",
            case.tool
        );
        let text = result["content"][0]["text"].as_str().expect("text content");
        let out: Value = serde_json::from_str(text).unwrap();
        for (pointer, expected) in &case.out {
            let field = out.pointer(pointer).and_then(Value::as_str).unwrap_or_else(|| {
                panic!("{}: no string at {pointer} in {out}", case.tool)
            });
            assert!(field.contains(expected), "{}{pointer}: {field:?} lacks {expected:?}", case.tool);
            assert!(raw.contains(expected), "{}{pointer}: {expected:?} not raw on the wire", case.tool);
        }
        if !case.out.is_empty() {
            assert!(raw.contains(HE_TEXT) || raw.contains(HE_ARG), "{}: no Hebrew on the wire", case.tool);
        }

        // Each Hebrew argument reached the Outlook client unchanged.
        let (name, recorded) = fake.calls().pop().expect("the tool reached the client");
        assert_eq!(name, case.tool);
        for (key, sent) in case.args.as_object().unwrap() {
            if sent.to_string().contains(HE_ARG) || sent.to_string().contains("תיקייה") {
                assert_eq!(&recorded[key], sent, "{}: argument {key} changed", case.tool);
            }
        }
    }
}

#[tokio::test]
async fn get_email_max_body_chars_defaults_to_none() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: GetEmailParams = serde_json::from_value(json!({"email_id": EMAIL_ID})).unwrap();
    assert_eq!(params.max_body_chars, None);
    server.get_email(Parameters(params)).await.unwrap();
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "get_email");
    assert_eq!(args["max_body_chars"], Value::Null);
}

#[tokio::test]
async fn get_email_forwards_max_body_chars() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: GetEmailParams = serde_json::from_value(json!({
        "email_id": EMAIL_ID, "prefer_html": true, "max_body_chars": 2_000_000
    }))
    .unwrap();
    server.get_email(Parameters(params)).await.unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["prefer_html"], true);
    assert_eq!(args["max_body_chars"], 2_000_000);
}

#[tokio::test]
async fn get_email_reports_truncation_fields() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    // Plain: body flags present, html ones omitted.
    let params: GetEmailParams = serde_json::from_value(json!({"email_id": EMAIL_ID})).unwrap();
    let json = result_json(&server.get_email(Parameters(params)).await.unwrap());
    assert_eq!(json["body_truncated"], false);
    assert_eq!(json["body_length"], 8);
    assert!(json.get("html_truncated").is_none());
    assert!(json.get("html_length").is_none());
    // prefer_html: html flags present too.
    let params: GetEmailParams =
        serde_json::from_value(json!({"email_id": EMAIL_ID, "prefer_html": true})).unwrap();
    let json = result_json(&server.get_email(Parameters(params)).await.unwrap());
    assert_eq!(json["html_truncated"], false);
    assert_eq!(json["html_length"], 15);
}

#[tokio::test]
async fn get_event_and_get_note_report_body_truncated() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: GetEventParams = serde_json::from_value(json!({"event_id": "x"})).unwrap();
    let json = result_json(&server.get_event(Parameters(params)).await.unwrap());
    assert_eq!(json["body_truncated"], false);
    let params: GetNoteParams = serde_json::from_value(json!({"note_id": "x"})).unwrap();
    let json = result_json(&server.get_note(Parameters(params)).await.unwrap());
    assert_eq!(json["body_truncated"], false);
}

#[tokio::test]
async fn send_email_passes_recipients_and_html_flag() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .send_email(Parameters(SendEmailParams {
            to: vec!["a@example.com".to_string(), "b@example.com".to_string()],
            subject: "Hi".to_string(),
            body: "Hello!".to_string(),
            cc: None,
            bcc: None,
            html: false,
            attachments: None,
            inline_images: None,
        }))
        .await
        .unwrap();
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "send_email");
    assert_eq!(args["to"], json!(["a@example.com", "b@example.com"]));
    assert_eq!(args["html"], false);
}

#[tokio::test]
async fn create_draft_returns_draft_saved_status() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .create_draft(Parameters(CreateDraftParams {
            to: vec!["a@example.com".to_string()],
            subject: "Hi".to_string(),
            body: "Hello!".to_string(),
            cc: None,
            bcc: None,
            html: false,
            attachments: None,
            inline_images: None,
        }))
        .await
        .unwrap();
    assert_eq!(result_json(&result)["status"], "draft_saved");
}

#[tokio::test]
async fn reply_email_passes_reply_all_and_send_flags() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .reply_email(Parameters(ReplyEmailParams {
            email_id: EMAIL_ID.to_string(),
            body: "Thanks!".to_string(),
            reply_all: true,
            html: false,
            send: false,
            attachments: None,
        }))
        .await
        .unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["reply_all"], true);
    assert_eq!(args["send"], false);
}

#[tokio::test]
async fn send_email_forwards_attachments() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: SendEmailParams = serde_json::from_value(json!({
        "to": ["a@x.com"], "subject": "Hi", "body": "yo",
        "attachments": ["C:/tmp/a.pdf", "C:/tmp/b.png"]
    })).unwrap();
    server.send_email(Parameters(params)).await.unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["attachments"], serde_json::json!(["C:/tmp/a.pdf", "C:/tmp/b.png"]));
}

#[tokio::test]
async fn update_draft_forwards_fields_and_lists_changes_in_apply_order() {
    let path = std::env::temp_dir().join("outlook-mcp-rs-tools-update-draft.txt");
    std::fs::write(&path, b"x").unwrap();
    let path_str = path.to_string_lossy().to_string();
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: UpdateDraftParams = serde_json::from_value(json!({
        "draft_id": EMAIL_ID, "attachments": [path_str], "bcc": [],
        "to": ["a@x.com", "b@x.com"], "html_body": "<p>hi</p>", "subject": "New"
    })).unwrap();
    let result = server.update_draft(Parameters(params)).await;
    let _ = std::fs::remove_file(&path);
    let v = result_json(&result.unwrap());
    assert_eq!(v["status"], "draft_updated");
    assert_eq!(v["id"], EMAIL_ID);
    assert_eq!(v["changed"], json!(["subject", "html_body", "to", "bcc", "attachments"]));
    let (name, args) = fake.calls().pop().unwrap();
    assert_eq!(name, "update_draft");
    assert_eq!(args["draft_id"], EMAIL_ID);
    assert_eq!(args["subject"], "New");
    assert_eq!(args["html_body"], "<p>hi</p>");
    assert_eq!(args["body"], Value::Null);
    assert_eq!(args["to"], json!(["a@x.com", "b@x.com"]));
    assert_eq!(args["cc"], Value::Null);
    assert_eq!(args["bcc"], json!([]));
    assert_eq!(args["attachments"], json!([path_str]));
}

#[tokio::test]
async fn update_draft_rejects_body_and_html_body_together() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: UpdateDraftParams = serde_json::from_value(json!({
        "draft_id": EMAIL_ID, "body": "plain", "html_body": "<p>html</p>"
    })).unwrap();
    assert!(server.update_draft(Parameters(params)).await.is_err());
    assert!(fake.calls().is_empty());
}

#[tokio::test]
async fn update_draft_rejects_an_empty_update() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: UpdateDraftParams = serde_json::from_value(json!({"draft_id": EMAIL_ID})).unwrap();
    assert!(server.update_draft(Parameters(params)).await.is_err());
    assert!(fake.calls().is_empty());
}

#[tokio::test]
async fn send_email_forwards_inline_images() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: SendEmailParams = serde_json::from_value(json!({
        "to": ["a@x.com"], "subject": "Hi", "html": true,
        "body": "<img src=\"cid:logo\">",
        "inline_images": [
            {"content_id": "logo", "path": "C:/img/logo.png"},
            {"content_id": "chart", "data_base64": "aGk=", "filename": "chart.png", "mime_type": "image/png"}
        ]
    })).unwrap();
    server.send_email(Parameters(params)).await.unwrap();
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "send_email");
    assert_eq!(args["html"], true);
    let imgs = &args["inline_images"];
    assert_eq!(imgs[0]["content_id"], "logo");
    assert_eq!(imgs[0]["path"], "C:/img/logo.png");
    assert_eq!(imgs[0]["data_base64"], Value::Null);
    assert_eq!(imgs[1]["content_id"], "chart");
    assert_eq!(imgs[1]["data_base64"], "aGk=");
    assert_eq!(imgs[1]["filename"], "chart.png");
    assert_eq!(imgs[1]["mime_type"], "image/png");
}

#[tokio::test]
async fn create_draft_forwards_inline_images_and_defaults_to_none() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: CreateDraftParams = serde_json::from_value(json!({
        "to": ["a@x.com"], "subject": "Hi", "body": "<img src=\"cid:logo\">", "html": true,
        "inline_images": [{"content_id": "logo", "data_base64": "aGk="}]
    })).unwrap();
    let result = server.create_draft(Parameters(params)).await.unwrap();
    assert_eq!(result_json(&result)["status"], "draft_saved");
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "create_draft");
    assert_eq!(args["inline_images"][0]["content_id"], "logo");
    assert_eq!(args["inline_images"][0]["data_base64"], "aGk=");

    // Omitted -> None (recorded as null).
    let params: CreateDraftParams = serde_json::from_value(json!({
        "to": ["a@x.com"], "subject": "Hi", "body": "plain"
    })).unwrap();
    server.create_draft(Parameters(params)).await.unwrap();
    assert_eq!(fake.calls()[1].1["inline_images"], Value::Null);
}

#[test]
fn inline_images_appear_in_send_and_draft_schemas() {
    for schema in [schemars::schema_for!(SendEmailParams), schemars::schema_for!(CreateDraftParams)] {
        let v = serde_json::to_value(&schema).unwrap();
        let prop = &v["properties"]["inline_images"];
        assert!(prop.is_object(), "inline_images missing from schema: {v}");
        assert!(prop["description"].as_str().unwrap_or("").contains("cid:CONTENT_ID"), "{prop}");
        let def = &v["$defs"]["InlineImage"];
        for field in ["content_id", "path", "data_base64", "filename", "mime_type"] {
            assert!(def["properties"][field]["description"].is_string(), "InlineImage.{field} lacks a description: {def}");
        }
        assert_eq!(def["required"], json!(["content_id"]));
    }
}

#[tokio::test]
async fn update_email_move_returns_new_id() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .update_email(Parameters(UpdateEmailParams {
            email_id: EMAIL_ID.to_string(),
            move_to: Some("Archive".to_string()),
            mark_read: None, flag: None, add_categories: None,
            remove_categories: None, importance: None,
        }))
        .await
        .unwrap();
    let v = result_json(&result);
    assert_eq!(v["id"], "new-entry|store-1");
    assert_eq!(v["status"], "updated");
    assert_eq!(v["changed"], serde_json::json!(["move_to"]));
}

#[tokio::test]
async fn update_email_state_only_keeps_same_id_and_lists_changes() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .update_email(Parameters(UpdateEmailParams {
            email_id: EMAIL_ID.to_string(),
            move_to: None,
            mark_read: Some(true),
            flag: Some("follow_up".to_string()),
            add_categories: Some(vec!["Work".to_string()]),
            remove_categories: None,
            importance: Some("high".to_string()),
        }))
        .await
        .unwrap();
    let v = result_json(&result);
    // No move → id unchanged.
    assert_eq!(v["id"], EMAIL_ID);
    assert_eq!(v["changed"], serde_json::json!(["mark_read", "flag", "add_categories", "importance"]));
    // The client saw the full update.
    let (name, args) = fake.calls().pop().unwrap();
    assert_eq!(name, "update_email");
    assert_eq!(args["flag"], "follow_up");
    assert_eq!(args["importance"], "high");
}

#[tokio::test]
async fn delete_email_defaults_to_soft_delete() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    // permanent omitted → serde default false.
    let params: DeleteEmailParams =
        serde_json::from_value(json!({"email_id": EMAIL_ID})).unwrap();
    let result = server.delete_email(Parameters(params)).await.unwrap();
    assert_eq!(result_json(&result)["permanent"], false);
    assert_eq!(
        fake.calls(),
        vec![("delete_email".to_string(), json!({"email_id": EMAIL_ID, "permanent": false}))]
    );
}

#[tokio::test]
async fn delete_email_forwards_permanent() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: DeleteEmailParams =
        serde_json::from_value(json!({"email_id": EMAIL_ID, "permanent": true})).unwrap();
    let result = server.delete_email(Parameters(params)).await.unwrap();
    assert_eq!(result_json(&result)["permanent"], true);
    assert_eq!(
        fake.calls(),
        vec![("delete_email".to_string(), json!({"email_id": EMAIL_ID, "permanent": true}))]
    );
}

#[tokio::test]
async fn empty_deleted_items_refuses_without_confirm() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    // confirm omitted → serde default false → the refusal surfaces as an error.
    let params: EmptyDeletedItemsParams = serde_json::from_value(json!({})).unwrap();
    let err = server.empty_deleted_items(Parameters(params)).await.unwrap_err();
    assert!(err.message.contains("confirm=true"));
    assert_eq!(
        fake.calls(),
        vec![("empty_deleted_items".to_string(), json!({"confirm": false}))]
    );
}

#[tokio::test]
async fn empty_deleted_items_with_confirm_returns_counts() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: EmptyDeletedItemsParams =
        serde_json::from_value(json!({"confirm": true})).unwrap();
    let result = server.empty_deleted_items(Parameters(params)).await.unwrap();
    let v = result_json(&result);
    assert_eq!(v["status"], "emptied");
    assert!(v["items_deleted"].is_number());
    assert!(v["folders_deleted"].is_number());
    assert!(v["failed"].is_number());
    assert_eq!(
        fake.calls(),
        vec![("empty_deleted_items".to_string(), json!({"confirm": true}))]
    );
}

#[tokio::test]
async fn client_error_propagates_as_tool_error() {
    let fake = Arc::new(FakeOutlookClient::new());
    fake.set_fail_with("Outlook exploded");
    let server = OutlookMcpServer::new(fake.clone());
    let params: ListEmailsParams = serde_json::from_value(json!({})).unwrap();
    let err = server.list_emails(Parameters(params)).await.unwrap_err();
    assert!(err.message.contains("Outlook exploded"));
}

#[tokio::test]
async fn list_events_passes_date_range() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .list_events(Parameters(ListEventsParams {
            start_date: Some("2026-06-10".to_string()),
            end_date: Some("2026-06-17".to_string()),
            query: None, category: None, show_as: None, my_response: None,
            attendees: None, attendee_role: None, meetings_only: false,
            all_day: None, calendar_of: None,
        }))
        .await
        .unwrap();
    let (name, args) = fake.calls().pop().unwrap();
    assert_eq!(name, "list_events");
    assert_eq!(args["start_date"], "2026-06-10");
    assert_eq!(args["end_date"], "2026-06-17");
}

#[tokio::test]
async fn list_events_forwards_all_filters() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .list_events(Parameters(ListEventsParams {
            start_date: None, end_date: None,
            query: Some("review".to_string()),
            category: Some("Work".to_string()),
            show_as: Some("busy".to_string()),
            my_response: Some("accepted".to_string()),
            attendees: Some(vec!["alice@example.com".to_string()]),
            attendee_role: Some("required".to_string()),
            meetings_only: true,
            all_day: Some(false),
            calendar_of: Some("bob@example.com".to_string()),
        }))
        .await
        .unwrap();
    let (name, args) = fake.calls().pop().unwrap();
    assert_eq!(name, "list_events");
    assert_eq!(args["query"], "review");
    assert_eq!(args["category"], "Work");
    assert_eq!(args["show_as"], "busy");
    assert_eq!(args["my_response"], "accepted");
    assert_eq!(args["attendees"], serde_json::json!(["alice@example.com"]));
    assert_eq!(args["attendee_role"], "required");
    assert_eq!(args["meetings_only"], true);
    assert_eq!(args["all_day"], false);
    assert_eq!(args["calendar_of"], "bob@example.com");
}

#[tokio::test]
async fn get_event_returns_subject_and_friendly_fields() {
    use outlook_mcp_rs::outlook::fake::EVENT_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .get_event(Parameters(GetEventParams { event_id: EVENT_ID.to_string() }))
        .await
        .unwrap();
    let v = result_json(&result);
    assert_eq!(v["subject"], "Standup");
    // New enriched fields surface at the top level (EventDetail flattens the summary).
    assert_eq!(v["show_as"], "busy");
    assert_eq!(v["my_response"], "accepted");
    assert_eq!(v["required_attendees"], "");
    assert_eq!(v["optional_attendees"], "");
    // The old nested "response" key is gone (renamed to my_response in the summary).
    assert!(v.get("response").is_none());
}

#[tokio::test]
async fn create_event_passes_attendees() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .create_event(Parameters(CreateEventParams {
            subject: "Sync".to_string(),
            start: "2026-06-12T14:00".to_string(),
            end: "2026-06-12T15:00".to_string(),
            body: None,
            location: None,
            attendees: Some(vec!["a@example.com".to_string()]),
            required_attendees: None,
            optional_attendees: None,
            all_day: false,
            reminder_minutes: None,
            categories: None,
            show_as: None,
            send: true,
            recurrence: None,
        }))
        .await
        .unwrap();
    let (_, args) = &fake.calls()[0];
    // The legacy `attendees` alias merges into `required_attendees`.
    assert_eq!(args["required_attendees"], json!(["a@example.com"]));
}

#[tokio::test]
async fn create_event_status_reflects_attendees_and_send() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());

    let base = |required: Option<Vec<String>>, send: bool| CreateEventParams {
        subject: "Sync".to_string(),
        start: "2026-06-12T14:00".to_string(),
        end: "2026-06-12T15:00".to_string(),
        body: None, location: None, attendees: None,
        required_attendees: required, optional_attendees: None,
        all_day: false, reminder_minutes: None, categories: None, show_as: None,
        send,
        recurrence: None,
    };

    let r = server.create_event(Parameters(base(Some(vec!["a@example.com".to_string()]), true)))
        .await.unwrap();
    assert_eq!(result_json(&r)["status"], "meeting_sent");

    let r = server.create_event(Parameters(base(Some(vec!["a@example.com".to_string()]), false)))
        .await.unwrap();
    assert_eq!(result_json(&r)["status"], "meeting_saved");

    let r = server.create_event(Parameters(base(None, true))).await.unwrap();
    assert_eq!(result_json(&r)["status"], "saved");
}

#[tokio::test]
async fn create_event_forwards_recurrence() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .create_event(Parameters(CreateEventParams {
            subject: "Standup".to_string(),
            start: "2026-06-12T09:00".to_string(),
            end: "2026-06-12T09:15".to_string(),
            body: None, location: None, attendees: None,
            required_attendees: None, optional_attendees: None,
            all_day: false, reminder_minutes: None, categories: None, show_as: None,
            send: true,
            recurrence: Some(RecurrenceParams {
                pattern: "weekly".to_string(),
                interval: Some(1),
                days_of_week: Some(vec!["monday".to_string(), "wednesday".to_string()]),
                day_of_month: None,
                until: None,
                occurrences: Some(10),
            }),
        }))
        .await
        .unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["recurrence"]["pattern"], "weekly");
    assert_eq!(args["recurrence"]["days_of_week"], json!(["monday", "wednesday"]));
    assert_eq!(args["recurrence"]["occurrences"], 10);
}

#[tokio::test]
async fn check_availability_forwards_params() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .check_availability(Parameters(CheckAvailabilityParams {
            people: vec!["alice@example.com".to_string(), "bob@example.com".to_string()],
            start: "2099-01-01T09:00".to_string(),
            end: "2099-01-01T17:00".to_string(),
            interval_minutes: 30,
            treat_as_free: vec!["free".to_string()],
        }))
        .await
        .unwrap();
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "check_availability");
    assert_eq!(args["people"], json!(["alice@example.com", "bob@example.com"]));
    assert_eq!(args["start"], "2099-01-01T09:00");
    assert_eq!(args["end"], "2099-01-01T17:00");
    assert_eq!(args["interval_minutes"], 30);
    assert_eq!(args["treat_as_free"], json!(["free"]));
}

#[tokio::test]
async fn check_availability_returns_people_and_common_free() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .check_availability(Parameters(CheckAvailabilityParams {
            people: vec!["alice@example.com".to_string()],
            start: "2099-01-01T09:00".to_string(),
            end: "2099-01-01T09:30".to_string(),
            interval_minutes: 30,
            treat_as_free: vec!["free".to_string()],
        }))
        .await
        .unwrap();
    let v = result_json(&result);
    assert_eq!(v["people"][0]["person"], "alice@example.com");
    assert_eq!(v["people"][0]["resolved"], true);
    assert_eq!(v["common_free"][0]["start"], "2099-01-01T09:00");
}

#[tokio::test]
async fn get_event_recurrence_is_none_by_default() {
    use outlook_mcp_rs::outlook::fake::EVENT_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .get_event(Parameters(GetEventParams { event_id: EVENT_ID.to_string() }))
        .await
        .unwrap();
    let v = result_json(&result);
    assert!(v["recurrence"].is_null());
}

#[tokio::test]
async fn respond_to_meeting_defaults_send_true() {
    use outlook_mcp_rs::outlook::fake::EVENT_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: RespondToMeetingParams =
        serde_json::from_value(json!({"event_id": EVENT_ID, "response": "accept"})).unwrap();
    server.respond_to_meeting(Parameters(params)).await.unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["response"], "accept");
    assert_eq!(args["send"], true);
}

#[tokio::test]
async fn update_event_lists_changed_fields() {
    use outlook_mcp_rs::outlook::fake::EVENT_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .update_event(Parameters(UpdateEventParams {
            event_id: EVENT_ID.to_string(),
            subject: Some("Renamed sync".to_string()),
            start: None, end: None, location: None, body: None, all_day: None,
            reminder_minutes: None, show_as: Some("tentative".to_string()),
            add_categories: Some(vec!["Work".to_string()]),
            remove_categories: None,
            add_required_attendees: Some(vec!["a@example.com".to_string()]),
            add_optional_attendees: None, remove_attendees: None,
            send_update: true,
            recurrence: None, clear_recurrence: false,
        }))
        .await
        .unwrap();
    let v = result_json(&result);
    assert_eq!(v["status"], "updated");
    assert_eq!(v["id"], EVENT_ID);
    assert_eq!(
        v["changed"],
        json!(["subject", "show_as", "add_categories", "add_required_attendees"])
    );
    let (name, args) = fake.calls().pop().unwrap();
    assert_eq!(name, "update_event");
    assert_eq!(args["send_update"], true);
}

#[tokio::test]
async fn update_event_remove_attendees_is_tracked() {
    use outlook_mcp_rs::outlook::fake::EVENT_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .update_event(Parameters(UpdateEventParams {
            event_id: EVENT_ID.to_string(),
            subject: None, start: None, end: None, location: None, body: None,
            all_day: None, reminder_minutes: None, show_as: None,
            add_categories: None, remove_categories: None,
            add_required_attendees: None, add_optional_attendees: None,
            remove_attendees: Some(vec!["a@example.com".to_string()]),
            send_update: false,
            recurrence: None, clear_recurrence: false,
        }))
        .await
        .unwrap();
    let v = result_json(&result);
    assert_eq!(v["changed"], json!(["remove_attendees"]));
}

#[tokio::test]
async fn update_event_forwards_recurrence() {
    use outlook_mcp_rs::outlook::fake::EVENT_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .update_event(Parameters(UpdateEventParams {
            event_id: EVENT_ID.to_string(),
            subject: None, start: None, end: None, location: None, body: None,
            all_day: None, reminder_minutes: None, show_as: None,
            add_categories: None, remove_categories: None,
            add_required_attendees: None, add_optional_attendees: None, remove_attendees: None,
            send_update: false,
            recurrence: Some(RecurrenceParams {
                pattern: "daily".to_string(), interval: Some(2), days_of_week: None,
                day_of_month: None, until: Some("2099-06-01".to_string()), occurrences: None,
            }),
            clear_recurrence: false,
        }))
        .await
        .unwrap();
    let v = result_json(&result);
    assert_eq!(v["changed"], json!(["recurrence"]));
    let (_, args) = fake.calls().last().unwrap().clone();
    assert_eq!(args["recurrence"]["pattern"], "daily");
    assert_eq!(args["recurrence"]["until"], "2099-06-01");
}

#[tokio::test]
async fn update_event_forwards_clear_recurrence() {
    use outlook_mcp_rs::outlook::fake::EVENT_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .update_event(Parameters(UpdateEventParams {
            event_id: EVENT_ID.to_string(),
            subject: None, start: None, end: None, location: None, body: None,
            all_day: None, reminder_minutes: None, show_as: None,
            add_categories: None, remove_categories: None,
            add_required_attendees: None, add_optional_attendees: None, remove_attendees: None,
            send_update: false,
            recurrence: None,
            clear_recurrence: true,
        }))
        .await
        .unwrap();
    let v = result_json(&result);
    assert_eq!(v["changed"], json!(["clear_recurrence"]));
}

#[tokio::test]
async fn update_event_rejects_recurrence_and_clear_recurrence_together() {
    use outlook_mcp_rs::outlook::fake::EVENT_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let err = server
        .update_event(Parameters(UpdateEventParams {
            event_id: EVENT_ID.to_string(),
            subject: None, start: None, end: None, location: None, body: None,
            all_day: None, reminder_minutes: None, show_as: None,
            add_categories: None, remove_categories: None,
            add_required_attendees: None, add_optional_attendees: None, remove_attendees: None,
            send_update: false,
            recurrence: Some(RecurrenceParams {
                pattern: "daily".to_string(), interval: None, days_of_week: None,
                day_of_month: None, until: None, occurrences: None,
            }),
            clear_recurrence: true,
        }))
        .await
        .unwrap_err();
    assert!(err.message.contains("cannot set recurrence and clear_recurrence"));
}

#[tokio::test]
async fn delete_event_returns_deleted_status() {
    use outlook_mcp_rs::outlook::fake::EVENT_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .delete_event(Parameters(DeleteEventParams {
            event_id: EVENT_ID.to_string(),
            send_cancellation: true,
        }))
        .await
        .unwrap();
    assert_eq!(result_json(&result)["status"], "deleted");
    let (name, args) = fake.calls().pop().unwrap();
    assert_eq!(name, "delete_event");
    assert_eq!(args["send_cancellation"], true);
}

#[tokio::test]
async fn list_attachments_returns_filename() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .list_attachments(Parameters(ListAttachmentsParams { email_id: EMAIL_ID.to_string() }))
        .await
        .unwrap();
    let v = result_json(&result);
    assert_eq!(v[0]["filename"], "report.pdf");
    assert_eq!(v[0]["index"], 1);
    assert_eq!(v[0]["size"], 1234);
    assert_eq!(v[0]["type"], "file");
    assert!(v[0]["content_id"].is_null());
    assert_eq!(v[0]["mime_type"], "application/pdf");
    assert_eq!(v[0]["hidden"], false);
    assert_eq!(v[0]["is_inline"], false);
    assert_eq!(v[1]["content_id"], "logo@example");
    assert_eq!(v[1]["mime_type"], "image/png");
    assert_eq!(v[1]["hidden"], true);
    assert_eq!(v[1]["is_inline"], true);
}

#[tokio::test]
async fn save_attachments_passes_dir_and_names() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .save_attachments(Parameters(SaveAttachmentsParams {
            email_id: EMAIL_ID.to_string(),
            save_dir: "/tmp/x".to_string(),
            attachment_names: Some(vec!["report.pdf".to_string()]),
        }))
        .await
        .unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["save_dir"], "/tmp/x");
    assert_eq!(args["attachment_names"], json!(["report.pdf"]));
    let v = result_json(&result);
    assert_eq!(v[0]["filename"], "report.pdf");
    assert_eq!(v[0]["status"], "saved");
    assert_eq!(v[0]["saved_to"], "/tmp/x");
    for key in ["index", "size", "type", "content_id", "mime_type", "hidden", "is_inline"] {
        assert!(v[0].get(key).is_some(), "missing {key}");
    }
}

#[tokio::test]
async fn get_inline_image_forwards_args_and_returns_data_uri() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .get_inline_image(Parameters(GetInlineImageParams {
            email_id: EMAIL_ID.to_string(),
            content_id: "cid:logo@example".to_string(),
            context_lines: None,
        }))
        .await
        .unwrap();
    assert_eq!(
        fake.calls(),
        vec![(
            "get_inline_image".to_string(),
            json!({"email_id": EMAIL_ID, "content_id": "cid:logo@example", "context_lines": null})
        )]
    );
    let v = result_json(&result);
    assert_eq!(v["content_id"], "logo@example");
    assert_eq!(v["filename"], "logo.png");
    assert_eq!(v["mime_type"], "image/png");
    assert_eq!(v["size"], 4);
    assert_eq!(v["data_uri"], "data:image/png;base64,iVBORw==");
    // Not requested: no `context` key at all.
    assert!(v.get("context").is_none(), "{v}");
}

#[tokio::test]
async fn get_inline_image_forwards_context_lines_and_returns_context() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .get_inline_image(Parameters(GetInlineImageParams {
            email_id: EMAIL_ID.to_string(),
            content_id: "logo@example".to_string(),
            context_lines: Some(3),
        }))
        .await
        .unwrap();
    assert_eq!(
        fake.calls(),
        vec![(
            "get_inline_image".to_string(),
            json!({"email_id": EMAIL_ID, "content_id": "logo@example", "context_lines": 3})
        )]
    );
    let v = result_json(&result);
    assert_eq!(v["context"], "Here is our new logo:");
    assert_eq!(v["data_uri"], "data:image/png;base64,iVBORw==");
}

#[test]
fn get_inline_image_params_context_lines_defaults_to_none() {
    let p: GetInlineImageParams =
        serde_json::from_value(json!({"email_id": EMAIL_ID, "content_id": "a@b"})).unwrap();
    assert_eq!(p.context_lines, None);
    let p: GetInlineImageParams =
        serde_json::from_value(json!({"email_id": EMAIL_ID, "content_id": "a@b", "context_lines": 5})).unwrap();
    assert_eq!(p.context_lines, Some(5));
    let negative = serde_json::from_value::<GetInlineImageParams>(
        json!({"email_id": EMAIL_ID, "content_id": "a@b", "context_lines": -1}));
    assert!(negative.is_err());
}

#[test]
fn get_inline_image_params_require_content_id() {
    let missing = serde_json::from_value::<GetInlineImageParams>(json!({"email_id": EMAIL_ID}));
    assert!(missing.is_err());
    let missing = serde_json::from_value::<GetInlineImageParams>(json!({"content_id": "a@b"}));
    assert!(missing.is_err());
}

#[tokio::test]
async fn get_inline_image_surfaces_client_errors() {
    let fake = Arc::new(FakeOutlookClient::new());
    fake.set_fail_with("Content-ID 'x@y' not found.");
    let server = OutlookMcpServer::new(fake.clone());
    let err = server
        .get_inline_image(Parameters(GetInlineImageParams {
            email_id: EMAIL_ID.to_string(),
            content_id: "x@y".to_string(),
            context_lines: None,
        }))
        .await
        .unwrap_err();
    assert!(err.message.contains("Content-ID 'x@y' not found."));
}

// ---- Tasks ----

#[tokio::test]
async fn list_tasks_passes_include_completed() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .list_tasks(Parameters(ListTasksParams {
            include_completed: true,
            category: None,
            importance: None,
            query: None,
        }))
        .await
        .unwrap();
    assert_eq!(fake.calls(), vec![
        ("list_tasks".to_string(), json!({
            "include_completed": true, "category": null, "importance": null, "query": null,
        })),
    ]);
}

#[tokio::test]
async fn list_tasks_forwards_filters() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .list_tasks(Parameters(ListTasksParams {
            include_completed: true,
            category: Some("Red Category".to_string()),
            importance: Some("high".to_string()),
            query: Some("milk".to_string()),
        }))
        .await
        .unwrap();
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "list_tasks");
    assert_eq!(args["include_completed"], true);
    assert_eq!(args["category"], "Red Category");
    assert_eq!(args["importance"], "high");
    assert_eq!(args["query"], "milk");
}

#[tokio::test]
async fn list_tasks_defaults_all_filters_to_none() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: ListTasksParams = serde_json::from_value(json!({})).unwrap();
    server.list_tasks(Parameters(params)).await.unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["include_completed"], false);
    assert!(args["category"].is_null());
    assert!(args["importance"].is_null());
    assert!(args["query"].is_null());
}

#[tokio::test]
async fn create_task_passes_importance() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: CreateTaskParams = serde_json::from_value(json!({
        "subject": "Buy milk", "due_date": "2026-06-15", "importance": "high"
    })).unwrap();
    server.create_task(Parameters(params)).await.unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["importance"], "high");
}

#[tokio::test]
async fn create_task_forwards_categories_start_date_and_reminder() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: CreateTaskParams = serde_json::from_value(json!({
        "subject": "Ship it",
        "categories": ["Blue Category"],
        "start_date": "2099-01-01",
        "reminder_time": "2099-01-01T09:00"
    })).unwrap();
    server.create_task(Parameters(params)).await.unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["categories"], json!(["Blue Category"]));
    assert_eq!(args["start_date"], "2099-01-01");
    assert_eq!(args["reminder_time"], "2099-01-01T09:00");
}

#[tokio::test]
async fn update_task_marks_complete() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    use outlook_mcp_rs::outlook::fake::TASK_ID;
    server
        .update_task(Parameters(UpdateTaskParams {
            task_id: TASK_ID.to_string(), mark_complete: Some(true),
            subject: None, body: None, due_date: None, start_date: None,
            importance: None, add_categories: None, remove_categories: None,
            percent_complete: None, reminder_time: None,
        }))
        .await
        .unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["mark_complete"], true);
}

#[tokio::test]
async fn update_task_reopens_with_mark_complete_false() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    use outlook_mcp_rs::outlook::fake::TASK_ID;
    let result = server
        .update_task(Parameters(UpdateTaskParams {
            task_id: TASK_ID.to_string(), mark_complete: Some(false),
            subject: None, body: None, due_date: None, start_date: None,
            importance: None, add_categories: None, remove_categories: None,
            percent_complete: None, reminder_time: None,
        }))
        .await
        .unwrap();
    let json = result_json(&result);
    assert!(json["changed"].as_array().unwrap().iter().any(|v| v == "mark_complete"));
}

#[tokio::test]
async fn update_task_forwards_field_edits() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    use outlook_mcp_rs::outlook::fake::TASK_ID;
    server
        .update_task(Parameters(UpdateTaskParams {
            task_id: TASK_ID.to_string(), mark_complete: None,
            subject: Some("Renamed".to_string()), body: None,
            due_date: None, start_date: None, importance: Some("high".to_string()),
            add_categories: Some(vec!["Red Category".to_string()]), remove_categories: None,
            percent_complete: Some(50), reminder_time: None,
        }))
        .await
        .unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["subject"], "Renamed");
    assert_eq!(args["importance"], "high");
    assert_eq!(args["add_categories"], json!(["Red Category"]));
    assert_eq!(args["percent_complete"], 50);
}

#[tokio::test]
async fn delete_task_records_call() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    use outlook_mcp_rs::outlook::fake::TASK_ID;
    let result = server
        .delete_task(Parameters(DeleteTaskParams { task_id: TASK_ID.to_string() }))
        .await
        .unwrap();
    let json = result_json(&result);
    assert_eq!(json["status"], "deleted");
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "delete_task");
    assert_eq!(args["task_id"], TASK_ID);
}

// ---- Notes ----

#[tokio::test]
async fn list_notes_records_call() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server.list_notes(Parameters(ListNotesParams { category: None, query: None })).await.unwrap();
    assert_eq!(fake.calls(), vec![("list_notes".to_string(), json!({"category": null, "query": null}))]);
}

#[tokio::test]
async fn list_notes_forwards_filters() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .list_notes(Parameters(ListNotesParams {
            category: Some("Green Category".to_string()),
            query: Some("renew".to_string()),
        }))
        .await
        .unwrap();
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "list_notes");
    assert_eq!(args["category"], "Green Category");
    assert_eq!(args["query"], "renew");
}

#[tokio::test]
async fn list_notes_defaults_filters_to_none() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: ListNotesParams = serde_json::from_value(json!({})).unwrap();
    server.list_notes(Parameters(params)).await.unwrap();
    let (_, args) = &fake.calls()[0];
    assert!(args["category"].is_null());
    assert!(args["query"].is_null());
}

#[tokio::test]
async fn get_note_returns_body() {
    use outlook_mcp_rs::outlook::fake::NOTE_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .get_note(Parameters(GetNoteParams { note_id: NOTE_ID.to_string() }))
        .await
        .unwrap();
    assert!(result_json(&result)["body"].as_str().unwrap().starts_with("Ideas"));
}

#[tokio::test]
async fn create_note_records_body() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .create_note(Parameters(CreateNoteParams {
            body: "Ideas\n- one".to_string(), categories: None, color: None,
        }))
        .await
        .unwrap();
    assert_eq!(fake.calls(), vec![
        ("create_note".to_string(), json!({"body": "Ideas\n- one", "categories": null, "color": null})),
    ]);
}

#[tokio::test]
async fn create_note_forwards_categories_and_color() {
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let params: CreateNoteParams = serde_json::from_value(json!({
        "body": "Remember to renew the domain",
        "categories": ["Yellow Category"],
        "color": "yellow"
    })).unwrap();
    server.create_note(Parameters(params)).await.unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["categories"], json!(["Yellow Category"]));
    assert_eq!(args["color"], "yellow");
}

#[tokio::test]
async fn get_note_includes_modified() {
    use outlook_mcp_rs::outlook::fake::NOTE_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .get_note(Parameters(GetNoteParams { note_id: NOTE_ID.to_string() }))
        .await
        .unwrap();
    let v = result_json(&result);
    // The fake may return null for a note that was never "modified" —
    // assert the key exists in the JSON shape, not a specific non-null value.
    assert!(v.as_object().unwrap().contains_key("modified"));
}

#[tokio::test]
async fn update_note_forwards_body_and_color() {
    use outlook_mcp_rs::outlook::fake::NOTE_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    server
        .update_note(Parameters(UpdateNoteParams {
            note_id: NOTE_ID.to_string(),
            body: Some("Updated body".to_string()),
            add_categories: None, remove_categories: None,
            color: Some("pink".to_string()),
        }))
        .await
        .unwrap();
    let (_, args) = &fake.calls()[0];
    assert_eq!(args["body"], "Updated body");
    assert_eq!(args["color"], "pink");
}

#[tokio::test]
async fn update_note_manages_categories() {
    use outlook_mcp_rs::outlook::fake::NOTE_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .update_note(Parameters(UpdateNoteParams {
            note_id: NOTE_ID.to_string(),
            body: None,
            add_categories: Some(vec!["Blue Category".to_string()]),
            remove_categories: None,
            color: None,
        }))
        .await
        .unwrap();
    let v = result_json(&result);
    assert!(v["changed"].as_array().unwrap().iter().any(|c| c == "add_categories"));
}

#[tokio::test]
async fn delete_note_records_call() {
    use outlook_mcp_rs::outlook::fake::NOTE_ID;
    let fake = Arc::new(FakeOutlookClient::new());
    let server = OutlookMcpServer::new(fake.clone());
    let result = server
        .delete_note(Parameters(DeleteNoteParams { note_id: NOTE_ID.to_string() }))
        .await
        .unwrap();
    let json = result_json(&result);
    assert_eq!(json["status"], "deleted");
    let (name, args) = &fake.calls()[0];
    assert_eq!(name, "delete_note");
    assert_eq!(args["note_id"], NOTE_ID);
}
