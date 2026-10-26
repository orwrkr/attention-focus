# T-0002: Desktop and Storage Integration Spike

Status: ready.
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

- [ ] Initialize a Rust 2024 Cargo package with library and desktop binary; resolve compatible dependencies and commit Cargo.lock.
- [ ] Show an English window with a tray icon and Show/Exit commands; window close hides only if the tray is available.
- [ ] Wake the hidden window from a tray event without relying on continuous UI polling.
- [ ] Open bundled SQLite in a temporary directory, round-trip a row, and close it without touching MetaPlan.
- [ ] Run formatting, tests, Clippy, and a Windows release build; document manual hide/show/exit results and limitations.
- [ ] Record toolchain, dependency versions, build commands, runtime requirements, and the next task.

## Checks

Not started. Record commands, results, and manual evidence here.

## Handoff

Start after T-0001 is complete. Update this card and the task index together.
