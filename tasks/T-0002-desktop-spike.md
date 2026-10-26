# T-0002: Desktop and Storage Integration Spike

Status: blocked.
Dependencies: T-0001.

## Goal

Deliver desktop and storage integration spike within the initial architecture.

## Scope

Follow [the architecture](../docs/ARCHITECTURE.md) and ADR-0004. Keep all authored
content and UI in English. Preserve external Unicode data and the read-only boundary.
Do not include later tasks or deferred product features.

## Plan

Implement the acceptance criteria in order, then verify and update project context.

## Acceptance criteria

- [x] Initialize a Rust 2024 Cargo package with library and desktop binary; resolve compatible dependencies and commit Cargo.lock.
- [ ] Show an English window with a tray icon and Show/Exit commands; window close hides only if the tray is available.
- [ ] Wake the hidden window from a tray event without relying on continuous UI polling.
- [x] Open bundled SQLite in a temporary directory, round-trip a row, and close it without touching MetaPlan.
- [ ] Run formatting, tests, Clippy, and a Windows release build; document manual hide/show/exit results and limitations.
- [x] Record toolchain, dependency versions, build commands, runtime requirements, and the next task.

## Checks

Implemented the English preview, tray menu/click callbacks, event-driven wakeup,
visible-window close interception, explicit exit, and a worker-based SQLite probe.
UI behavior is implemented but not marked accepted without interactive verification.

- `cargo fmt --check`: passed.
- `cargo test --offline --locked`: passed, one persistence test.
- `cargo clippy --offline --locked --all-targets -- -D warnings`: passed.
- `cargo build --offline --locked --release`: passed on Windows x64 MSVC.
- Rust reported an incremental-cache hard-link fallback to copying during the final
  test/Clippy run. It did not fail the checks; no source lint diagnostics were emitted.
- Windows Computer Use `list_windows` timed out twice, then again after resetting
  and reinitializing the JavaScript helper. No app clicks or visual checks succeeded.
- The release executable has been built but has not been interactively verified.

## Handoff

Blocked on an operational Windows UI automation helper or a human completing the
manual checklist in docs/DEVELOPMENT.md. Verify close-to-hide, tray click restoration,
Show Focus, and Exit both visible and hidden. Record the results before marking done.
Then start T-0003. Do not bypass this integration gate or claim a portable release.
