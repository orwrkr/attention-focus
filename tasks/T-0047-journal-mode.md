# T-0047: Commit Latency and the SQLite Journal Mode

Status: done.
Dependencies: T-0046; the owner's decision (taken on 2026-11-09).

## Goal

Make Start, Stop and checkpoints commit quickly, also when the machine is under load, without
weakening durability.

## Scope

Choose the journal mode from measurements and switch the history to it. No schema change, no new
setting. Record the decision in an ADR (ADR-0043, replacing the journal part of ADR-0004).

## Plan

Measure the rollback journal against WAL, `TRUNCATE` and `PERSIST` through bare transactions with
changing content and through the engine path; switch in `Store::open` after any upgrade; test
durability, exports and copies of the files.

## Acceptance criteria

- [x] The alternatives are measured and compared for durability and compatibility (ADR-0043).
- [x] The history uses WAL with FULL sync; the upgrade path and its backup are unchanged.
- [x] Exports read the history while it is open and after a clean exit.
- [x] A committed session survives a crash through the log, and a clean exit leaves one complete file.

## Checks

- Measurements on the owner's machine, real engine path, Start or Stop: rollback journal about
  9 ms and WAL 2 to 3 ms at normal load; 150 to 370 ms median (up to about 1 s) against 2 to 4 ms
  when the machine had little free memory, both modes measured minutes apart. Bare transactions
  with changing content: `DELETE`, `TRUNCATE` and `PERSIST` about 9 ms each, WAL about 2.3 ms.
  T-0046 first reported 150 to 370 ms as the normal cost; that run was under memory pressure.
- `examples/history_scaling.rs` after the switch, on a quiet machine: Start and Stop 4 to 5 ms for
  a year at 20 sessions a day, 34 to 54 ms for ten years, 79 to 130 ms for ten years at 50 a day;
  what remains there is the linear review check and aggregate pass recorded in ADR-0042.
- `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo test --locked`: 250 passed, in about 3 seconds instead of about 27. Two new tests:
  `history_uses_a_synced_write_ahead_log_that_exports_read_and_a_clean_exit_folds_back` checks
  `journal_mode=wal` and `synchronous=2`, exports with the history open and closed, and that the
  log is folded back and removed on exit; `a_commit_survives_a_crash_in_the_log_but_not_in_a_copy_of_the_main_file_alone`
  copies the files while the store is open and recovers the commit from the log. The existing
  upgrade tests from every older schema pass with the switch after the upgrade.
- Version 0.26.5: `scripts/build.ps1` refused to rebuild the already packaged 0.26.4 with changed
  inputs, as intended. 0.26.4 stays built with T-0046 alone and is not published.
  `scripts/build.ps1`, `scripts/verify-release.ps1`, `scripts/test-release-output.ps1` and
  `scripts/test_release_tools.py` passed for 0.26.5. The guide changed only in its version.

Unverified: the switch of the owner's real Production history, which happens on the first start
of 0.26.5; a real power loss; the log on folders where SQLite cannot use shared memory, where the
rollback journal stays.

## Handoff

0.26.5 holds T-0046 and T-0047. After Production runs it, `focus.sqlite3-wal` and `-shm` appear in
its data directory while the application is open; that is expected.
