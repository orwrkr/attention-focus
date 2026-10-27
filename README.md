# Attention Focus Management

A local Rust desktop application for tracking time spent focusing: select a project
and optionally a task, start or stop the timer, switch activities, and review how
time was spent. The primary interface is a compact Windows system tray application.

## Status

The Windows desktop preview now records project/task focus sessions, remembers window
placement, and includes embedded application icons with running/stopped tray states.
Click the tray to show the window; double-click to toggle tracking. Hover to see
current session and total tracked time.
Configure an Instance folder under MetaPlan source and projects, then choose a project
and optional task. Start tracking or use Switch focus while running. The previous
selection is retained for tray starts. Text fields support the system clipboard.
Search projects in the picker. Focus history and reports shows inclusive date ranges
in a chosen time zone, saved totals by project/task/day, and up to 200 recent sessions.
Running time and unreviewed checkpoints are shown separately from saved totals.
Some interactive desktop checks remain pending.
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
Run `cargo run --locked` for development. Build the distributed EXE with
`./scripts/build.ps1`; its only output is `dist/AttentionFocus/attention-focus.exe`.
If the EXE is locked, exit the running app and retry. No alternate copy is created.
Only the EXE is required beside the application. Its data is created separately in
`%LOCALAPPDATA%/AttentionFocus`; icons, fonts, and SQLite are compiled in.
Do not commit personal settings, databases, or exports.
