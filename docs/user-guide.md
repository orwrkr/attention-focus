# Attention Focus
## User Guide

Version {{VERSION}} | Windows x64 | English interface

Attention Focus records time that you choose to spend on a project or task. It stays
in the Windows notification area (system tray) when its window is hidden. It does not
watch your screen or keyboard, score productivity, or edit your MetaPlan files.

## Start in three steps

1. Extract the production ZIP to a folder and run `attention-focus.exe`.
2. Open **MetaPlan source and projects**, enter the full path to your `Instance`
   folder, and click **Save and refresh**. Ctrl+V is supported.
3. Choose a project and optional task, then click **Start tracking**.

The folder must contain `PROJECTS/Active`. Data in `Archive` can be previewed but is
not offered for new focus sessions. Valid projects still load if another project has
a missing or malformed Markdown file; read the diagnostic shown in the source section.

## Recognize the icons

The file, window, taskbar, Alt-Tab, and Task Manager use a neutral application logo.
That logo identifies the application; it never means that tracking is running.
Only the tray status icon and the window's **Tracking / Stopped** label indicate state.

<!-- icon-legend -->

Development tray icons have a purple **D** badge. Hover over an icon to confirm its
profile and status before using it. A purple application logo identifies the debug
development build and is also independent of tracking state.

## What to keep in the extracted folder

The package contains the EXE, this PDF, third-party notices, and a release manifest.
Only the EXE is needed to run. Icons, fonts, SQLite, and the C runtime are embedded.
Keep the notices when redistributing the package. No Rust, Python, .NET, Node, or
WebView2 installation is needed to run the app. Windows system components and a
working graphics driver are required. Clean-machine compatibility is still being verified.

<!-- pagebreak -->

# Record and switch focus

## Choose an activity

- Type in **Search projects** to filter by project title or directory slug.
- Select a project and optionally an unfinished task from its current cycle.
- Choose **Unassigned focus** when no project is appropriate.
- Add an optional note to describe the session.

The selected project must be available in the loaded source. Completed or archived
projects and unavailable tasks cannot start a new session. Refresh MetaPlan or choose
another activity if the window reports that a selection is unavailable.

## Change projects and tasks

- **Start tracking** starts with the activity currently shown in the picker.
- Choosing a project or task while stopped saves the next activity, including for tray
  starts. It does not start tracking.
- While tracking, choosing another project or task immediately saves the old interval
  and begins a new one at the same boundary. This is the default behavior.
- Disable **Start a new interval when project or task changes** to correct the entire
  current interval instead. All its elapsed time moves to the new project/task; its
  original start stays unchanged. Saved earlier intervals are never changed. The option
  is remembered separately for Production and Development.
- Choosing the already selected project keeps its task and does not create an interval.
- **Stop tracking** saves elapsed time and leaves the saved activity selected.

The heading **Tracking now (saved activity)** shows what the current timer belongs to.
When stopped, **Next tracking activity (saved selection)** shows the next tray-start
choice. These update after saving succeeds. During a save the controls briefly pause;
on failure the confirmed heading stays unchanged and a retry button is available.

Notes alone stay draft. Apply them with **Start new interval with changes** or
**Save changes to current interval**, according to the option above; while stopped use
**Apply selection**. A subsequent project/task change also includes the draft note.

## Find recent choices

Both project and task lists put the most recently tracked choice first, including
Unassigned focus and No task. Unused choices follow alphabetically. Simply opening,
searching, or choosing a next activity while stopped does not count as tracking use.
Order survives restart and is seeded from existing history after updating. Separate
sources and Production/Development keep independent usage.

<!-- pagebreak -->

# Tray controls and interruptions

## Use the tray and window controls

- A single left click on the tray icon shows the window. Opening waits briefly for
  Windows to distinguish a single click from a double click.
- A double left click starts or stops tracking using the last applied selection.
- Hover shows status, current session time, and saved plus active total time.
- A right click opens the menu with Show Focus, Start/Stop, and Exit.
- **Hide to tray** hides the window. Closing the window normally hides it too.
- **Exit** saves the active session and ends the app. Use Exit before replacing its EXE.

Hide to tray and Exit stay pinned above scrolling content, even with reports expanded.
Hiding a window does not stop tracking. If tray setup fails, hiding is disabled so that
the app cannot silently become inaccessible.

## Interruptions and returning to work

Windows lock and sleep notifications stop the session. Unlocking or waking does not
restart it: start explicitly when ready. The timer does not stop merely because you
are reading or thinking without keyboard or mouse input.

If the app closes unexpectedly or detects a clock change or missed heartbeat, it asks
you to review saved checkpoint time. **Accept saved time** retains the last durable
checkpoint; time after it is excluded. Restarting never counts the time the app was off.

<!-- pagebreak -->

# Read history and reports

Open **Focus history and reports** and choose a period:

1. Enter **From** and **Through** dates as `YYYY-MM-DD`. Both dates are included.
2. Enter an IANA **Time zone**, such as `Europe/Istanbul`, `America/New_York`, or `UTC`.
3. Click **Refresh report**, or choose Today, Last 7 days, or Last 30 days.

Presets use the time zone you selected. A report can cover at most 3,660 calendar days.
Invalid dates, reversed ranges, and unknown time zones produce an error; correct the
fields and refresh again. An earlier result keeps its own period and snapshot label.

## Understand the totals

- **Saved focus** includes finalized sessions in the selected period.
- **Running (not finalized)** shows the active session separately.
- **Needs review (excluded)** shows saved interrupted time awaiting your decision.

These three categories are not combined into a misleading finalized total. Reports
are snapshots: refresh to include activity recorded since the displayed snapshot time.

Expand **By project**, **By project and task**, or **By day** for breakdowns. Sources
with the same project slug are kept separate; hover on a project total to see its source.
Project relationships never add the same interval again. Notes do not create duplicate totals.

## Inspect sessions

Expand **Sessions**, then a session to see its activity, optional note, start/end times,
full recorded duration, and why it ended. The time in the list is clipped to the chosen
period. Start/end labels show the selected time zone's UTC offset, including DST changes.

Details show up to 200 recent matching sessions. Totals include every matching session,
including older ones not shown in that list. Renamed, deleted, or archived projects do
not erase existing history; sessions keep snapshots of the titles they recorded.

## How calendar time is counted

Reports split at local midnight using bundled time-zone rules. A daylight-saving day
can be shorter or longer than 24 hours. Recorded elapsed time remains authoritative.
When clipping a session across a boundary, elapsed time is allocated proportionally
over its wall-clock interval. Adjacent periods still add to the full recorded duration.
This allocation cannot reconstruct unrecorded details of a session. Large clock changes
require checkpoint review instead of silently inventing continuity.

Displayed times use whole seconds; underlying totals retain milliseconds. Independently
displayed rows can therefore differ from the displayed total by rounding. Invalid saved
intervals are excluded with a diagnostic rather than silently counted.

<!-- pagebreak -->

# Data, profiles, and updates

## Production and Development are separate

Use the packaged EXE for normal **Production** use. For development, run
`scripts/dev.ps1` from a source checkout, or launch `target/debug/attention-focus.exe`.
Debug builds always default to Development and reject selecting Production.

Both profiles can run together. Each permits one instance and has its own history,
selected source, applied activity, window placement, and backups. The header identifies
the profile; development also has a distinct window title and tray badge. Building
development does not replace a running production EXE.

The fixed data directories are:

- Production: `%LOCALAPPDATA%/AttentionFocus/Production`
- Development: `%LOCALAPPDATA%/AttentionFocus/Development`

Each contains `focus.sqlite3` for history, `source.sqlite3` for source settings, and
`window.ron` for window placement. SQLite may create journal files in the same folder.
Moving the EXE does not move or reset this data.

Older files directly under `%LOCALAPPDATA%/AttentionFocus` remain unchanged. The new
Production profile starts with empty history; only a valid old MetaPlan path is imported.
Later launches keep the new production history normally. Development history is never
copied into Production automatically.

## Protect and back up your data

MetaPlan is a read-only source. The app never changes project statuses, priorities,
tasks, or source files. Application data cannot be stored inside the configured source,
nor can the source contain application data. Linked source files/projects are skipped.

To back up history and settings, exit the relevant profile first and copy that entire
profile's data directory to a safe location. Do not copy only an open SQLite database
while it may have a journal. Schema upgrades make a local backup before migrating old
history. These backups complement, rather than replace, your own backups.

Version 0.3 upgrades history to schema two and creates a `focus-before-v2-*.sqlite3`
backup for schema-one databases. Earlier EXEs cannot open the upgraded history.

## Install an update

1. Exit Production through its window or tray menu. Close an older development build
   too if you intend to replace that development EXE.
2. Extract the new versioned production ZIP and use its `attention-focus.exe`.
3. Start the EXE. Your production data remains in the fixed data directory.

The app version appears in the window header, Windows file properties, this guide,
and the ZIP/PDF filenames. `release.json` records the package version and SHA-256 hashes.
No asset files or third-party runtime DLLs need to be copied beside the EXE.

<!-- pagebreak -->

# Troubleshooting and current limits

## The app is already running

Use that profile's tray icon to show its window, or choose Exit there before launching
another copy of the same profile. Production and Development are independent, so one
of each is allowed. An app hidden in the tray is still running.

## The source does not load

Check that the selected folder is `Instance`, not its parent, and contains
`PROJECTS/Active`. Check the per-project diagnostics. Missing task files, duplicate
slugs across Active/Archive, linked entries, or malformed metadata can exclude entries.
Imported names retain their original Unicode text. Filesystem paths written inside
task descriptions are never opened automatically.

A missing source does not prevent stopping a session or reading existing history.
Choose an available activity or Unassigned focus before starting a new session.

## A save, clock, or recovery error appears

Do not assume a successful start/stop/switch until the window confirms it. Check free
disk space and permissions on the profile's data directory. Preserve its files before
attempting repairs. Accept recovery only after reviewing the checkpoint explanation.
If the system clock is earlier than the latest saved end, correct the clock before
starting another session. The app does not rewrite past times to hide clock changes.

## The EXE is busy during a build or update

Exit the running app through Exit; merely closing its window can leave it in the tray.
The build returns an error and does not create an alternate production EXE. Production
is published only to `dist/AttentionFocus/attention-focus.exe`. Development builds use
the separate internal `target/debug` output. Agents do not terminate apps automatically.

## What is not included yet

Manual time correction, export to CSV/Markdown, global shortcuts, automatic startup,
an installer, and an automatic in-app updater are not implemented. Real Windows tray,
sleep/lock, multi-monitor geometry, and clean-machine compatibility still have manual
acceptance checks outstanding. Unit tests and simulated notifications do not prove
those operating-system behaviors on every computer.

When reporting a problem, include the app version, Production/Development profile,
what you clicked, and the visible error. Share a screenshot or logs only after checking
that they do not expose private project names, notes, paths, or history.
