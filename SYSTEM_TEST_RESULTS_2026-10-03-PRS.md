# System test results: open PRs #14–#25 merged together, 2026-10-03

Plan: `SYSTEM_TEST_PLAN_2026-10-03-PRS.md`. Branch: `systest/all-prs-2026-10-03`
(worktree `.worktrees/systest-all-prs`), based on `origin/main` @ `3eac5a5`.
Test: `tests/system_test_prs.rs`. Mailbox: `adamkopelman@outlook.com` (Outlook.com).

## Outcome

- **Final run: 43/43 PASS.** The first run was 40/43:
  - one real bug, in PR #16 (fixed);
  - two wrong expectations in my own test (corrected).
- The PRs' own 11 `#[ignore]`d live tests in `tests/live_outlook.rs` pass on the merged branch.
- Offline suite passes: lib 179, tools 76, http_transport 2.

## Per-PR status

| PR | Topic | Merge | Live tests | Needs a change before landing |
|---|---|---|---|---|
| #18 | attachment metadata | clean | P18-1, P18-2 PASS | — |
| #23 | is_inline | clean (on #18) | P23-1, P23-2 PASS | — |
| #24 | get_inline_image | conflict with #23 (`client.rs` imports + helpers, `live_outlook.rs`) | P24-1..4 PASS | **yes**: `attachment_info` now takes `html_body` (from #23), so `get_inline_image` must pass `""`. Also rename its result type (see #22). |
| #25 | context_lines | clean (on #24) | P25-1..3 PASS | — |
| #14 | offset / cap 200 | conflict with #25 (`mod.rs`) | P14-1..3 PASS | test literals in other PRs need `offset: 0` |
| #15 | non-ASCII search | conflict with #14 (`list_emails` loop, consts, README) | P15-1..4 PASS | hand-merged with #14's `take_page` |
| #16 | permanent delete / empty_deleted_items | import conflicts | P16-2, P16-3 PASS; **P16-1 FAILED → fixed** | **yes: bug fix below** |
| #17 | `to` filter | conflict with #14/#15 (`list_emails`, fake, server description, README, test literals) | P17-1..4 PASS | hand-merged with #14 + #15 |
| #19 | update_draft | conflict with #16 (`delete_email` neighbours in 5 files) | P19-1..5 PASS | — |
| #20 | truncation flags | conflict (consts, test modules, README) | P20-1, P20-2 PASS | stale `get_email(id, bool)` calls in other PRs' tests |
| #21 | UTF-8 / Hebrew | conflict (com.rs, fake.rs `get_email`, live tests) | P21-1, P21-2 PASS | — |
| #22 | inline images on send/draft | conflict (README, constants, mod.rs, tools.rs) | P22-1..4, S1 PASS | **yes**: type-name clash with #24 and a duplicate `base64` in Cargo.toml (below) |
| — | regression smoke | — | R1–R3 PASS | — |

## Failures and root causes

### P16-1: `delete_email(permanent=true)` left the item in Deleted Items (real bug, PR #16)

- **Symptom:**
  - The call returned `{"status":"deleted","permanent":true}`.
  - The old id no longer resolved.
  - But the item was still sitting in Deleted Items.
  - The first run's cleanup sweep found 19 such leftovers.
- **Root cause:**
  - For an item outside Deleted Items, the code does `item.Move(DeletedItems)` and then `Delete()` on the object `Move` returns.
  - On this Outlook.com store, that `Delete()` silently does nothing.
- **How it was confirmed:** a raw PowerShell COM probe, bypassing this codebase.
  - `Move` then `moved.Delete()` left 1 item. Waiting 3 s before `Delete()` still left 1, so it isn't timing.
  - `GetItemFromID(moved.EntryID, DeletedItems.StoreID).Delete()` left 0.
- **Fix:** commit "Fix permanent delete_email leaving the item in Deleted Items (PR #16)". It re-opens the moved item by EntryID and deletes that fresh object.
- **Verification:**
  - PR #16's own live test `permanent_delete_of_draft_skips_deleted_items` **fails** without the fix ("a permanently deleted draft must not remain in Deleted Items") and passes with it.
  - P16-1 passes.
  - It probably slipped through because #16 was tested against an Exchange account.

### P20-1 / P20-2: wrong expectation in my test, not a bug

The truncated body was 1036 characters, not 1000, because `truncate` appends `"\n\n[... truncated at 1000 characters]"`. That marker predates PR #20, which keeps it. I fixed the assertions to "first 1000 chars + marker" and updated the plan.

## Cross-PR integration fixes, made in merge commits

These conflicts are invisible to git, so each PR passes CI on its own but they break once combined. Whichever PRs land later need these changes:

1. **Type-name clash.** #22 adds `outlook::InlineImage`, the input type for `send_email`/`create_draft`, and it appears in the tool schema as `$defs/InlineImage`. #24 adds `types::InlineImage`, the result of `get_inline_image`. On the merged branch I renamed #24's type to `InlineImageData`.
2. **Duplicate `base64 = "0.22"`** in `Cargo.toml`: #22 and #24 both add it.
3. **New fields and arguments break other PRs' test call sites:**
   - `EmailQuery.offset` (#14) and `EmailQuery.to` (#17);
   - `delete_email(.., permanent)` (#16);
   - `get_email(.., max_body_chars)` (#20);
   - `create_draft`/`send_email(.., inline_images)` (#22);
   - `GetEmailParams.max_body_chars` and `CreateDraftParams.inline_images` in `tests/tools.rs`.
4. **`attachment_info` signature** (#23 adds `html_body`) breaks #24's `get_inline_image`. Fixed by passing `""`, since the result doesn't expose `is_inline`.
5. **`list_emails` filter loop** was rewritten by #14 (lazy iterator + `take_page`), #15 (non-ASCII fallback + 2000-item cap) and #17 (recipient fallback + 2000-item cap). I merged it by hand into one iterator that applies both fallbacks, caps the scan at the smaller limit, then pages.
6. **README:** each PR said "27 tools". The merged total is **29** (+update_draft, +empty_deleted_items, +get_inline_image). The `list_emails`/`get_email`/`send_email` lines were combined.

## Not tested live (by design)

- `empty_deleted_items(confirm=true)`: Deleted Items holds 53 of the user's own items. Only the refusal path was tested (P16-3: refused, count unchanged). The real path is covered by fake-client tests only.
- `count` clamping at exactly 200: no folder has more than 200 mail items. P14-3 only checked that more than 50 come back.

## Cleanup

- Every item the test created was permanently deleted, and a tag sweep ran over Drafts, Inbox, Sent Items and Deleted Items.
- Running the PRs' own live tests left 6 drafts in Deleted Items. Their cleanup uses a soft `delete_email(id, false)`. I removed those 6 by exact subject.
- Two probe items from my PowerShell COM checks were removed in the same probes.
- **Final raw-COM state, matching the start of the session:**
  - Inbox 47, Drafts 12, Deleted Items 53, Outbox 0;
  - 0 `outlook-mcp-rs` test items touched in the last 2 h in any of those folders or in Sent Items.

## Landing status

Nothing has been merged to `main` and nothing has been pushed to the PR branches yet: the permission check blocked both. These changes are needed when the PRs land:

- **#16:** add the permanent-delete fix commit. It is prepared locally as branch `upd-16` in `.worktrees/landing`. Offline tests and #16's live test pass on it.
- **#24:** rename the result type to `InlineImageData`. Prepared locally as an uncommitted change on `upd-24`. #25 then needs #24's update merged in, plus the same rename.
- Everything else is the conflict resolution listed above, done as a "merge main into the PR" commit just before each PR lands.
- Tool count in the README: 27 after #24, 28 after #16, 29 after #19.

## Other observations (not failures)

- **Two different `cid:` matchers.** #23's `com::is_inline` and #25's `mod::find_cid_reference` use different end-of-id rules: #23 stops at whitespace, quotes, brackets or parens; #25 stops at any non-Content-ID character. So an image referenced as `cid:logo;` (or similar) can get `is_inline=false` while `context_lines` still finds the reference. Worth merging into one helper.
- **Clippy.** The PRs add two "too many arguments (9/7)" warnings, on `send_email`/`create_draft` after `inline_images`. There are no other new warnings compared with `origin/main`.
- **PR live tests leave debris.** Several PR live tests clean up with a soft delete, which leaves drafts in Deleted Items. Now that #16 exists, they could use `permanent=true`.
