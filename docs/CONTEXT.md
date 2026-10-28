# Current Context

Updated: 2026-10-28.

## Current state

Current release: 0.12.1. T-0018, T-0020 and T-0021 are complete; next implementation card is
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

Counter frames use measured margins and symmetric reserved scroll margins. Narrow
windows place the interval above two daily cards; wide windows use three columns.
Only egui's automatic elided-name tooltip is enabled. Native width is limited to 760
logical pixels. The maximized-bounds subclass accounts for current monitor work area
and DPI-scaled resize borders, including once at restored startup. Native non-client
painting and a client outline make the constrained maximized boundary explicit.
No measured UI height or size/maximize commands run per frame. See ADR-0018/0019.

Running Current interval has a green frame/tint and explicit Tracking dot/text. An
eight-second cosine gently varies its accent at up to twenty frames per second while
visible. The tray uses five cached green brightness levels within twelve percent,
updated by existing worker wakeups; shape, alpha and Development badge stay fixed.
Stopped and permanent icons are static. Windows client-area animation preference
disables motion; lookup failure also gives static status. No animation thread, new
database queries or timekeeping changes. Hidden eframe windows run logic without UI
painting. See ADR-0020 and T-0021.

Scrollbars appear on overflow with a visible dormant handle. Symmetric 12-point margins
keep content width constant, opaque framebuffer clearing prevents stale background,
and content-size changes request another layout/repaint. Start/Stop uses scoped native
egui widget visuals for distinct idle/hover/pressed states. Search is inside the project
popup, which stays open for editing; Unassigned is distinguished without losing recency.
Browse uses an STA folder picker with New Folder hidden. It only fills the draft;
Save and refresh performs the existing source/storage validation. Cancel keeps the path.

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

69 Rust tests, formatting, Clippy and five package tests passed for T-0021. Windows
junction checks ran with the required link permissions.
Tests cover responsive margins at 100-200 percent scale, mirrored scrollbar gutters,
a single name hint, native monitor/frame bounds and project/global daily accounting.
Native T-0018 verified live counters, stopped reset, previous-day report independence,
restored maximized startup and narrow/wide layouts. T-0020 verified overflow-only bars,
clean source/report collapse, visible maximized boundary, popup filtering without
selection changes, distinguished Unassigned selection and folder confirm/save/cancel.
Actual-paint tests verify button hover/pressed/disabled states and repeated reflow.
Earlier downward sizing passed without snapback. T-0021 native Development 0.12.0
verified running/stopped accents and advancing interval/daily counters without moving
cards. Its short synthetic Unassigned interval belongs only to Development. Normal
Exit completed and process lookup confirmed only external Production remained running.
Native tray-shell animation and Windows animation-preference changes were not exercised.
Release 0.12.1 package/PDF checks are recorded in the T-0021 card.

All agent-created sessions are synthetic and Development-only under the ignored
manual fixture. No input was sent to Production, which remained untouched.
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
