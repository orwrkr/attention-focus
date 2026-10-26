# Attention Focus Management

A local Rust desktop application for tracking time spent focusing: select a project
and optionally a task, start or stop the timer, switch activities, and review how
time was spent. The primary interface is a compact Windows system tray application.

## Status

The first desktop integration preview is implemented and builds on Windows x64.
It contains a window, tray commands, and a disposable SQLite persistence check.
It is not yet a usable focus tracker. Interactive tray verification is still pending.
See [docs/CONTEXT.md](docs/CONTEXT.md) for current status and next steps.

## Accepted constraints

- Implement the application in Rust.
- Read projects from an existing `MetaPlan/Instance` directory configured in settings.
- Never modify MetaPlan files, statuses, tasks, or Git state.
- Store application settings and session history outside MetaPlan.
- Support project/task selection, start, stop, and focus switching.
- Report session history and time spent on activities.
- Use English only for repository-authored content and application UI.

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
Do not commit personal settings, databases, or exports.
