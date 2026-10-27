# ADR-0009: Date-Range Focus Reports

Date: 2026-10-27.
Status: accepted.

## Decision

Read history through the existing SQLite owner. UI report requests do not open another
writable connection or block the UI. Keep report failures separate from tracking failures.
Show saved totals, running time, and unreviewed checkpoints separately. Reports are
explicit snapshots refreshed by the user. Keep at most 200 recent detail rows in the
UI while computing totals over every matching session.

Use Jiff 0.2 with a bundled IANA timezone database. It provides date arithmetic,
historical DST rules, system timezone lookup, and compatible resolution of midnight
gaps/folds without a runtime download. This is preferable to manually implementing
timezone transitions or treating each day as 24 hours. The database increases binary
size and is updated with application dependencies.

From/Through dates are inclusive in the selected timezone; query a half-open UTC
interval ending at the beginning of the following local date. Limit one query to
3,660 civil days. Saved duration remains the recorded monotonic elapsed time. Allocate
it across the wall interval using cumulative integer proportions, subtracting cumulative
values at each clip boundary. This preserves exact totals across adjacent days and
queries even when small wall-clock drift makes wall and elapsed durations differ.
It is an allocation assumption, not reconstructed sub-session measurements. Major
discontinuities already require checkpoint review in the engine. Invalid zero/negative
wall intervals with nonzero elapsed time are excluded with diagnostics.

Group by source identity/path, project slug, and task snapshot. Preserve history when
source projects are archived, removed, or renamed; notes do not create extra group totals.
Use the newest title snapshot within each group for display. Do not join against live
MetaPlan or recursively add project relationships.

## Sources reviewed

- [Jiff civil dates](https://docs.rs/jiff/latest/jiff/civil/struct.Date.html)
- [Jiff timezone ambiguity handling](https://docs.rs/jiff/latest/jiff/civil/struct.DateTime.html#method.to_zoned)
- [Jiff day boundaries](https://docs.rs/jiff/latest/jiff/struct.Zoned.html#method.start_of_day)
