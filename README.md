# Attention Focus Management

A local Rust desktop application for tracking time spent focusing: select a project
and optionally a task, start or stop the timer, switch activities, and review how
time was spent. The primary interface is a compact Windows system tray application.

## Status

The Windows desktop preview now records unassigned focus sessions, remembers window
placement, and includes embedded application icons with running/stopped tray states.
Double-click the tray to toggle tracking; use its menu to show the window.
Project/task integration is not implemented. Interactive desktop verification is pending.
See [docs/CONTEXT.md](docs/CONTEXT.md) for current status and next steps.

## Accepted constraints

- Implement the application in Rust.
- Read projects from an existing `MetaPlan/Instance` directory configured in settings.
- Never modify MetaPlan files, statuses, tasks, or Git state.
- Store application settings and session history outside MetaPlan.
- Support project/task selection, start, stop, and focus switching.
- Report session history and time spent on activities.
- Use English only for repository-authored content and application UI.
- Remember window placement and use consistent application/tray icons.
- Toggle tracking by double-clicking the tray icon.

External user data remains unchanged and may contain any language. The language
policy does not require translating or rejecting imported project names.

The selected stack is egui/eframe, tray-icon, and rusqlite with bundled SQLite.
See [the architecture](docs/ARCHITECTURE.md) for module boundaries, timer behavior,
storage safeguards, and validation gates. The UI choice remains subject to validation.
Chat may use the user's language; all created and stored project content stays English.

## Development

1. Read [AGENTS.md](AGENTS.md).
2. Restore context from [docs/CONTEXT.md](docs/CONTEXT.md).
3. Select a task from [tasks/INDEX.md](tasks/INDEX.md).
4. Follow [docs/WORKFLOW.md](docs/WORKFLOW.md).

Requirements: [docs/PRODUCT.md](docs/PRODUCT.md).
Decisions: [docs/decisions/README.md](docs/decisions/README.md).

## Local environment

Build and preview instructions: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).
Run `cargo run --locked` or build with `cargo build --locked --release`.
Only the EXE is required beside the application. Its data is created separately in
`%LOCALAPPDATA%/AttentionFocus`; icons, fonts, and SQLite are compiled in.
Do not commit personal settings, databases, or exports.
