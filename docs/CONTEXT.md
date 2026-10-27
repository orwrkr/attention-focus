# Current Context

Updated: 2026-10-27.

## Current state

Current release: 0.7.1. T-0017 UI restoration/daily tray and T-0005 are complete.
T-0002 remains a separate native desktop acceptance gate. Next implementation card:
T-0016, CSV/Markdown export. It is planned, not implemented. History editing, shortcuts,
autostart, installer and updater are still deferred. Read task cards and ADRs for history.

The Rust app uses egui/eframe, tray-icon and bundled rusqlite. Dependencies are pinned
in Cargo.lock. App/window/file icons are neutral; only filled-green/hollow-gray tray
states indicate running/stopped, with a D badge in Development. Tray single click shows
and double click toggles. Project/task changes split intervals by default; EditCurrent
preserves the active interval start/timer. Pickers use durable recent-use order.

Reports accept half-open minute boundaries and bundled timezone choices. Until 24:00
means end of day. Periodic queries run only while live time grows inside the filter;
affected stop/edit/recovery events refresh once. Saved, Running and Needs review remain
separate. Calendar/DST allocation is covered by tests. See ADR-0014.

Windows owns resizing/maximization; there are no per-frame native size commands.
Only content width is capped. This supersedes ADR-0015 clamps, which caused jitter.
Sections/scroll, report custom filters or rolling presets, searches and note draft
persist per profile in window.ron. Native restart verified restoration without tracking.
Today in the tray is the selected project's local-day saved plus active total across
tasks, isolated by source ID/path/slug, excluding review time. Saved sums are cached;
idle refresh is every 30 seconds. Unavailable daily data does not block state updates.
See ADR-0016. Native maximization and downward sizing via Windows Size menu passed.

## Lifecycle and data boundaries

Window close hides if the tray is available; tracking continues. Exit commits before
closing. Received lock/suspend notifications finalize at their captured boundary;
unlock/resume never restart. Confirmed WM_ENDSESSION also saves, with a bounded worker
confirmation; canceled shutdown queries do not stop tracking. Force-kill, power loss,
missed heartbeats or clock discontinuities use checkpoint review. Checkpoints run about
every ten seconds, not a hard loss guarantee under IO failure. See ADR-0008/0015.

MetaPlan is strictly read-only and its Instance path is configurable. Resolve paths
and directory links before writes. Application storage is outside the source in the
Windows LocalAppData known folder under AttentionFocus/Production or Development.
Debug builds reject Production. Histories, settings, mutexes and geometry are separate.
Legacy root history stays untouched. Schema remains two since 0.3.0; upgrading schema
one backs it up first; EXEs before 0.3.0 cannot read schema two. See ADR-0010/0012.

All agent-created sessions are synthetic and Development-only. The ignored fixture is
under target/manual-instance. Development was exited normally after checks; the external
portable Production app stayed open and untouched. The user explicitly approved repeated
agent-launched Development test runs and normal UI Exit for verification. Do not ask
again for that scope; this does not authorize closing Production or force-killing apps.

## Checks and limits

T-0017: 54 Rust tests, formatting and Clippy passed; both 0.7.1 EXEs and automatic ZIP
passed verification. All seven guide pages inspected. Native restoration and stable
maximization/downward sizing passed; native tray and multi-monitor gates remain.

52 Rust tests, five package tests, formatting and Clippy passed. Junction tests initially
hit restricted permissions; the permitted rerun passed all tests. Native listener tests
used synthetic messages, not actual machine shutdown. Both 0.6.1 EXEs and automatic ZIP
passed verify-release metadata/icons/fingerprint/stamp/member/hash checks. All seven
PDF pages were rendered and visually inspected. An initial 0.6.0 archive is retained;
0.6.1 adds the explicit shrink correction found during native testing.

Native checks covered synthetic source settings/search/calendars (T-0014), task split,
EditCurrent timing, recency, saved task totals and UTC selection. Exit during tracking
saved the interval; restart was stopped, without unfinished/overlapping rows in read-only
Development inspection. Maximize/collapse, pinned actions and placement/preference
restoration passed. Minimum-width drag did not establish a resize, so remains unverified.
Actual topmost Z-order, tray input/rendering, multiple monitors, real lock/sleep/hibernate/
logoff/shutdown and clean-machine compatibility are unverified. The native helper exposes
app windows but did not expose a targetable taskbar/tray window. Do not disrupt the
user's live environment for real power/session tests.

Pre-commit follow-up: the earlier final Computer Use observation was interrupted by
physical Escape. Current read-only process inspection confirms Development has exited
and only the external Production process remains. No GUI input was needed. Repeated
52 Rust tests, five package tests, formatting, Clippy and release verification passed.

## Next step

Complete the remaining T-0002 checklist with suitable native access or human verification
on a safe test environment. Proceed with T-0016 export as the next product change; research
file selection and record its ADR first. Keep one primary implementation task selected.
Publish changes only through scripts/build.ps1, which builds both canonical EXEs and
regenerates the PDF/notices/ZIP; run verify-release.ps1 before handoff. Canonical paths:
dist/AttentionFocus/attention-focus.exe and target/debug/attention-focus.exe. A locked
EXE is an error, never a reason to create alternate copies. Cargo.toml owns the version;
changed shipped inputs of an archived version require a bump and complete rebuild.

Use ATTENTION_FOCUS_PYTHON with the bundled Python runtime when the default Python
lacks ReportLab/Pillow/pypdf. Poppler renders previews under ignored target/pdf-preview.
All stored project content stays English; chat can use the user's language. Never commit
personal source data, profile history, machine-specific paths or generated artifacts.
