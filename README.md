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
Run `./scripts/dev.ps1` (or `cargo run --locked`) for Development. Build the distributed EXE with
`./scripts/build.ps1`; it builds both EXEs and automatically creates a verified,
versioned production ZIP under dist. The canonical production EXE is
`dist/AttentionFocus/attention-focus.exe`; the test EXE is `target/debug/attention-focus.exe`.
If the EXE is locked, exit the running app and retry. No alternate copy is created.
Only the EXE is required beside the application. Its data is created separately in
`%LOCALAPPDATA%/AttentionFocus/Production`; development uses the separate `Development`
subdirectory. Both may run together. Debug builds cannot open Production. Window/tray
labels identify Development, and Hide to tray/Exit remain pinned above scrolling content.
Existing history directly under AttentionFocus stays untouched; new Production starts
empty and keeps its history on later launches. Only the old source path is imported.
Icons, fonts, and SQLite are compiled in.
Do not commit personal settings, databases, or exports.

## Guide and distribution

Read the English [user guide](docs/user-guide.md). The build ships its PDF, third-party
notices, and a version/hash manifest with the EXE in AttentionFocus-VERSION-windows-x64.zip.
Application icons are neutral blue (purple for Development); only the tray's green play
or gray pause icon indicates tracking. Development tray icons have a purple D badge.

For PDF build dependencies, run `./scripts/setup-docs.ps1` once. Use
`./scripts/export-user-guide.ps1` to re-export the PDF and automatically refresh the ZIP,
`./scripts/package-production.ps1` to package prepared production outputs, and
`./scripts/verify-release.ps1` to verify the package and both EXEs. Cargo.toml owns the
version; changed production inputs require a version bump and a full build after a
version has been archived. See [development instructions](docs/DEVELOPMENT.md).
