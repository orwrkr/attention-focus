# T-0004: Durable Sessions and Recovery

Status: done.
Dependencies: T-0003.

## Goal

Deliver durable sessions and recovery within the initial architecture.

## Scope

Follow [the architecture](../docs/ARCHITECTURE.md) and ADR-0004. Keep all authored
content and UI in English. Preserve external Unicode data and the read-only boundary.
Do not include later tasks or deferred product features.

## Plan

Add a clock-injected serialized engine, transactional activity snapshots and switching,
versioned backup-before-migration, and Windows lock/suspend notifications. Connect a
minimal project/task picker for exercising attribution; date-range reports stay T-0005.

## Acceptance criteria

- [x] Implement Idle/Running/Recovery transitions behind a serialized controller and single-instance ownership.
- [x] Persist start/stop/switch atomically; enforce one open session in SQLite and reject duplicate starts.
- [x] Test transaction rollback on write failure and rapid repeated commands with a fake clock.
- [x] Implement checkpoints, crash review, lock/suspend stop, explicit resume, and clock-jump handling.
- [x] Verify no overnight accrual on restart and no negative or overlapping finalized durations.
- [x] Document actual Windows lifecycle tests separately from simulated domain tests.

## Checks

Twenty tests passed with cargo test --offline --locked, including nine tracking tests
and one native notification-window test. Fake-clock cases cover twenty rapid switches,
exact shared boundaries, duplicate starts, failed-switch rollback after closing the old
row, failed stop, persistence, immutable Unicode snapshots, recovery after overnight
downtime, clock rewind/jumps, missed heartbeats, and delayed suspend command processing.
Legacy migration verifies the saved backup's old schema and unchanged totals.

The native listener was registered with Windows and received synthetic posted messages;
no real system lock or sleep was triggered. Real delivery timing, sleep/lock, new picker
interaction, and full clipboard/source workflow still require interactive verification.
The running older app locks its original EXE. The updated build is distributed separately
in dist/AttentionFocus-Update; the existing process and its session were not terminated.
Formatting, Clippy with -D warnings, and release build passed. Only environment-related
incremental-cache hard-link fallback warnings were emitted.
The final PE dependency audit contains only Windows system libraries, including the
new wtsapi32.dll and advapi32.dll imports. No sidecar runtime, SQLite, or asset DLL is needed.

## Handoff

Continue T-0005: search and date-range history/reports. Verify the complete workflow
after exiting the old app; record interactive tray and lifecycle evidence in T-0002.
