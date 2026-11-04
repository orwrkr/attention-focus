# Architecture

A local Rust desktop application: egui/eframe for the window, tray-icon for the notification
area, and bundled SQLite through rusqlite for history. Decisions and their reasons are in
[docs/decisions](decisions/README.md). The stack was chosen in
[ADR-0004](decisions/0004-desktop-stack.md).

## Modules

| Module | Responsibility | Boundary |
| --- | --- | --- |
| `tracking` | The timer, history, project list and the worker that owns the database | No UI; clocks are injected |
| `tracking::engine` | Applies commands; detects clock and heartbeat problems; splits days | Takes wall and monotonic time as input |
| `tracking::store` | Session transactions, selection, preferences, report queries | Only touches the validated database |
| `tracking::catalog` | The application's project and task list | Finished history is never rewritten |
| `tracking::migrate` | Explicit schema versions, backups and conversions | Older formats appear only here |
| `tracking::worker` | One thread that serializes every command | The UI talks to it through messages |
| `reports` | Calendar boundaries, clipping, totals, chart data | Pure; counts each interval once |
| `charts` | Draws the chart views and assigns project colors | Paints computed data; holds no totals of its own |
| `export` | CSV and Markdown output with protected destinations | Reads a separate snapshot |
| `i18n`, `bidi`, `fonts` | Interface languages, right-to-left text, Windows fonts on demand | Tables are read-only; the language is process-wide |
| `paths`, `profiles` | Storage identity, link checks, Production and Development | Fixed locations only |
| `platform`, `lifecycle`, `startup` | Windows APIs: windows, instance lock, session events, sign-in startup | Windows code stays here |
| `main`, `counters`, `project_ui`, `report_ui`, `note_memory`, `motion`, `ui_support` | Window, tray, interface and shared style | Change state only through commands |

Use a fake clock and temporary storage in tests. OS events become commands, so timing
rules can be tested without a desktop.

## Projects and tasks

The history database owns the list; see [ADR-0031](decisions/0031-application-projects.md). A
project has a permanent numeric ID. Each session stores that ID with the title and task text
as recorded, so renaming, archiving or finishing never rewrites finished history. Report
totals show current names and session rows show recorded ones. Anything with recorded
time can be archived or finished but not deleted.

## Storage boundary

Each profile has a fixed directory under the Windows per-user LocalAppData known folder:
`AttentionFocus/Production` or `AttentionFocus/Development`. It holds `focus.sqlite3`,
`window.ron` and any schema backups. Debug builds cannot select Production. There is no
setting for another location. Linked profile directories, and linked entries inside
them, are rejected before anything writable opens. Older files directly under
`AttentionFocus`, and `source.sqlite3` in a profile, remain unread and unchanged.

Exports go through the native Save dialog, may not overwrite a file or land inside application
storage after resolving links, and are checked again before the finished file is published.
Malicious replacement of directory links while the application runs is outside the threat
model; see [SECURITY.md](../SECURITY.md).

## Persistence model

- `focus_sessions`: start, end, last checkpoint, elapsed milliseconds, state (`running`,
  `stopped`, `interrupted`), end reason and an activity snapshot. A unique partial index
  allows only one running row.
- `focus_selection`: the saved next activity. `focus_preferences`: tracking options.
- `focus_recent`: recent-use order, updated with the history it belongs to.
- `focus_projects`, `focus_tasks`: the list. `focus_project_aliases`: identities from before
  schema four, kept only to convert older note preferences.

Schema versions are explicit, in `PRAGMA user_version`, and currently four. An upgrade first
copies the database with the SQLite backup API to `focus-before-vN-*.sqlite3`, then applies
its changes in one transaction. Newer history is refused rather than guessed at. On a storage
failure, the last committed state stays, an error appears and the action can be retried.
Interface preferences live in `window.ron`, separate from history.

## Timer semantics

| State / command | Result |
| --- | --- |
| Idle / Start | Commit a new session, then show Running |
| Running / Stop | Close the session; retain selection for a later Start |
| Running / Switch | Close old and open new at one boundary in one transaction |
| Idle / project or task picker change | Save the next activity; remain stopped |
| Running / picker change, default mode | Atomically switch at one boundary |
| Running / picker change, EditCurrent mode | Reassign the entire active interval; preserve ID, start, timer and finalized history |
| Running / Start | Reject duplicate start; never create a second active row |
| Running / local day start, default | Close old and open new at one boundary in one transaction; same activity; elapsed time divided |
| Running / local day start, split disabled | Continue the same interval; reports clip it by day |
| Running / lock or suspend | Close with an interruption reason; retain selection |
| Running / confirmed Windows session end | Save at the event boundary with a bounded confirmation wait |
| Any / canceled shutdown query | Keep tracking state unchanged |
| Interrupted / resume or unlock | Remain stopped until the user explicitly starts |
| Running / explicit Exit | Stop durably, then exit; stay open on save failure |
| Any / window close | Hide only when the tray is available; never stop implicitly |
| Startup / unfinished session | Enter recovery review before allowing another Start |
| Second manual launch | Ask the running instance to show its window, then exit |

There is no idle auto-stop, because reading and thinking may involve no input, and no automatic
restart after unlock. See ADR-0008, ADR-0012, ADR-0015, ADR-0029 and ADR-0030.

Monotonic time decides elapsed duration, and UTC places it on the timeline. Checkpoints are
written about every ten seconds while running. A heartbeat gap over 30 seconds or a clock
divergence over five seconds ends the interval at its last checkpoint for review. During a
smaller backward correction, wall time continues by monotonic time until the interval ends,
so intervals never overlap and always span their elapsed time.

The UI sends one command at a time and queues later ones. Each gets one answer, Committed or
Rejected, while interruptions, reports and ticks report separately. The worker publishes a snapshot once per second while tracking and every
30 seconds otherwise, reusing stored state until a command, day split or interruption changes it.
Showing the window raises it natively and confirms the foreground; see ADR-0028.

## Reporting

Report ranges are half-open minute ranges resolved in a selected time zone with bundled rules,
so a day is not assumed to last 24 hours. Recorded elapsed time is allocated over each session's
wall-clock span with cumulative rounding, which makes adjacent ranges add up exactly. Saved,
Running and Needs review stay separate, and Needs review is never counted as finished.

Visible expanded reports poll only while active time grows inside the filter, with one query
in flight. Edits are debounced, superseded replies are rejected and the last good result stays
on error. Rolling periods move to the new date before that gating; Custom stays fixed. See
ADR-0009, ADR-0013, ADR-0014 and ADR-0017. Exports read an independent read-only snapshot.

Chart data is computed next to the report from the same records and rules, so charts cannot
disagree with the tables, and project colors are interface preferences. See ADR-0032.

## Languages

`locales/en.json` is the source text and the fallback; other files there translate it. Text is
composed in logical order, and painted for egui at the last moment, which only matters in Arabic:
see `src/i18n.rs` and [ADR-0035](decisions/0035-interface-languages.md). Windows draws the tray menu
and tooltip, so those stay logical. Exports never go through `i18n`, so their contents do not depend
on the interface language.

## Validation

Automated tests cover timing with injected clocks, upgrades from each older schema, totals
across calendar boundaries, and interface behavior through headless pointer events. They do
not prove real tray behavior, power events, or multi-monitor geometry. Those remain manual
checks; see [DEVELOPMENT.md](DEVELOPMENT.md).
