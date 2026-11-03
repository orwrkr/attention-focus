# T-0032: Report Period That Follows the Current Day

Status: done.
Dependencies: T-0030.

## Goal

A report left open must not keep showing the previous day after midnight, unless the
user chose fixed dates. Rolling presets already existed, but nothing showed that one was
active, and any date or time edit silently left the mode.

## Scope

Report panel period controls and date rollover only. No storage or accounting changes.

## Acceptance criteria

- [x] Today, Last 7 days and Last 30 days show as selected while they follow the date.
- [x] Custom is selectable, keeps the visible dates and shows that they stay fixed.
- [x] A one-line hint states the current mode and time zone.
- [x] Rollover updates filters immediately, even during debounce or a pending query.
- [x] A reply for the previous date is discarded, then one fresh query runs.
- [x] Fixed dates stay unchanged across midnight and restart.
- [x] The rollover uses the report's selected time zone.

## Checks

`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and
`cargo test --locked` passed. Tests inject wall time through `Request::recent_at` for
midnight rollover and exercise the period buttons through headless pointer events.
The saved `report_filter_v1` format is unchanged, so existing preferences still load.

This commit also adds accounting invariants for T-0033 in `src/reports.rs`. The test
fails when allocation is temporarily changed to per-window rounding.

Unverified: a real midnight rollover in the running application.

## Handoff

Editing a date or time switches to Custom by design, and the highlight shows it. Selecting
today's date by hand therefore stays fixed; choose Today to follow the current day.
