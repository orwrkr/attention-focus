# T-0004: Durable Sessions and Recovery

Status: backlog.
Dependencies: T-0003.

## Goal

Deliver durable sessions and recovery within the initial architecture.

## Scope

Follow [the architecture](../docs/ARCHITECTURE.md) and ADR-0004. Keep all authored
content and UI in English. Preserve external Unicode data and the read-only boundary.
Do not include later tasks or deferred product features.

## Plan

Implement the acceptance criteria in order, then verify and update project context.

## Acceptance criteria

- [ ] Implement Idle/Running/Recovery transitions behind a serialized controller and single-instance ownership.
- [ ] Persist start/stop/switch atomically; enforce one open session in SQLite and reject duplicate starts.
- [ ] Test transaction rollback on write failure and rapid repeated commands with a fake clock.
- [ ] Implement checkpoints, crash review, lock/suspend stop, explicit resume, and clock-jump handling.
- [ ] Verify no overnight accrual on restart and no negative or overlapping finalized durations.
- [ ] Document actual Windows lifecycle tests separately from simulated domain tests.

## Checks

Not started. Record commands, results, and manual evidence here.

## Handoff

Start after T-0003 is complete. Update this card and the task index together.
