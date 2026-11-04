# T-0036: Graphical Activity Views

Status: done.
Dependencies: T-0035.

## Goal

Show recorded activity graphically over a period, such as a day, several days or a week.

## Scope

Three views in the reports section, as chosen by the owner after comparing a mock: Timeline, By
day and Weekly rhythm. Colors are automatic and charts stay inside the reports section, as
recommended, since the owner changed only the choice of views. See ADR-0032.

## Acceptance criteria

- [x] Timeline shows sessions per day on a 24-hour axis, with each day's total.
- [x] By day shows stacked columns; a single day becomes one split bar with shares.
- [x] Weekly rhythm shows time per weekday and hour, including over long periods.
- [x] Every view follows the report's period and time zone and refreshes with it.
- [x] Chart numbers add up to the report's totals across calendar boundaries.
- [x] Running and review time look different from finished time, and review stays out of totals.
- [x] A project keeps its color across periods and as other projects come and go.
- [x] Hovering a mark shows its details; long or empty periods explain themselves.
- [x] The selected view and the colors survive a restart.
- [ ] Native check in the running application, see Handoff.

## Checks

`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and
`cargo test --locked` passed. The palette passed the validator for the three leading
colors in both themes on 2026-11-03; the full eight-color order is the validated default.

The look was checked once without running the application: a throwaway test, not committed,
converted the shapes egui actually paints to SVG, in both themes, and these were rendered to an
image. That showed correct layout, and led to raising the contrast of axis labels in the dark
theme. It uses a different text renderer than the application.

Unverified: the charts in the real window, including hover at real sizes and scrolling inside
the long timeline. An interactive mock with invented data was published privately for the
choice of views; it is not part of the repository.

## Handoff

Check in Development: open Focus history and reports, switch the three views and periods,
hover over marks, and track across a report refresh to see the running interval.
