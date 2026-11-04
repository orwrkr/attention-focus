# T-0036: Graphical Activity Views

Status: backlog.
Dependencies: an owner decision on the options below.

## Goal

Show recorded activity graphically over a period, such as a day, several days or a week.

## Options

An interactive mock with invented data compared these at the app window's width:

- **A. Day timeline.** One row per day on a 24-hour axis, with one block per session and the day's
  total at the end. Shows when and on what, including breaks and context switching.
- **B. Totals by day.** Stacked columns per day by project. A single day becomes one split bar.
  Compares days and shows trends over weeks.
- **C. Weekly rhythm.** One cell per weekday and hour, in a single-hue scale. Shows habits, but
  needs weeks of history and cannot tell projects apart.

## Recommendation

Build A and B as two tabs in the existing reports section, sharing its period and time zone,
and leave C for later.

- Draw with the egui painter, without a plotting dependency. Record the choice in an ADR.
- Extend the worker's report with per-day clipped segments, using the same allocation as the
  totals and without the 200-row detail limit, so charts cannot disagree with the tables.
- Keep each project's color stable as others come and go. The validated categorical order
  supplies eight colors, with a neutral "Other" beyond them; the table stays complete.
- Style running time and time awaiting review differently from saved time.
- Handle 23- and 25-hour local days explicitly on the 24-hour axis.

## Decisions needed

1. Which views to build.
2. Automatic colors, or a color per project chosen by the user.
3. Inside the reports section, or in a separate larger window. The main window is limited to
   760 points wide.

No implementation has been started.
