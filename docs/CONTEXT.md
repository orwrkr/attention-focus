# Current Context

Updated: 2026-10-29.

## Current state

Current release: 0.14.0, T-0024 sign-in startup/tray launch. Next implementation card is
[T-0019](../tasks/T-0019-history-correction.md), history correction/manual intervals.
That card is defined but not implemented. Global shortcuts, installer and
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
Startup now computes maximized bounds directly instead of sending a synthetic zeroed
MINMAXINFO query before winit constraints exist. Missing native tracking limits fall
back to an explicit width cap. Each undersized normal axis recovers to its initial
default once, preserving valid axes, position and all other preferences. See ADR-0021.
T-0023 adds native HTCAPTION regions for the app heading and free header space, excluding
the pinned controls and resize borders. Rectangles are updated on the UI thread and
released on window destruction. First-run, collapsed, inaccessible and legacy top-left
placement is centered in the work area; ordinary saved placement remains. Constrained
maximization is centered horizontally. See ADR-0022.

Running Current interval has a green frame/tint and explicit Tracking dot/text. An
eight-second cosine gently varies its accent at up to twenty frames per second while
visible. The tray uses five cached green brightness levels within twelve percent,
updated by existing worker wakeups; shape, alpha and Development badge stay fixed.
Stopped and permanent icons are static. Windows client-area animation preference
disables motion; lookup failure also gives static status. No animation thread, new
database queries or timekeeping changes. Hidden eframe windows run logic without UI
painting. See ADR-0020 and T-0021.
Application settings now report the motion preference and offer explicit per-user
Windows Run registration, with independent Production/Development value names. No
registration is written on ordinary launch. The optional tray preference is included
only in the marked autostart command; manual launch remains visible and never starts
tracking. Tray failure keeps the window visible. The native subclass suppresses the
first show once and logic reconciles winit's cached visibility before later Show/errors.
Duplicate autostart exits quietly. See ADR-0023 and T-0024.

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

74 Rust tests, formatting and Clippy passed for T-0024. Windows
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
T-0022 Development 0.12.2 recovered a seeded 64-by-64 saved size to 620-by-640 at
150-percent scale. Maximize/Exit/relaunch retained the full constrained maximized size;
Restore returned to the recovered normal size. Other preferences remained intact and
no session was started. Development exited normally. Production remained untouched.
T-0023 tests cover native header/control hit testing, shared-state cleanup, work-area
centering and negative monitor origins. First-run centering was observed in Development.
Final agent capture was stopped with physical Escape; the user subsequently reported
checking the rebuilt Development app and approved the release/commit. Native acceptance
is user-reported; do not claim an agent replay of every drag/restore/restart scenario.
Release 0.13.0 package checks are recorded in the T-0023 card.
T-0024 verified isolated HKCU fixture round trips, profile/value isolation, quoted Unicode
commands, length/flag validation, hidden-first-show and later native Show. Development
UI confirmed settings, Windows motion enabled, tray-preference persistence and visible
manual launch. A duplicate marked autostart exited quietly with code zero. Test tray
preference was reset and Development exited normally. No real Run registration was
enabled and no tracking session was started. Real sign-in and full shell tray-start
acceptance remain unexecuted; never log off the user's session for these checks.
Both 0.14.0 EXEs and the automatically rebuilt ZIP passed verify-release metadata,
neutral icons, source fingerprint, allowlist and hashes. The shipped nine-page PDF
matched the fully rendered/reviewed preview. See the completed T-0024 card.

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
