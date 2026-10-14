# Changelog

Notable changes to outlook-mcp-rs. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [semantic versioning](https://semver.org/).

**Every pull request adds its own entry under [Unreleased]**, in the matching group (Added / Changed / Deprecated / Removed / Fixed), ending with a link to the PR. A release moves those entries under a new version heading.

## [Unreleased]

### Added

- `list_emails`: `offset` paging; the `count` cap is raised to 200 ([#14](https://github.com/adamkopelman/outlook-mcp-rs/pull/14)).
- `delete_email` with `permanent: true` hard-deletes an email, and the new `empty_deleted_items` tool empties Deleted Items (`confirm: true` required) ([#16](https://github.com/adamkopelman/outlook-mcp-rs/pull/16)).
- `list_emails`: a `to` filter matching any To/CC recipient by name or address ([#17](https://github.com/adamkopelman/outlook-mcp-rs/pull/17)).
- Attachment metadata (Content-ID, MIME type, attachment type, hidden flag) from `list_attachments` and `save_attachments` ([#18](https://github.com/adamkopelman/outlook-mcp-rs/pull/18)).
- `update_draft` tool: edit an unsent draft's subject, body, HTML body, To/CC/BCC and attachments ([#19](https://github.com/adamkopelman/outlook-mcp-rs/pull/19)).
- `get_email`: `body_truncated` / `html_truncated` flags and a `max_body_chars` limit ([#20](https://github.com/adamkopelman/outlook-mcp-rs/pull/20)).
- Hebrew round-trip tests over the full COM → JSON → stdio path ([#21](https://github.com/adamkopelman/outlook-mcp-rs/pull/21)).
- Inline Content-ID images on `send_email` and `create_draft` (`inline_images`) ([#22](https://github.com/adamkopelman/outlook-mcp-rs/pull/22)).
- `is_inline` on attachments: inline `cid:` content vs. a regular attachment ([#23](https://github.com/adamkopelman/outlook-mcp-rs/pull/23)).
- `get_inline_image` tool: an attachment by Content-ID as a base64 `data:` URI ([#24](https://github.com/adamkopelman/outlook-mcp-rs/pull/24)), with optional `context_lines` (the text before the image) ([#25](https://github.com/adamkopelman/outlook-mcp-rs/pull/25)).
- A system test for PRs #14–#25, and guidance for testing a batch of PRs ([#26](https://github.com/adamkopelman/outlook-mcp-rs/pull/26)).
- User-facing skills (`using-outlook-mcp`, `troubleshooting-outlook-mcp`), and README / CONTRIBUTING brought up to date ([#27](https://github.com/adamkopelman/outlook-mcp-rs/pull/27)).
- This changelog; every PR adds its entry under Unreleased ([#56](https://github.com/adamkopelman/outlook-mcp-rs/pull/56)).
- Content-writing tools share one input convention: exactly one of `body` / `html_body` / `body_file` / `html_body_file` on the mail tools, `body_file` on event, task and note tools; `categories` / `importance` on `send_email`, `create_draft`, `reply_email` and `update_draft`; `inline_images` on `reply_email` and `update_draft`; `update_draft` can `send`. ([#37](https://github.com/adamkopelman/outlook-mcp-rs/pull/37))
- `data:` image URIs in an HTML body become inline attachments on every mail-writing tool ([#37](https://github.com/adamkopelman/outlook-mcp-rs/pull/37)).
- Read tools share one set of output controls: batch ids on `get_email`, `get_event`, `get_note`, `get_task`, `list_attachments` and `get_inline_image` (per-item errors); `include`, `body_format`, `max_body_chars` with `*_truncated` / `*_length`, and `output_dir` on every body-reading tool ([#38](https://github.com/adamkopelman/outlook-mcp-rs/pull/38)).
- `get_task` tool; `get_email` `resolve_inline_images` (`cid:` → `data:`); `save_attachments` `inline` filter ([#38](https://github.com/adamkopelman/outlook-mcp-rs/pull/38)).
- One date grammar for every date parameter (ISO, keywords such as `start_of_week`, offsets such as `-14d`), and `<field>_after` / `<field>_before` range filters on every list tool ([#39](https://github.com/adamkopelman/outlook-mcp-rs/pull/39)).
- Shared `list_*` conventions: one `query` syntax (phrases, wildcards, `field:` scopes), list-valued filters, enum filters that mirror output fields (`item_type`, `importance`, `flag`), and `count` / `offset` paging on every list tool ([#39](https://github.com/adamkopelman/outlook-mcp-rs/pull/39)).
- A Hebrew round-trip test of every text field of every tool over the wire, a live mojibake diagnostic, and README troubleshooting for garbled text ([#40](https://github.com/adamkopelman/outlook-mcp-rs/pull/40)).

### Changed

- HTTP mode answers a request body over 64 MiB with `413` and a compressed body with `415`, instead of a generic error ([#37](https://github.com/adamkopelman/outlook-mcp-rs/pull/37)).
- A multi-word unquoted `query` matches all words in any order; a bare date in `*_before` includes that whole day; tasks and notes page at 500 ([#39](https://github.com/adamkopelman/outlook-mcp-rs/pull/39)).

### Deprecated

- `send_email` / `create_draft` `html: bool` (use `html_body`), and `update_draft`'s `draft_id` (use `email_id`) ([#37](https://github.com/adamkopelman/outlook-mcp-rs/pull/37)).
- `get_email` `prefer_html` (use `body_format: "html"`) ([#38](https://github.com/adamkopelman/outlook-mcp-rs/pull/38)).
- `list_emails` `since_days`, `flagged`, `high_importance`, and `list_events` `start_date` / `end_date` ([#39](https://github.com/adamkopelman/outlook-mcp-rs/pull/39)).

### Fixed

- `list_emails` search with Hebrew or other non-ASCII text returned nothing; it now falls back to a client-side match ([#15](https://github.com/adamkopelman/outlook-mcp-rs/pull/15)).
- HTTP mode declares `charset=utf-8` on its responses, so clients no longer decode Hebrew as Latin-1 ([#40](https://github.com/adamkopelman/outlook-mcp-rs/pull/40)).

## [0.3.0] - 2026-07-18

### Added

- Optional network mode: Streamable HTTP transport with bearer-token auth (`--http`), alongside the default stdio mode.
- CONTRIBUTING.md, issue and pull request templates, and a standalone README.

## [0.2.0] - 2026-07-17

### Added

- `check_availability`: free/busy for several people and the windows when everyone is free.
- Task tools: `list_tasks` filters (category, importance, query), `update_task` (replaces `complete_task`, now `mark_complete`), `delete_task`, and more `create_task` fields.
- Note tools: `list_notes` filters, `update_note`, `delete_note`, `modified` on `get_note`, and categories/color on `create_note`.
- `list_tasks` `query` matches the task body, not only the subject.

## [0.1.1] - 2026-07-07

### Fixed

- Detail tools tolerate items that lack optional properties.

## [0.1.0] - 2026-07-07

### Added

- First release: email, calendar, attachment, task and note tools over stdio, driving classic Outlook through COM, with a fake client for tests, live-Outlook system tests, and CI release builds.

[Unreleased]: https://github.com/adamkopelman/outlook-mcp-rs/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/adamkopelman/outlook-mcp-rs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/adamkopelman/outlook-mcp-rs/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/adamkopelman/outlook-mcp-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/adamkopelman/outlook-mcp-rs/releases/tag/v0.1.0
