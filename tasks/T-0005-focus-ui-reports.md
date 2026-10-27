# T-0005: Focus Workflow and Time Reports

Status: in_progress.
Dependencies: T-0004.

## Goal

Deliver focus workflow and time reports within the initial architecture.

## Scope

Follow [the architecture](../docs/ARCHITECTURE.md) and ADR-0004. Keep all authored
content and UI in English. Preserve external Unicode data and the read-only boundary.
Do not include later tasks or deferred product features.

## Plan

Add project search and worker-owned report requests, pure interval allocation,
daily and project/task totals, and recent session details. Resolve inclusive date
ranges in an explicit IANA timezone with Jiff's bundled database. Keep saved, running,
and unreviewed time separate. Verify clipping, DST, midnight, identity, and UI behavior.

## Acceptance criteria

- [x] Connect source settings, project search, optional task, Start/Stop/Switch, and tray controls to the controller.
- [x] Show confirmed timer state and actionable storage/source errors in English.
- [x] Display session history and project/task totals for a selected date range with uncertain time separated.
- [x] Test clipped intervals, midnight and daylight-saving boundaries, Unicode titles, and project archival.
- [ ] Manually verify the full settings-to-report workflow and record release build instructions.
- [x] Document deferred export, editing, shortcuts, autostart, and installer work without presenting them as delivered.

## Checks

Implementation is complete; full interactive tracking acceptance remains pending.
Twenty-seven tests passed with cargo test --offline --locked. Added checks cover
calendar-day boundaries (23/25-hour DST days, midnight gaps, skipped dates), clipped
and adjacent interval additivity, monotonic/wall drift allocation, Unicode snapshots,
source identity, archival independence, history truncation without truncated totals,
and worker report errors that do not acknowledge tracking commands or block Stop.
All automated data is synthetic in temporary storage; no live MetaPlan tests were added.

Formatting, Clippy with -D warnings, packaging tests, and scripts/build.ps1 passed.
The canonical EXE has only Windows system DLL imports. The build consumes the main
Cargo artifact and creates no alternate release. With the app then running, an intentional
second build failed before compilation with the expected locked-file error (exit 1).
The already published EXE was unchanged; dist contains only AttentionFocus/attention-focus.exe.

Interactive checks: opened the canonical EXE; verified source-field Ctrl+C, clearing
unsaved text, and Ctrl+V restoring it without saving source settings. Restored selection
and source appeared. Verified project filtering without selecting/applying another
activity, report refresh, project/task/day totals, session-detail expansion, invalid-date
errors, and recovery to a valid Today report. Did not start/switch/stop fake sessions
in real history. Real lock/suspend, full synthetic UI tracking, tray, and geometry checks
remain unverified. No personal names, paths, or session figures are recorded here.

The user reported exiting the agent-launched app. Process inspection then found no app,
and scripts/build.ps1 successfully published again to the canonical path. A final UI
discovery request was stopped by the user's physical Escape key; no further UI input
was sent. This does not invalidate the report/clipboard checks already recorded above.

## Handoff

Complete the remaining manual tracking/tray/lifecycle checks with disposable history,
then mark this task done. T-0014 added native Development evidence for synthetic source
settings, project filtering/selection, minute/calendar filters and start/stop reporting.
Historical ranges paused while Today updated live; Stop moved Running to Saved.
Task switching/edit mode, real tray, geometry and lock/suspend/recovery still need
acceptance. Development 0.5.0 remains open and stopped on synthetic data; obtain approval
before closing it. The external portable Production instance was left untouched.
Export and manual corrections remain deferred; do not expand this task to include them.
