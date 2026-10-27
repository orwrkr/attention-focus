# Current Context

Updated: 2026-10-27.

## State

T-0005 implements project search and worker-owned date-range reports: inclusive dates,
an explicit IANA timezone, project/task/day totals, and 200 recent session details.
Saved, live, and unreviewed time are separate. Jiff 0.2.38 bundles timezone rules.
Clipped duration allocation preserves adjacent-period totals (ADR-0009).
The canonical EXE is published. Implementation and automated checks pass; T-0005 stays
in_progress for full interactive tracking acceptance, which has not been claimed.

T-0004 adds durable project/task/note snapshots, an applied selection restored on
restart, and transactional Start/Stop/Switch. One clock-injected engine owns writes.
Version-one migration backs up old unassigned history using SQLite's backup API.
Windows lock/suspend messages stop at the captured event boundary; resume stays stopped.
Missed heartbeats and clock changes require checkpoint review. See ADR-0008.

T-0008 enables eframe's native clipboard feature. The user confirmed direct path
typing works; Ctrl+V previously used the disabled system-clipboard fallback.
T-0009 establishes scripts/build.ps1 and dist/AttentionFocus/attention-focus.exe as
the only distributed output. Obsolete AttentionFocus-Update was removed. Locked files
are errors; never create fallback copies or terminate an app automatically. If an agent
launched a blocking instance, offer to exit it and wait for user approval.
Source-field Ctrl+C/clear/Ctrl+V was now visually verified without saving settings.

T-0003 now adds source settings and a read-only project/task preview (ADR-0007).
Source identity and selection live in source.sqlite3. Startup reads settings read-only
and validates canonical source/storage boundaries before opening writable app files.
Background loading reports per-project diagnostics and excludes duplicate slugs and links.
The project/task picker uses this snapshot; unavailable or archived activities cannot
start new sessions. Stopping and exit remain available when the source cannot load.

Completed T-0007: single-click window access and a live native hover tooltip.
ADR-0006 supersedes the old ignored-single-click rule. Double click still toggles
tracking without accidentally showing the window; single click waits for the system
double-click interval. Totals include active time once and exclude unreviewed time.

T-0001 is complete. T-0002 remains blocked on interactive verification, but the user
explicitly authorized continuing with desktop features. T-0006 adds remembered window
geometry, embedded icons, two tray states, double-click tracking, and durable unassigned
sessions. A worker owns SQLite and commits state before the UI updates.
Search and reports are implemented; export, corrections, shortcuts, autostart, and
installer support remain deferred.

## Decisions

- Rust with egui/eframe, tray-icon, and rusqlite with bundled SQLite: ADR-0004.
- Configurable MetaPlan/Instance source, strictly read-only: ADR-0001.
- Fixed application storage outside the source; validate resolved paths before writes.
- English-only authored and stored content/UI: ADR-0002.
- Chat may follow the user's language: ADR-0003.
- External Unicode data remains unchanged.
- ADR-0005 authorizes desktop state before source selection and eframe RON geometry
  alongside SQLite. Both use the LocalAppData/AttentionFocus directory.
- Icons are generated from src/icons.rs and embedded at build time with winresource.
- Static CRT linking is configured in .cargo/config.toml for Windows x64 MSVC.
- Architecture and test expectations: docs/ARCHITECTURE.md.
- UI selection rationale and its unmeasured assumptions are saved in ADR-0004.
  The user accepted continuing with egui as a working hypothesis, not a benchmark winner.

## Environment

Verified Rust 1.98.0, Cargo 1.98.0, and stable-x86_64-pc-windows-msvc.
Dependency resolution and linking passed. Cargo.lock records the tested dependency graph:
eframe 0.37.0, tray-icon 0.26.1, rusqlite 0.41.0, tempfile 3.27.0, and winresource 0.1.34.
The distributed executable is dist/AttentionFocus/attention-focus.exe. Cargo's primary
release artifact is consumed by the packaging script; target remains internal build data.
Clean-machine runtime compatibility remains unverified.

## Next step

Verify the checklist in docs/DEVELOPMENT.md with an operational UI helper or human:
geometry restoration, real double-click routing, both tray states, and exit while hidden.
The user exited the agent-launched app after report/clipboard checks. Process inspection
found no app, and the canonical build was republished successfully. An earlier intentional
build while it ran returned the expected locked-file error without fallback copies.
The last UI discovery request was stopped with physical Escape; no further UI input
was sent. Complete T-0005's full manual tracking gate and T-0002's
remaining tray/geometry/lifecycle checks using disposable history, not fake real sessions.

## Verification and limitations

Twenty-seven automated tests pass, including source integrity/junctions, atomic switching,
write-failure rollback, backup migration, immutable activity snapshots, crash recovery,
clock changes, and delayed suspend processing using its original event timestamp.
A real hidden native listener was registered and routed synthetic Windows lock/suspend
messages while ignoring unlock/resume. These are not real machine sleep/lock tests.
Formatting, Clippy, and release build passed; the final results are in T-0004.
The report checks and canonical packaging results are recorded in T-0005/T-0009.
The updated PE dependency audit remains Windows-system-only; wtsapi32 and advapi32
are now included. The canonical single-EXE distribution is current. T-0009 packaging
checks verified locked-file rejection, old-file preservation, and obsolete-copy cleanup.
Formatting, lint, and release build results for the tray refinement are in T-0007.
Tests cover persistence, interrupted recovery, duplicate starts, failed-stop rollback,
worker exit confirmation, and ICO structure. ICO artwork was visually inspected.
Rust emitted only an incremental-cache hard-link fallback warning on the final lint run.
Windows Computer Use list_windows again timed out three times, including after reset.
The current helper can capture the app. Clipboard paste, search filtering, report refresh,
totals sections, details, invalid-date errors, and valid refresh were visually verified.
The background now fills the window. Full tracking actions, window restoration, real tray
events, and Windows shell rendering still need interactive verification.
Release/import-audit results are recorded in T-0006.
T-0006 implementation is complete. Release build and PE import audit passed; the
dist/AttentionFocus folder contains only the EXE. Runtime imports are Windows-only.
No writes were made to MetaPlan. No remote, installer, or published release exists.
