# Current Context

Updated: 2026-10-28.

## Current state

Current release: 0.10.0. T-0018 is complete; next implementation card is
[T-0019](../tasks/T-0019-history-correction.md), history correction/manual intervals.
That card is defined but not implemented. Global shortcuts, autostart, installer and
updater remain deferred. T-0002 is a separate real desktop/OS acceptance gate.

The local Rust app uses egui/eframe, tray-icon and bundled SQLite. Cargo.lock pins
versions. MetaPlan is read-only. Source normalization and directory-link resolution
protect every write destination. Never use personal source data as repository fixtures.

Current interval has the prominent blue Start/Stop action. Today for project and
Today - all projects share cached local-calendar-day accounting with the tray. Include
saved plus active time once, all tasks and (for the global sum) all sources/unassigned
focus; exclude review time. Report filters/timezones do not change Today. Errors show
unavailable without blocking successful tracking state. Schema remains two.

Counter frames use measured margins and a mirrored reserved scrollbar gutter. Narrow
windows place the interval above two daily cards; wide windows use three columns.
Only egui's automatic elided-name tooltip is enabled. Native width is limited to 760
logical pixels. The maximized-bounds subclass accounts for current monitor work area
and DPI-scaled resize borders, including once at restored startup. No measured UI
height or size/maximize commands run per frame. See ADR-0018.

Sections/scroll, custom report dates/minutes or rolling presets, timezone/project
searches, note draft, window geometry and Stay on top persist separately per profile.
Restoring preferences never restarts tracking. Relative report presets roll forward.
Project/task changes split intervals by default; the persistent opt-out reassigns the
current interval without changing its start. Pickers use durable recent-use ordering.

Reports use half-open minute ranges and bundled timezone rules. Periodic queries run
only while live time grows inside the filter; affected stop/edit/recovery events refresh
once. Saved, Running and Needs review remain separate. CSV/Markdown exports read an
independent immutable checkpoint snapshot, include every match beyond the UI preview,
protect MetaPlan/app-storage paths and never overwrite. See ADR-0014/0017 and T-0016.

## Lifecycle and profile boundaries

Closing the window hides it if the tray is available; Exit saves before closing.
Received lock/suspend notifications stop without automatic resume. Confirmed Windows
session end saves with bounded acknowledgement; canceled queries do not stop tracking.
Forced exits, missed heartbeats and clock discontinuities use checkpoint review.
Checkpoints are approximately ten seconds apart, not a loss guarantee under IO failure.

Settings/history live in the Windows LocalAppData known folder under AttentionFocus/
Production or Development, outside MetaPlan. Debug builds reject Production. Profile
locks, histories and geometry are independent. Legacy root history remains untouched.
Schema two dates from 0.3.0; schema-one upgrade backs up first. Older EXEs cannot read
schema two. T-0019 must research compatibility before introducing further migrations.

## Checks and exact next step

64 Rust tests, formatting, Clippy and five package tests passed. Restricted junction
creation failed in one rerun; all 64 passed with the required Windows link permissions.
Tests cover responsive margins at 100-200 percent scale, mirrored scrollbar gutters,
a single name hint, native monitor/frame bounds and project/global daily accounting.
Native Development verified prominent Start/Stop, live daily counting, stopped reset,
previous-day report independence, restored maximized startup and final narrow/wide
layouts. Earlier downward sizing passed without snapback. Both 0.10.0 EXEs, generated
notices, eight-page PDF and automatic ZIP passed verify-release metadata/icon/hash/
fingerprint/allowlist checks. All eight rendered guide pages were visually inspected.

All agent-created sessions are synthetic and Development-only under the ignored
manual fixture. Development exited normally after checks; a process check confirmed
only the external portable Production app remained running. Production was untouched.
The user already approved repeated agent-launched Development runs and normal UI Exit;
do not ask again for that scope. Never force-kill or close Production.

Real tray-shell hover/rendering, multiple native monitor/DPI combinations, actual
lock/sleep/hibernate/logoff/shutdown and clean-machine validation remain unverified
T-0002 gates. Do not interrupt the user's working machine to test power/session events.

Select T-0019 next, read its card and relevant storage/recovery/report ADRs, define
validation/audit/compatibility decisions before changing the history model. Keep one
primary implementation task active. Publish only through scripts/build.ps1, preserving
canonical EXE paths; locked output must fail without alternate copies. Every shipped
change requires consistent Cargo version, both builds and fresh verified ZIP/PDF.
