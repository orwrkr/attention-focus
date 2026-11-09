# T-0047: Commit Latency and the SQLite Journal Mode

Status: backlog.
Dependencies: T-0046; the owner's decision.

## Goal

Decide whether to change the journal mode to reduce commit latency without compromising durability.

## Scope

T-0046 did not change journal mode. On the owner's machine Start and Stop take a median 150 to
370 ms, up to about one second, even with an empty history in `journal_mode=DELETE` and
`synchronous=FULL`; each commit creates and deletes `focus.sqlite3-journal`. A plain one-row
commit took under 1 ms. Windows file scanning interacting with journal churn is a possible cause,
not proven. With the same engine path on a WAL database Start and Stop took 2 to 4 ms. The
10-second checkpoint commit pays the same cost on the worker thread.

Research WAL with `synchronous=FULL` against `TRUNCATE` and `PERSIST`, including durability;
the `-wal` and `-shm` files and copying history while the app runs; read-only `export_snapshot`;
upgrade backups through the SQLite backup API (`backup_before_upgrade`); older versions opening
a WAL database; user guide wording about the history file; and tests. Measure with
`examples/history_scaling.rs` and record the decision in an ADR.

## Plan

Await the owner's decision, research alternatives and document the decision before implementation.

## Acceptance criteria

- [ ] Compare alternatives and their durability and compatibility implications.
- [ ] Record measurements, tests and a decision in an ADR.

## Checks

None yet.

## Handoff

The owner decides whether this is the next technical task. No implementation is selected.
