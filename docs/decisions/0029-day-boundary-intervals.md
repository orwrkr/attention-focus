# ADR-0029: Split Intervals at the Local Day Start

Date: 2026-11-04.
Status: accepted.
Basis: explicit user request; extends ADR-0008, ADR-0012 and ADR-0016.

## Context

A running interval could span midnight. Reports and Today already clip it correctly by
allocating its elapsed time over its wall-clock span, but history then shows one session
belonging to two dates, and a finished day keeps changing until that session stops.

## Decision

By default an interval never spans a local day start. When one has passed, the engine
closes the running row at that boundary with `end_reason='day_boundary'` and inserts a
new running row starting at the same boundary with an identical activity snapshot, in
one transaction. It does not count as a new use, so recent-use order is unchanged, and
the saved selection and any unapplied draft are untouched.

- **Calendar:** the current system time zone, as for Today (ADR-0016). Day starts use the
  same first-valid-start rule as reports, including DST gaps and skipped dates.
- **Time:** elapsed monotonic time is divided at the boundary, never recalculated, so the
  parts add up to the original duration even with small clock drift.
- **Timing:** checked on every worker tick and before every command, so a command after
  midnight applies to the new interval. A session ending exactly at the boundary stays
  one interval, without an empty continuation.
- **Durability:** the new row is written with a checkpoint at the current time, so a crash
  loses no more than an ordinary checkpoint interval.
- **Recovery:** gaps over 30 seconds and clock changes are still interrupted first at the
  last checkpoint. Sleep and lock stop tracking before midnight, without resuming.
- **Failure:** a failed split rolls back and retries on the next tick. Checkpoints continue,
  and explicit commands still apply, because reports clip an unsplit interval correctly.
- **Setting:** persisted per profile in `focus_preferences.split_on_day`, default on. When
  turned on during a long interval, every day start already passed is split on the next
  tick, so the rule holds without tracking when the option changed. Turned off, later
  intervals behave as before; finished rows are never merged.

Schema three adds that column. Following ADR-0012, an existing schema-two database is
backed up to `focus-before-v3-*.sqlite3` through the SQLite backup API and upgraded in one
transaction. Exports require the current schema version.

The engine also stops treating a lifecycle boundary as a clock rewind when it predates
the worker's last observation because the command waited in its queue. Both clocks move
back together then. A wall clock moving back while monotonic time advances is still a
clock change. A boundary captured just before a queued start still stops that interval.

## Alternatives

- Keep clipping only: totals were right, but it does not give the requested history.
- Store the option in `window.ron`, or add a table without a version bump: this would keep
  older EXEs working, but tracking rules would depend on UI preferences, or the schema would
  change without its version recording it. ARCHITECTURE.md requires explicit versions.
- Split only at midnight in the report time zone: reports can use any zone per query, and
  rewriting history for one of them would be wrong for the others.

## Consequences

Older EXEs refuse a schema-three database. Returning to one requires restoring the backup,
losing sessions recorded since. Current interval and the tray Session value restart at
midnight, All time gains the finished part, and the session count grows by one per crossed
day. Editing the current interval changes only the part after the last split. Reports
in another time zone still clip split rows correctly, but those rows then break at the
system-zone midnight rather than the report-zone one. Changing the system time zone
applies to the next boundary. T-0019 must treat split rows as ordinary finished sessions.
