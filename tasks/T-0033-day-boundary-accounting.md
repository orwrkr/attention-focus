# T-0033: Day-Boundary Intervals and Verified Totals

Status: done.
Dependencies: T-0018.

## Goal

Split a tracked interval at the local day start by default, with an option to disable it,
and verify that totals are correct across interval, day and calendar boundaries.

## Scope

Engine, schema three, the setting and accounting tests. See ADR-0029. Report period
behavior is T-0032. History correction remains T-0019.

## Acceptance criteria

- [x] Crossing a local day start closes the old row at that boundary and opens a new one.
- [x] Both rows share one boundary and activity; elapsed time adds up exactly.
- [x] Selection and recent-use order are unchanged; the split is atomic and durable.
- [x] The option defaults to on, persists per profile and survives restart.
- [x] Disabling it keeps one interval with identical totals per day and overall.
- [x] Enabling it during a long interval divides every day start already passed.
- [x] DST gaps, 23/25-hour days, skipped dates and non-hour offsets are handled.
- [x] Crash, failed split, stop at exactly midnight and later commands are covered.
- [x] Schema two upgrades with a backup; failure keeps schema two; newer history is refused.
- [x] Totals by day, project, task and session agree, and adjacent partitions add up.
- [x] Today counters match a report for the same zone before and after midnight.

## Audit of existing totals

No arithmetic defect was found in report allocation. Deterministic randomized tests
compare groupings and partitions in UTC, Istanbul, New York, Sao Paulo, Apia, Kathmandu and
Lord Howe. The main invariant failed as expected when allocation was temporarily changed
to per-window rounding, so it can detect this class of error.

One accounting defect was found and fixed: a lock or suspend boundary processed after a
later worker observation looked like a clock rewind. The session then ended at its last
checkpoint, up to ten seconds early, and required review instead of stopping cleanly.

Documented behavior retained: rows show whole seconds, so displayed rows can differ
from a displayed total by rounding. By day and group tables hold saved time only, while
Total adds running time. Needs review is excluded everywhere.

## Checks

`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and
`cargo test --locked` (106 tests in total) passed. All timing tests use injected clocks and
temporary databases. Production was not opened or modified.

An independent read of the diff found no lost time, overlap or migration defect. It found
that a failing split also failed report requests, which was fixed and is now tested.

Unverified: a real midnight crossing in the running application, and the upgrade of a
real Production database. The upgrade path was only exercised on synthetic databases. The
failed-upgrade test proves that schema two and history survive a failing statement; it does
not force a failure after that statement, so it does not separately prove rollback.

Release: both 0.18.1 EXEs and the ZIP were built through `scripts/build.ps1`, and
`scripts/verify-release.ps1` passed. All nine PDF pages were rendered and reviewed, and the
shipped guide hash matches the preview. The earlier 0.18.0 archive has two nearly empty PDF
pages, found in this review, and was superseded before handoff. The package tests passed.

## Known limits

- If a lock boundary captured before midnight waits behind another queued command that runs
  just after midnight, the split comes first. The finished day then keeps that queue delay,
  usually milliseconds, and the stop leaves an empty row at midnight. Totals remain correct
  to within that delay.
- In historical time zones where local dates step backward across a repeated hour, an
  interval starting inside that hour is not split. Reports still clip it by day.

## Handoff

Leave Development tracking across midnight, then check the two sessions and their end
reason in Focus history and reports. Before first running this version on Production,
note that the upgrade is one-way for older EXEs; see ADR-0029.
