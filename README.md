# Attention Focus Management

A local Rust desktop application for tracking time spent focusing: select a project
and optionally a task, start or stop the timer, switch activities, and review how
time was spent. The primary interface is a compact Windows system tray application.

## Status

The repository provides an agent development workflow. Application code does not
exist yet. See [docs/CONTEXT.md](docs/CONTEXT.md) for current status and next steps.

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

The UI toolkit, storage format, and interruption behavior remain undecided.
Choosing Rust does not automatically mean choosing Tauri.

## Development

1. Read [AGENTS.md](AGENTS.md).
2. Restore context from [docs/CONTEXT.md](docs/CONTEXT.md).
3. Select a task from [tasks/INDEX.md](tasks/INDEX.md).
4. Follow [docs/WORKFLOW.md](docs/WORKFLOW.md).

Requirements: [docs/PRODUCT.md](docs/PRODUCT.md).
Decisions: [docs/decisions/README.md](docs/decisions/README.md).

## Local environment

Git, rustc, and Cargo were checked during initialization. The Cargo project and
build commands will be added after architecture selection. Do not commit personal
settings, databases, or exports.
