# Plan — system-test all open PRs (#14–#25), 2026-10-03

## Goal
Merge every open GitHub PR into one integration branch, update the
live-outlook-system-test skill + write a dated system-test plan, run the
live system test over the merged result, fix whatever fails, and report.

## Inputs
12 open PRs by YanayGoor, all against `main`:
#14 list_emails offset/cap 200 · #15 non-ASCII search fallback · #16 permanent
delete + empty_deleted_items · #17 list_emails `to` filter · #18 attachment
metadata · #19 update_draft · #20 get_email truncation flags · #21 UTF-8
audit/Hebrew tests · #22 inline images on send/draft · #23 is_inline flag
(stacked on #18) · #24 get_inline_image (stacked on #18) · #25 context_lines
(stacked on #24).

## Steps
1. **Isolate.** Worktree `.worktrees/systest-all-prs`, branch
   `systest/all-prs-2026-10-03` from `origin/main`. The main checkout's
   uncommitted multi-account WIP is left untouched.
2. **Merge.** `git merge --no-ff pr-N` in order 18, 23, 24, 25, then 14–17,
   19–22. Resolve conflicts (expected hot spots: `server.rs` tool router,
   `client.rs`, `fake.rs`, `mod.rs`, `README.md` tool counts, `tests/tools.rs`,
   `tests/live_outlook.rs`), keeping both sides' behaviour.
3. **Offline gate.** `cargo build`, `cargo clippy --all-targets`, `cargo test`
   (unit + FakeOutlookClient + http transport). Fix anything broken.
4. **Update the skill** (`.claude/skills/live-outlook-system-test/SKILL.md`):
   add a "testing a batch of PRs" section (integration-branch workflow,
   per-PR attribution of failures), Hebrew/non-ASCII and attachment/inline-image
   guidance, permanent-delete safety (only ever on tagged items; never call
   `empty_deleted_items` against a Deleted Items folder that holds non-test
   data), and the worked-example pointer for this run.
5. **Write `SYSTEM_TEST_PLAN_2026-10-03-PRS.md`**: scope = new/changed
   behaviour from PRs 14–25, plus a smoke regression of core tools; mechanism
   = direct `WindowsOutlookClient`; tag `[outlook-mcp-rs systest PRS]`;
   self-loop sends only (adamkopelman@outlook.com); seed data; one test id per
   behaviour with concrete expectations; skip list; cleanup checklist.
6. **Implement** `tests/system_test_prs.rs` — one `#[ignore]`d, non-panicking
   test with a results log, unconditional tagged cleanup, summary table, final
   assert.
7. **Run live** (`cargo test --test system_test_prs -- --ignored --nocapture`),
   plus the PRs' own `#[ignore]`d tests in `tests/live_outlook.rs`.
8. **Root-cause + fix** each failure (isolated probe first, then fix with a
   test, re-run live). Commit fixes on the integration branch, each attributed
   to the PR it belongs to.
9. **Verify cleanup** with a raw PowerShell COM sweep for the tag.
10. **Report** `SYSTEM_TEST_RESULTS_2026-10-03-PRS.md` + summary to the user:
    per-PR status, failures and fixes, conflicts resolved, anything left open.
    No pushing to GitHub or commenting on PRs without the user's say-so.
