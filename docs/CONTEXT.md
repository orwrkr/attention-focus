# Current Context

Updated: 2026-11-04.

0.18.1 delivers T-0031 (verified tray activation, ADR-0028), T-0032 (report periods that
follow the current day) and T-0033 (intervals split at the local day start by default,
schema three, audited totals, ADR-0029). The first full project review is in T-0034.
106 Rust tests, formatting, Clippy and the package tests passed. Both canonical EXEs and
the ZIP passed verify-release; all nine PDF pages were reviewed, and the shipped guide hash
matches the reviewed preview. An independent read of the diff found no lost time, overlap
or migration defect; its other findings were fixed or recorded as known limits.
The 0.18.0 archive was built first but has two nearly empty PDF pages. It was superseded
before handoff, so distribute 0.18.1 only.

No application was launched and no desktop control was used for this work. Unverified:
real tray foreground behavior, a real midnight crossing, and upgrading a real Production
database. Production 0.17.1 kept running untouched from its own location, and its schema-two
database stays usable by that version until 0.18.1 is run against it.

Exact next step: the user checks tray activation and a midnight crossing in Development
(see T-0031 and T-0033), or grants fresh desktop permission for an agent check. Then decide
the T-0034 owner questions. Real GPU usage remains unmeasured.

## Current state

Current release: 0.18.1. Next implementation card is
[T-0034](../tasks/T-0034-public-release-readiness.md) if publishing proceeds, otherwise
[T-0019](../tasks/T-0019-history-correction.md), history correction/manual intervals.
Neither is implemented. Global shortcuts, installer and
updater remain deferred. T-0002 is a separate real desktop/OS acceptance gate.

The local Rust app uses egui/eframe, tray-icon and bundled SQLite. Cargo.lock pins
versions. MetaPlan is read-only. Source normalization and directory-link resolution
protect every write destination. Never use personal source data as repository fixtures.

Current interval has the prominent blue Start/Stop action. Today for project and
Today - all projects share cached local-calendar-day accounting with the tray. Include
saved plus active time once, all tasks and (for the global sum) all sources/unassigned
focus; exclude review time. Report filters/timezones do not change Today. Errors show
unavailable without blocking successful tracking state. Schema three is current.

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

Running Current interval has a static green frame/tint and Tracking dot/text. A fading
quarter-perimeter trail follows its rounded border clockwise over eight seconds, with
up to twenty frames per second while visible. Border geometry and repaint requests
are gated by clip visibility and minimized/OS-reported occluded state. Tray hiding
minimizes before hiding so the pinned backend skips window GPU rendering, including
autostart. Show restores the window. See ADR-0026. The old status-row marker is removed.
The trail is inset inside fixed card bounds and has no layout or storage effects.
The tray keeps five cached brightness levels within twelve percent, driven by worker
wakeups. See ADR-0025 and T-0028.
Stopped and permanent icons are static. Windows client-area animation preference
disables motion; lookup failure also gives static status. No animation thread, new
database queries or timekeeping changes. Hidden eframe windows run logic without UI
painting. See ADR-0020 and T-0021.
Application settings offer explicit per-user
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
UI persistence checks run every thirty seconds instead of two, plus normal Exit.
Unchanged serialized values do not rewrite window.ron; explicit Stay on top still
flushes immediately. Forced exit can lose recent UI changes without widening the
history checkpoint window. See ADR-0024.

Settings/history live in the Windows LocalAppData known folder under AttentionFocus/
Production or Development, outside MetaPlan. Debug builds reject Production. Profile
locks, histories and geometry are independent. Legacy root history remains untouched.
Schema two dates from 0.3.0 and schema three from 0.18.0; each upgrade backs up first. Older
EXEs cannot read a newer schema. T-0019 must research compatibility before further migrations.

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
Ask for fresh user permission before each new computer-use session, even for
Development. End the session immediately after UI checks; do not leave it idle while
coding or building. Normal Exit of agent-launched Development remains authorized
within an approved session. Never force-kill or close Production.

Real tray-shell hover/rendering, multiple native monitor/DPI combinations, actual
lock/sleep/hibernate/logoff/shutdown and clean-machine validation remain unverified
T-0002 gates. Do not interrupt the user's working machine to test power/session events.

T-0025 adds a fixed Settings button next to Exit; it opens the panel above the
counters and scrolls it into view. Native opening/closing and reveal after scrolling
passed with user permission. Both 0.14.2 EXEs and ZIP were rebuilt and verified;
the nine-page guide was rendered/reviewed. 74 tests, formatting and Clippy passed.
No startup registration or tracking changes were made during the UI check.

T-0026 orders the header buttons Settings, Hide to tray, Exit and removes the animation
diagnostic from settings. Refresh still explicitly rereads Windows registration.
The same status-row layout reserves indicator space while stopped and running, fixing
a reproduced five-point Start/Stop shift. The actual-paint regression now passes in
narrow and wide layouts. All 74 tests, formatting and Clippy passed. Native Development
confirmed toolbar order, simplified settings and stationary start/stop with a short
synthetic interval; normal Exit completed and the computer-use session was reset.
Both 0.14.3 EXEs and ZIP were rebuilt and verified; all nine guide pages were reviewed.

T-0027 adds a visible moving marker with fixed layout and reduces UI save checks to
thirty seconds. 76 tests, formatting and Clippy passed; temporary DB tests measured
zero idle row mutations and six running checkpoint updates per minute. These are not
physical disk write counts. Native Development verified marker movement and stationary
controls, then stopped the synthetic interval and exited normally. The computer-use
session was reset. Both 0.15.1 EXEs and ZIP were verified; nine PDF pages were reviewed.
Native reduced-motion switching and live Production IO profiling remain unexecuted.

T-0028 replaces the status-row marker with a fading rounded-border trail. 76 tests,
formatting and Clippy passed. Both 0.16.0 EXEs and ZIP were built and verified; all nine
guide pages were reviewed. A permitted native check could not proceed: list_windows
timed out three times including one reset/reinitialize attempt. Development was
launched, but no UI input or tracking start was sent; it may remain open. The JS session
was reset and the user was informed. Native visual acceptance is still unverified.
Next immediate step: user review, or a fresh authorized check after helper recovery.

T-0029 gates animation geometry and repaint scheduling on actual clip/viewport
visibility, and makes tray hiding minimize before hiding. 77 tests, formatting and
Clippy passed; headless tests assert zero trail geometry and no animation loop when
invisible. Both 0.16.1 builds and the guide/archive passed release verification;
the shipped PDF matches the reviewed preview. Native
hide/restore behavior and hardware GPU counters remain unmeasured; desktop permission
was requested but no new computer-use session was activated without a reply.
Next immediate check: native hide/restore and scroll visibility with permission.

Before T-0019, read its card and relevant storage/recovery/report ADRs, and define
validation/audit/compatibility decisions before changing the history model. Keep one
primary implementation task active. Publish only through scripts/build.ps1, preserving
canonical EXE paths; locked output must fail without alternate copies. Every shipped
change requires consistent Cargo version, both builds and fresh verified ZIP/PDF.
