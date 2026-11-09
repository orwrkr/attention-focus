# ADR-0042: History Growth and Query Cost

Date: 2026-11-09.
Status: accepted.
Basis: T-0046.

## Context

Synthetic histories measured through the real store, engine and reports exposed growing costs.
Day rows allocated every session to every day: a whole-period report took 3.3 seconds for ten
years at 20 sessions a day and 8.3 seconds at 50. While tracking, a report including today is
recomputed every two seconds, occupying the worker and delaying Start and Stop. SQLite scanned
all of `focus_sessions` for period queries, including today, taking 80 to 210 ms per refresh at
ten years; `ORDER BY id` alone also favored a scan. Each command rebuilt a snapshot whose catalog
decoded every session's activity to mark usage, taking about 480 of 665 ms at 182,500 sessions;
saved totals and pending review also required separate full passes.

Data volume is small: about 1.3 MB per year at 20 sessions a day and about 0.1 MB in the owner's
two-week history. Ticks, checkpoints and today counters were already index-bound and constant.

## Decision

- Allocate sessions only to overlapping days, found by two binary searches. Report allocation
  takes one pass over days plus the days sessions span.
- Split `store::REPORT_ROWS` into a `UNION ALL` for ended and open sessions, both using
  `focus_sessions_ended`, and sort by id in Rust. Rows and order stay the same.
  `engine::DAILY_ROWS` names the today-counter query.
- Keep recorded project/task keys in `Store.usage`, initially read from history. After commit,
  `begin` adds its key; `reassign`, `save_interval` and `delete_interval` invalidate the set.
  `split_day` and `rename_project` keep it. Any new writer of `focus_sessions.activity` must add
  to or invalidate the set.
- Compute saved total, count and pending review in one aggregate pass. A stopped activity change
  checks review with one query instead of rebuilding a snapshot. No schema change is needed.
- Use `examples/history_scaling.rs` for machine-dependent measurements; tests assert query plans
  and results, not timings.

## Alternatives

A partial index on state would remove the remaining aggregate and `begin` review scans, but is
deferred to the next schema change: schema six requires a backup and upgrade tests. Changing the
journal mode is a separate decision in T-0047.

## Consequences

The benchmark trends improved (see T-0046), though individual timings vary with machine load.
One aggregate pass per snapshot and the review check in `begin` remain linear, taking tens of
milliseconds at ten years. Opening reads usage once, about 0.3 seconds at ten years and 0.6 seconds
at 182,500 sessions. A long real history and the multi-year report window remain unverified; By day
still lays out every day each frame.
