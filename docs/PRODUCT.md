# Product Scope

## Confirmed by the user

- A desktop focus-switching tool that stays in the system tray.
- Project and task selection, with timer start and stop.
- Reports showing how much focus time was spent on each activity.
- Rust implementation.
- Project data from `MetaPlan/Instance`, with the path configured in settings.
- No modifications to the source.
- English-only project-authored content and application UI.
- Remember window placement between launches.
- Use a shared application logo for the executable, window, taskbar, and tray.
- Distinguish running/stopped tracking in the tray; double click toggles tracking.
- Single-click the tray to show the window; hover to see session and total tracked time.
- Keep Hide to tray and Exit visible above scrolling content.
- Separate production/development data and allow simultaneous independent instances.
- Start new production with empty history while retaining the previous development history.
- Permanent icons must be neutral or state-correct; never imply running while stopped.
- Main builds must also produce the development EXE for testing.
- Provide an English user guide and a PDF export script.
- Automatically refresh a production ZIP with the EXE, guide PDF, and required notices.
- Maintain consistent version metadata and appropriate versioned distribution filenames.

Windows is the initial target based on the current environment and tray scenario.
Other operating systems are not currently required.

## Proposed first working slice

1. Configure and validate the source directory.
2. Read names, statuses, and focus priorities from `project.md`.
3. Select a project and optional task, with a free-form session description.
4. Start, stop, and switch, ending the previous session exactly when the next begins.
5. Persist history locally outside the source.
6. Show history and total time by project for a selected period.
7. Provide tray controls and an explicit application exit action.

This is an implementation proposal, not a claim that these features already exist.

## MetaPlan format observations

- Projects live in `PROJECTS/Active/<slug>/` and `PROJECTS/Archive/<slug>/`.
- Being in `Active/` does not guarantee an `Active` status; read `project.md`.
- The human-readable name is in the heading of `project.md`; the directory name is the slug.
- Statuses are `Active`, `Paused`, `Completed`, and `Suspended`.
- A source focus field distinguishes primary and background projects. It describes
  project priority, not timer state. Starting a session must not change it.
- Source labels can be non-English. English descriptions here explain their meaning,
  not literal parser tokens; verify the actual format when implementing the reader.
- Tasks in `tasks.md` use Markdown checkboxes. Empty items, completed cycles, and
  references to external workspaces are possible.
- Do not automatically follow external filesystem paths mentioned in task text.
- Stable task IDs are not guaranteed. Matching edited tasks needs a separate decision;
  session history must preserve the original task title.
- Archiving a project must not delete its accumulated history.
- Imported names and user-authored notes retain their language and Unicode content;
  application-owned controls, messages, and report headings are English.

## Implementation decisions

The initial architecture selects egui/eframe, tray-icon, and bundled SQLite; see
[ADR-0004](decisions/0004-desktop-stack.md) and [the architecture](ARCHITECTURE.md).
These are implementation choices, not claims of completed functionality.

## Deferred features and remaining validation

- Validate tray wakeup, packaging requirements, and Windows lifecycle behavior.
- Complete real Windows validation of the implemented storage, backup, and recovery design.
- Session correction/manual entry, CSV/Markdown export, shortcuts, and autostart.
- Project identity when slugs change or the source directory is replaced.
- Complete interactive validation of implemented time zones, midnight boundaries, and clock changes.

## Accounting principles

Only one session is active at a time; switching must not double-count time.
Time represents user-declared attention, not automatically measured concentration.
Project dependency relationships must not inflate the total by counting time twice.
The source must remain independently usable without the application.
