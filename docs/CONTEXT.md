# Current Context

Updated: 2026-10-27.

## State

Completed T-0007: single-click window access and a live native hover tooltip.
ADR-0006 supersedes the old ignored-single-click rule. Double click still toggles
tracking without accidentally showing the window; single click waits for the system
double-click interval. Totals include active time once and exclude unreviewed time.

T-0001 is complete. T-0002 remains blocked on interactive verification, but the user
explicitly authorized continuing with desktop features. T-0006 adds remembered window
geometry, embedded icons, two tray states, double-click tracking, and durable unassigned
sessions. A worker owns SQLite and commits state before the UI updates.
MetaPlan source settings, task attribution, full lifecycle handling, and reports remain
unimplemented. This is a bootstrap timer, not the finished project-aware tracker.

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
The Windows release executable is target/release/attention-focus.exe.
Clean-machine runtime compatibility remains unverified.

## Next step

Verify the checklist in docs/DEVELOPMENT.md with an operational UI helper or human:
geometry restoration, real double-click routing, both tray states, and exit while hidden.
Record the evidence against T-0002. Next product task is T-0003 (source configuration
and read-only parsing), including canonical-path checks against all application data.

## Verification and limitations

Passed cargo test --offline --locked (ten tests), including single/double-click routing.
Formatting, lint, and release build results for the tray refinement are in T-0007.
Tests cover persistence, interrupted recovery, duplicate starts, failed-stop rollback,
worker exit confirmation, and ICO structure. ICO artwork was visually inspected.
Rust emitted only an incremental-cache hard-link fallback warning on the final lint run.
Windows Computer Use list_windows again timed out three times, including after reset.
Window restoration, real tray events, and Windows shell rendering remain unverified.
Release/import-audit results are recorded in T-0006.
T-0006 implementation is complete. Release build and PE import audit passed; the
dist/AttentionFocus folder contains only the EXE. Runtime imports are Windows-only.
No writes were made to MetaPlan. No remote, installer, or published release exists.
