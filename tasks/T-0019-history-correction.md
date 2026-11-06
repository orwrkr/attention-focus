# T-0019: History Correction and Manual Intervals

Status: done.
Depends on: T-0018.

## Goal

A context menu on chart intervals and session rows: delete after confirmation, and create or edit through a
dialog with start and end, as the owner asked on 2026-11-05. See ADR-0039 for the rules.

## Acceptance

- [x] Data model, validation, overlap policy and migration recorded (ADR-0039).
- [x] Timeline menu on intervals and empty space; session row menu; buttons for keyboard use.
- [x] Dialog with project, task, note, and exact boundaries in the selected time zone, a before and after
  duration preview, and explicit save and cancel. Delete asks for confirmation.
- [x] Changes apply in one transaction with durable audit information and clear errors, and refresh reports
  and both daily totals.
- [x] Synthetic tests: invalid and daylight-saving boundaries, conflicts with running and interrupted
  intervals, restart durability, failure rollback, Unicode, and upgrades from every older schema.
- [x] Guide and product documents updated, version raised, release folder rebuilt and verified, and the
  Development window checked.

## Checks

Codex (GPT-6.1 Sol) implemented the brief; its sandbox could not run the startup registry test. DeepSeek V4 Pro
reviewed the change adversarially. Its findings, and those of a separate reading, were fixed with tests: a dialog
that could stay disabled when the outbox refused a command; a report cleared after every save, which reset the
scroll position; a reused session ID letting a stale view change another interval, now refused through the
boundaries the interface showed; a failure after a committed write not refreshing the report; note-only changes
that replaced the recorded title or were blocked by order and future checks; an empty-space draft overlapping the
interval before it; an empty menu outside the rows.

`cargo fmt --check`, Clippy, `cargo test --locked` (244 tests), `scripts/test_release_tools.py`,
`scripts/test-release-output.ps1`, `check-notices`, `scripts/build.ps1`, `scripts/verify-release.ps1` and
`git diff --check` passed for 0.25.2. The guide was previewed page by page before the build.

Checked in the real Development window on 2026-11-05, in Russian and Arabic: the upgrade to schema five with its
backup; the timeline menu on an interval and on empty space; editing, with the time picker inside the dialog,
Escape closing only the picker, an overlap rejected with the dialog kept, and a touching end saved; adding from
empty space with a project chosen; delete with cancel, Escape and confirmation; the session row menu and buttons;
Today, totals and the chart refreshing without losing the scroll position; three audit rows with the right values
and no overlaps in the file. A menu right-aligned for Arabic stretched across the window, so it stays compact and
left-aligned. Each correction after the first build changed shipped input, so 0.25.0 and 0.25.1 were built but are
superseded by 0.25.2.

The owner upgraded Production to 0.26.3 on 2026-11-06. A read-only check of its database printed
only metadata: `user_version` 5, the `focus_session_edits` table present, one `focus-before-v5-` backup
beside it, and `quick_check` ok.

Unverified: a running interval's disabled menu in the window, which a
test covers; keyboard-only use of the dialog beyond Tab order in tests; daylight-saving boundaries in a real zone
with such a change, which tests cover with America/New_York.

## Handoff

The audit table makes an undo or an "edited" mark possible later; neither is requested.
