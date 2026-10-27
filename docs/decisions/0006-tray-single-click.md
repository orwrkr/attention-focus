# ADR-0006: Single-Click Window Access and Hover Time

Date: 2026-10-27.
Status: accepted.
Basis: explicit user request.

## Decision

Single left click shows the window, superseding ADR-0005's ignored-single-click rule.
Double click continues to toggle tracking. Delay single-click activation using the
Windows double-click interval, cancel it on a double click, and ignore the final
release of that double click. No continuously polling click timer is needed.

Use the native Windows tray hover tooltip for the requested balloon: status, current
session duration, and accumulated saved plus active time. Refresh it from worker
snapshots once per second while tracking. Windows controls tooltip appearance/delay;
do not create repeated notification-center messages on mouse hover.

## Consequences

Single-click opening has a short system-configured delay. Unreviewed interrupted time
is excluded from totals and called out. English-only authored UI remains unchanged.
