# T-0019: History Correction and Manual Intervals

Status: ready.
Depends on: T-0018.

## Scope and plan

Implement the next deferred product capability: explicit correction of finalized
sessions and manual addition of missed intervals. First define the edit, overlap,
recovery and audit rules in an ADR. Keep active/review sessions protected, preserve
original imported activity snapshots and never edit MetaPlan. Do not migrate Production
as part of development or use its history as fixtures. Review compatibility with an
older running Production EXE before choosing any schema change.

## Acceptance

- [ ] Record the data model, validation/overlap policy and migration/backup strategy.
- [ ] Provide a clear editor with project/task/note and minute-precise boundaries,
  selected timezone, before/after duration preview and explicit save/cancel behavior.
- [ ] Apply changes transactionally with durable audit information and clear errors.
  Refresh affected reports and both daily totals after successful saves.
- [ ] Verify invalid/DST boundaries, conflicts with active intervals, restart durability,
  failure rollback, Unicode, source integrity and profile isolation using synthetic data.
- [ ] Document user behavior and compatibility limits, build both versioned EXEs and
  refresh/review/verify the guide and production ZIP. Perform Development UI acceptance.

No implementation or database change has been made for this card. Global shortcuts,
autostart and installer work follow later; T-0002 retains the real desktop/OS gates.
