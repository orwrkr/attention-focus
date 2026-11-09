# ADR-0043: A Write-Ahead Log for the History

Date: 2026-11-09.
Status: accepted.
Basis: T-0047, measured with `examples/history_scaling.rs` and direct commit timings. Replaces the
journal part of ADR-0004.

## Context

ADR-0004 chose a rollback journal with FULL sync and deferred WAL "until a demonstrated need". In
that mode each commit creates `focus.sqlite3-journal`, syncs it, writes and syncs the history,
then deletes the journal. Every Start, Stop, switch and ten-second checkpoint pays this on the
worker thread, whatever the size of the history.

Measured on the owner's machine through the real engine path, and through bare transactions with
changing content:

| Journal | Start or Stop, normal load | Start or Stop, little free memory |
| --- | --- | --- |
| Rollback, `DELETE` (before) | about 9 ms | 150 to 370 ms median, up to about 1 s |
| Rollback, `TRUNCATE` or `PERSIST` | about 9 ms | not measured |
| WAL | 2 to 3 ms | 2 to 4 ms |

The two load columns come from runs minutes apart under the same conditions for both modes. A
slow commit holds the worker, so a click waits, and ADR-0008 allows 1.5 seconds for a durable stop
at a Windows session end.

## Decision

- `Store::open` switches the history to `journal_mode=WAL` after any schema upgrade, so the
  upgrade path and its backup stay as tested. The mode is stored in the file.
- `synchronous=FULL` stays: each commit syncs the log, so a confirmed commit survives a crash or
  power loss as before.
- If the folder cannot hold the log, the rollback journal remains; nothing else depends on the mode.
- No schema change and no new setting. The automatic checkpoint and SQLite's removal of the log
  when the last connection closes are kept as they are.

## Alternatives

- **`TRUNCATE` or `PERSIST`** keep one file but measured the same as `DELETE`.
- **WAL with `synchronous=NORMAL`** would be faster still but can lose the last commits on power
  loss, against the durable-history invariant.
- **Keeping the rollback journal** leaves commits several times slower, and very slow when the
  machine is short of memory.

## Consequences

While the application runs, `focus.sqlite3-wal` and `focus.sqlite3-shm` sit beside the history,
and recent commits may be only in the log. A copy of `focus.sqlite3` alone, taken while the
profile is open, can miss them. The user guide already says to exit the profile and copy the whole
data directory, and that SQLite creates journal files there. A clean exit folds the log back and
removes both files.

The read-only export connection reads its own snapshot without blocking the worker. Older
versions open a WAL history unchanged; the mode is not part of the schema. Tests cover the
mode, exports with the history open and closed, recovery of a commit from the log as a crash
leaves it, and what a copy of the main file alone contains.
