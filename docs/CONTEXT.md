# Current Context

Updated: 2026-10-27.

## State

T-0013 implements version 0.4.0: visible expanded reports refresh every two seconds
and on committed tracking changes, with a single in-flight request, editable/debounced
filters, stale-parameter reply rejection, and last-good-result retention. Date fields
have Jiff calendar popups; timezone selection searches bundled names. Stay on top uses
native WindowLevel and immediate per-profile window.ron persistence. Tracking schema
remains two, as in 0.3; upgrades from 0.2 still migrate/backup schema one on app startup.
Tooltips name the active project within 127 UTF-16 units. Production headers omit the
profile word; Development stays explicit. Secondary text uses theme-aware blue.
No new dependencies were added. See ADR-0013 and T-0013.

T-0012 implements version 0.3.0: project/task picker changes apply immediately. While
running they create an atomic new interval by default; the per-profile checkbox can
instead reassign the entire active interval without resetting its start/timer. While
stopped they save the next activity without starting. The upper caption explicitly
identifies committed tracking activity or the saved next selection. Notes alone remain
draft until applied or included in a picker change. Projects/tasks sort by durable
tracking-use order with deterministic alphabetical ties, scoped by source and profile.
Schema two adds preferences/recency and backs up schema-one databases before one-time
history backfill. See ADR-0012. Development and Production histories remain separate.

T-0011 introduces version 0.2.0, neutral blue application icons and purple development
identity; only tray states are green play/gray pause, with a D badge in Development.
The common renderer feeds Windows resources, runtime window icons, tray icons, and the
five-page English guide. See ADR-0011 and docs/user-guide.md.
scripts/build.ps1 now builds both EXEs and automatically publishes/verifies a versioned
ZIP with the EXE, guide PDF, third-party notices, and release.json hashes. Exporting the
PDF also refreshes the ZIP. Cargo owns the version; archived versions reject changed
inputs. The build stamp rejects stale EXEs. Run scripts/verify-release.ps1 before handoff.
Install documentation tools with setup-docs.ps1 or provide ATTENTION_FOCUS_PYTHON.

T-0010 pins Hide to tray/Exit above scrolling content and isolates Production and
Development under LocalAppData/AttentionFocus. The release defaults to Production;
debug and scripts/dev.ps1 use Development and reject explicit Production. Profiles
have separate history, source settings, geometry, mutexes, and app/window identities.
Old root files remain untouched. Only a valid old source path is seeded; new production
history starts empty and persists normally after that. See ADR-0010.

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
the only canonical production EXE. T-0011 adds the versioned package. Obsolete
AttentionFocus-Update was removed. Locked files
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
- Validate source containment against the shared application root, not just one profile.
- Development is the only profile for agent experiments; never reset or seed Production.
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

Use dist/AttentionFocus-VERSION-windows-x64.zip for production and
target/debug/attention-focus.exe for Development. No app was launched during T-0013.
Windows discovery timed out twice; initialization also timed out after reset. Verify
calendar/timezone controls, automatic reporting, topmost restore, tooltip project and
theme contrast, then immediate
picker changes, edit-mode timing, recent ordering, and the
pinned buttons, two simultaneous visible profiles, and remaining T-0005/T-0002 tracking,
tray, geometry, and lifecycle checks in Development. At the last read-only inspection,
a portable 0.2.0 Production copy outside the repository was running against schema one.
Do not equate free canonical build outputs with no app running, and do not stop or
replace the external copy automatically. Never create fake Production time.

## Verification and limitations

T-0013: 45 Rust tests, five package tests, formatting, and Clippy pass. Both 0.4.0 EXEs
and automatic ZIP built; verify-release passed versions/icons/fingerprint/stamp/members/
hashes. Six PDF pages inspected and byte-matched to final output. New checks cover
refresh scheduling/staleness/backoff, leap/month grids, bundled timezone resolution,
and Unicode tooltip state/length. Native UI/topmost/persistence/theme checks remain
unverified: two list_windows timeouts, then initialization timeout after reset. No app
was launched, real database changed, or running portable copy replaced/stopped.

T-0012: 39 Rust tests, five package tests, formatting, and Clippy pass. Both 0.3.0 EXEs
and the automatically refreshed ZIP built and verify-release passed (versions, eight
icon sizes each, exact members, build stamp, source fingerprint, and SHA-256 hashes).
All six PDF pages visually inspected; the final PDF is byte-identical to that preview.
New tests verify interval boundaries, current-interval edits, persisted/source-scoped
recency and actual sorting, write rollback, backup/backfill and failed-migration rollback,
plus recovery without auto-resume. Windows discovery timed out three times including
reset. No app was launched or real profile/source data changed by development tools.

T-0011: both 0.2.0 EXEs and the production ZIP built; 32 Rust tests, five Python tests,
formatting, Clippy, output-lock checks, and PowerShell parsing pass. Verified file version
metadata and all eight embedded neutral icon sizes per EXE. Inspected every one of five
PDF pages and proved the final packaged PDF byte-identical. ZIP allowlist/hashes/stamp
and automatic refresh through both build and PDF export pass. Both PE imports are
Windows-system-only. No app was launched; real shell rendering remains unverified.

Thirty-two automated tests pass, including profile isolation/defaults/legacy preservation,
native simultaneous profile mutexes, junction rejection, source integrity, atomic switching,
write-failure rollback, backup migration, immutable activity snapshots, crash recovery,
clock changes, and delayed suspend processing using its original event timestamp.
A real hidden native listener was registered and routed synthetic Windows lock/suspend
messages while ignoring unlock/resume. These are not real machine sleep/lock tests.
Formatting, Clippy, and release build passed; the final results are in T-0004.
The report checks and canonical packaging results are recorded in T-0005/T-0009.
The updated PE dependency audit remains Windows-system-only; wtsapi32 and advapi32
are now included. The canonical EXE remains the only production executable. T-0009 packaging
checks verified locked-file rejection, old-file preservation, and obsolete-copy cleanup.
Formatting, lint, and release build results for the tray refinement are in T-0007.
Tests cover persistence, interrupted recovery, duplicate starts, failed-stop rollback,
worker exit confirmation, and ICO structure. ICO artwork was visually inspected.
Rust emitted only an incremental-cache hard-link fallback warning on the final lint run.
Windows Computer Use list_windows again timed out three times, including after reset.
Earlier helper captures verified clipboard paste, search filtering, report refresh,
totals sections, details, invalid-date errors, and valid refresh were visually verified.
The latest helper cannot enumerate windows (three timeouts including reset). The pinned
header/two-app GUI changes are not visually verified. Full tracking actions, window restoration, real tray
events, and Windows shell rendering still need interactive verification.
Release/import-audit results are recorded in T-0006.
T-0006 implementation is complete. Release build and PE import audit passed. T-0011
expands dist/AttentionFocus with the guide, notices, and manifest. Runtime imports are Windows-only.
No writes were made to MetaPlan. No remote, installer, or published release exists.
