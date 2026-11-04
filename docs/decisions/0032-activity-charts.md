# ADR-0032: Activity Charts in Reports

Date: 2026-11-04.
Status: accepted.
Basis: explicit user request; the owner chose all three views after comparing a mock.

## Context

Reports listed time in tables. The owner wanted to see activity graphically over a day,
several days or a week.

## Decision

Add a Charts section to Focus history and reports, with three views that share its period
and time zone. Each answers one question:

- **Timeline:** when, and on what? One row per day on a 24-hour axis, one block per session.
- **By day:** how much, and how was it split? Stacked columns, or one split bar for a single day.
- **Weekly rhythm:** when usually? One cell per weekday and hour, in a single-hue scale.

**Drawing.** Use the egui painter, with no new dependency. These are rectangles and text,
and `egui_plot` targets numeric axes, so it would add code without simplifying them.

**Numbers.** The worker computes `reports::Chart` next to the report, from the same records,
with the same validity rules and cumulative allocation, so charts cannot disagree with the
tables. Tests assert that days, segments, series and hourly cells add up to the totals across
daylight-saving days, gaps at midnight, skipped dates and non-hour offsets. Exports skip it.

**Time that is not final.** Finished time is solid, running time is a lighter fill with an
outline, and time awaiting review is an outline only. Review time stays out of day totals
and the weekly rhythm, matching Total.

**Local clock.** Blocks start at their local clock time and last their real duration, clamped
to the row. The hour repeated when daylight saving ends can therefore overlap on the row, and
a block on a short day can look longer than its clock times suggest. This matches what the user
remembers, and totals are unaffected. Hover text states the times the clock actually read.

**Colors.** Automatic, using one validated categorical order with separate light and dark
steps, checked for color vision differences. A project keeps its color as others come and go.
A new project takes the color used least by active projects, then by archived ones, so a
restored project rarely collides. Colors are interface preferences in `window.ron`: they are
not history, and losing them only assigns them again. Past eight active projects two can share
a color; in a chart the larger keeps it and the smaller joins a neutral Other, while hover still
names it. Unassigned focus has its own neutral color.

**Short marks.** A mark stays visible at any size. Unfinished marks keep a larger minimum, since
egui paints a narrow outline solid, which would make unfinished time look finished.

**Limits.** Timeline and By day cover up to 92 days and say so beyond that, since a row or
column per day stops being readable. Weekly rhythm has no such limit.

## Alternatives

- `egui_plot`: see above.
- A separate larger window: the reports section already holds the period controls, and the
  charts fit its width, which reaches 684 points. Labels that do not fit are left out.
- Colors in the history database: this would need another schema change for a preference.
- Colors from the project ID: two projects would collide even with few active projects.
- Automatic weekly or monthly columns for long periods: more calendar logic than the current
  need justifies. The message and Weekly rhythm cover long periods for now.

## Consequences

Details on a mark need a pointer, as egui tooltips are not reachable by keyboard. The By
project, By day and Sessions tables hold the same values. The report takes slightly longer
to compute, which only matters for very long histories, and it runs on the worker thread.
The native look was checked by rendering egui's actual shapes to images in both themes, not by
running the application, so the real window still needs a check.
