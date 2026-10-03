# System test plan — open PRs #14–#25 merged together, 2026-10-03

## Purpose

Live-verify the new and changed behaviour of all 12 open PRs, **merged
together** on the integration branch `systest/all-prs-2026-10-03`, against
the real running Outlook mailbox. Per
`.claude/skills/live-outlook-system-test/SKILL.md` ("Testing a batch of PRs").
The user asked for this to run autonomously, so this plan was not
signed off in advance; judgement calls are listed in the results doc.

## Mechanism

Direct `WindowsOutlookClient` calls from one `#[ignore]`d, non-panicking test,
`tests/system_test_prs.rs`:

    cargo test --test system_test_prs -- --ignored --nocapture

The `server.rs` tool methods are thin wrappers, and the tool layer's new
params (`offset`, `to`, `permanent`, `max_body_chars`, `inline_images`,
`context_lines`, `UpdateDraftParams`, `EmptyDeletedItemsParams`) are already
covered by the fake-client tests in `tests/tools.rs`. The PRs' own `#[ignore]`d
tests in `tests/live_outlook.rs` are also run as a second pass.

## Accounts and real-world traffic

- Mailbox: `adamkopelman@outlook.com` (the only account in the profile).
- **One real send:** test S1 sends a self-loop email (to
  `adamkopelman@outlook.com`, the user's own mailbox, used for self-loop sends
  in earlier system tests) carrying one inline image, so we can check that
  inline images survive real transport and test `update_draft`'s refusal on a
  received item. Nothing goes to any other address.
- Every other email item is a **draft** addressed to `nobody@example.invalid` /
  `someone-else@example.invalid` (reserved `.invalid` TLD, never delivered).

## Tagging / cleanup

- Subject tag `[outlook-mcp-rs systest PRS]` plus a per-run token `r<unix-secs>`.
- Hebrew tokens are per-run too (Hebrew word + run digits), so old items can't
  match.
- Cleanup deletes every created id with `delete_email(permanent=true)`, then
  sweeps Drafts / Inbox / Sent Items / Deleted Items for the tag (count 200,
  repeated until a pass finds nothing), also permanently. A raw PowerShell COM
  sweep for the tag is the ground truth afterwards.
- **`empty_deleted_items(confirm=true)` is NOT run live.** Deleted Items holds
  53 of the user's own items. Only the `confirm=false` refusal is tested live.

## Seed data (created in phase 0, all drafts)

| label | subject (after tag + run) | to | notes |
|---|---|---|---|
| PG1..PG5 | `page N pg<run>` | nobody@ | for offset paging |
| HE-S | `<hebrew-subject-token> שיקוף` | nobody@ | Hebrew in subject |
| HE-B | `hebrew body <run>` | nobody@ | Hebrew token only in the body |
| TO-A | `to-a <run>` | nobody@example.invalid | `to` filter |
| TO-B | `to-b <run>` | someone-else@example.invalid | `to` filter |

## Tests (ID → expected)

### PR #14 — list_emails offset/cap
- **P14-1** `query=pg<run>`, folder drafts, `count=2` with `offset` 0/2/4 → page
  sizes 2/2/1; pages disjoint; union = PG1..PG5; concatenation equals the
  `offset=0,count=5` order.
- **P14-2** `offset` past the end (10) → empty. Negative offset (-3) → same as 0.
- **P14-3** cap: `count=500` on Deleted Items succeeds; if the folder holds >50
  items, the result has >50 (old cap was 50) and ≤200.

### PR #15 — non-ASCII search fallback
- **P15-1** Hebrew subject token → exactly {HE-S} among tagged items.
- **P15-2** Hebrew token that appears only in HE-B's body → {HE-B}.
- **P15-3** Hebrew token that appears nowhere → no tagged items.
- **P15-4** ASCII query still works (`pg<run>`, already P14) and a Hebrew query
  combined with `to=nobody` still finds HE-S (fallback + `to` together).

### PR #16 — permanent delete / empty_deleted_items
- **P16-1** new draft → `delete_email(permanent=true)` → `{status: deleted,
  permanent: true}`; `get_email(id)` errors; not in Deleted Items (subject search).
- **P16-2** new draft → `delete_email(permanent=false)` → `permanent: false`, the
  item **is** in Deleted Items; then `delete_email(<deleted-items id>,
  permanent=true)` → gone from Deleted Items.
- **P16-3** `empty_deleted_items(false)` → Err mentioning `confirm`; Deleted Items
  item count (via `list_folders`) unchanged.

### PR #17 — list_emails `to` filter
- **P17-1** `to=someone-else@example.invalid` (+ run query) → {TO-B}.
- **P17-2** `to=nobody@example.invalid` → contains TO-A, not TO-B.
- **P17-3** `to=NOBODY@EXAMPLE.INVALID` (case) → contains TO-A.
- **P17-4** `to=no-such-recipient-<run>` → no tagged items.

### PR #18 — attachment metadata
- **P18-1** draft with a `.txt` attachment → `list_attachments` entry: `type=file`,
  `mime_type` `text/plain`, `content_id` none, `hidden=false`, `size>0`.
- **P18-2** `save_attachments` to a scratch dir → entry has the same metadata keys
  plus `status=saved`, `saved_to`; file bytes equal the original.

### PR #19 — update_draft
- **P19-1** edit subject + html_body + to + cc in one call → `status
  draft_updated`, `changed` = [subject, html_body, to, cc]; `get_email` shows the
  new subject, HTML marker, To and CC.
- **P19-2** `cc: []` clears CC; `bcc` set → `get_email` cc empty, bcc set.
- **P19-3** `attachments` append → attachment count goes from 1 to 2, the
  original is still there.
- **P19-4** `body` + `html_body` together → Err; subject unchanged.
- **P19-5** received item (S1's inbox copy) → Err "only unsent drafts"; subject
  unchanged.

### PR #20 — get_email truncation
- **P20-1** draft with a ~3000-char plain body: default → `body_truncated=false`,
  `body_length` ≥ 3000; `max_body_chars=1000` → body is the first 1000 chars
  plus the existing `"\n\n[... truncated at 1000 characters]"` marker,
  `body_truncated=true`, same `body_length`.
- **P20-2** HTML draft, `prefer_html=true, max_body_chars=1000` →
  `html_truncated=true`, `html_length` > 1000, html is 1000 chars + marker.

### PR #21 — UTF-8 / Hebrew round-trip
- **P21-1** draft with Hebrew + emoji subject/body → `get_email.subject` equal
  exactly; body contains the exact Hebrew line; `list_emails` summary subject
  equal exactly.
- **P21-2** `update_draft` to a new Hebrew subject → reads back exactly.

### PR #22 — inline images on send/draft
- **P22-1** `create_draft(html, inline_images=[path PNG cid "logo", base64 PNG cid
  "chart"])` → two attachments with content_ids `logo`/`chart`, `mime_type`
  `image/png`, `hidden=true`.
- **P22-2** `inline_images` with `html=false` → Err; no draft created.
- **P22-3** duplicate content_id → Err; no draft created.
- **S1** `send_email` self-loop, html, one inline base64 PNG → `status sent`;
  received copy found in Inbox within ~90 s.
- **P22-4** received copy's attachment has the content_id and is inline.

### PR #23 — is_inline
- **P23-1** draft with html referencing `cid:logo`, plus a regular `.txt`
  attachment → `logo.is_inline=true`, `.txt.is_inline=false`.
- **P23-2** same flag through `save_attachments` entries.

### PR #24 — get_inline_image
- **P24-1** `get_inline_image(P22-1 draft, "cid:LOGO")` (prefix + other case) →
  `content_id=logo`, `mime_type=image/png`, data URI decodes to exactly the PNG
  bytes; `<chart>` form works too.
- **P24-2** unknown id → Err listing `logo` and `chart`.
- **P24-3** draft with no Content-ID attachments → Err pointing to
  `list_attachments`.
- **P24-4** received S1 copy → bytes equal the sent PNG (survives transport).

### PR #25 — context_lines
- **P25-1** html `<p>Line one</p><p>Line two</p><p>Line three</p><img
  src="cid:logo">`, `context_lines=2` → `context == "Line two\nLine three"`.
- **P25-2** no `context_lines` → `context` absent.
- **P25-3** inline image whose cid isn't in the HTML → `context == ""`.

### Regression smoke
- **R1** `list_folders` has Inbox/Drafts/Deleted Items/Sent Items.
- **R2** `list_emails(inbox, count 5)` returns ≤5 items.
- **R3** `reply_email(S1 copy, send=false)` → draft_saved; then deleted.

## Skip list

- `empty_deleted_items(confirm=true)` — would destroy the user's real Deleted
  Items. Covered by fake-client tests only.
- `count` clamping to exactly 200 — needs a folder with >200 items; only the
  ">50" half is checked.
- The DASL-only path for Hebrew (when Outlook's own search does match): we
  can't control which path the store picks. Both paths give the same visible
  result, which is what is asserted.

## Cleanup checklist

1. Permanently delete every tracked id (drafts, received S1 copy, Sent Items S1
   copy, reply draft).
2. Sweep Drafts/Inbox/Sent Items/Deleted Items for the tag, permanently, until
   a pass finds 0.
3. Remove scratch files (PNG, txt, save dir).
4. Raw PowerShell COM sweep: 0 items with the tag in those 4 folders; Deleted
   Items count back to its pre-run value.
