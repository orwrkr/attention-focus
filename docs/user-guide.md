# Attention Focus
## User Guide

Version {{VERSION}} | Windows x64 | English interface

Attention Focus records time that you choose to spend on a project or task. It stays
in the Windows notification area (system tray) when its window is hidden. It does not
watch your screen or keyboard, score productivity, or edit your MetaPlan files.

## Start in three steps

1. Extract the production ZIP to a folder and run `attention-focus.exe`.
2. Open **MetaPlan source and projects**, type/paste the `Instance` path or use
   **Browse...**, then click **Save and refresh**. Choosing a folder only fills the
   field; cancel leaves it unchanged. Source/storage validation runs when saving.
3. Choose a project and optional task, then click **Start tracking**.

The folder must contain `PROJECTS/Active`. Data in `Archive` can be previewed but is
not offered for new focus sessions. Valid projects still load if another project has
a missing or malformed Markdown file; read the diagnostic shown in the source section.

## Recognize the icons

The file, window, taskbar, Alt-Tab, and Task Manager use a neutral application logo.
That logo identifies the application; it never means that tracking is running.
The tray status icon and the window's **Tracking / Stopped** label indicate state.

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

- Open the Project dropdown and type in its **Search projects** field to filter by
  project title or directory slug. The search stays inside the open list.
- Search ignores case, filters projects
  rather than tasks/history, and does not change the current tracked activity.
- Select a project and optionally an unfinished task from its current cycle.
- Choose the colored, bold italic **Unassigned focus (no project)** entry when no
  project is appropriate. Its position still follows recent-use ordering.
- Add an optional note to describe the session. Each project remembers its last applied
  note and eight distinct recent nonblank notes. Typing filters its suggestions; choosing
  one fills the draft. Project changes restore that project's note; task changes keep it.
  Unassigned focus has its own notes. A blank applied note clears the default but keeps
  suggestions. Notes are profile-local preferences, remembered from this version onward.

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

**Current interval** in the window counts only the running interval and returns to
zero when stopped. Its large blue **Start tracking / Stop tracking** button is the
primary action, with distinct hover and pressed states. **Today for project** combines the named project's tasks;
**Today - all projects** includes every project and unassigned focus. Both use the local
day and include live time once. **All projects, all time (saved)** counts finalized
history only. The window and tray share both daily values.

## Use the tray and window controls

- A single left click on the tray icon shows the window. Opening waits briefly for
  Windows to distinguish a single click from a double click.
- A double left click starts or stops tracking using the last applied selection.
- Hover shows status, project, **Session**, **Today/project** and **Today/all**.
  Today combines all its tasks and saved plus current time inside the Windows local
  calendar day, excluding unreviewed time. Report filters do not affect this value.
  All-project Today also includes unassigned focus and saved history from other sources.
  Long names are shortened; the project's window label shows its full name in one hint.
  While stopped, project Today refers to the saved next project.
- A right click opens the menu with Show Focus, Start/Stop, and Exit.
- **Hide to tray** hides the window. Closing the window normally hides it too.
- **Exit** saves the active session and ends the app. Use Exit before replacing its EXE.

Hide to tray and Exit stay pinned above scrolling content, even with reports expanded.
Hiding a window does not stop tracking. If tray setup fails, hiding is disabled so that
the app cannot silently become inaccessible.

Enable **Stay on top** in the header to keep the window above ordinary windows.
Disable it for normal window order. This is remembered separately for each profile,
together with window placement, and does not prevent hiding to the tray. The normal
header shows the app/version; the test version also shows Development. Secondary
explanations use a blue accent to distinguish them from primary text.

Drag the app heading or empty header space above scrolling content to move the window.
New windows open centered; ordinary saved positions are restored. Inaccessible or legacy
top-left positions are recentered. A constrained maximized window is centered horizontally.
Resize or maximize the window normally. Width stops at a readable limit; vertical
resizing remains independent. A visible edge marks the constrained maximized window.
The scrollbar appears only on overflow; reserved margins keep content width stable
as sections open or close. Long reports scroll, and header
actions remain accessible. Expanded/collapsed sections, scroll positions, project and
timezone searches, report filters and unapplied note drafts are restored on restart.
Today/Last 7/Last 30 presets roll forward; custom dates and minutes stay as entered.
Preferences are saved per profile. Restoring the UI never restarts tracking.

## Interruptions and returning to work

Windows lock, sleep and hibernation notifications stop the session at the received
event boundary. Unlocking or waking does not restart it: start explicitly when ready.
Confirmed Windows logoff, shutdown or restart saves the active interval before the
notification returns. A canceled shutdown query does not stop tracking. Saving has a
bounded wait; a forced termination or failed write can still require recovery.

If the app closes unexpectedly or detects a clock change or missed heartbeat, it asks
you to review saved checkpoint time. **Accept saved time** retains the last durable
checkpoint; time after it is excluded. Restarting never counts the time the app was off.
Checkpoints are written about every ten seconds. Force-killing the process or losing
power can lose the unsaved tail. Reading or thinking without input does not stop time.

<!-- pagebreak -->

# Startup and animation settings

Click **Settings**, the first button in the fixed top toolbar. Application settings open above the
counters and scroll into view. Click **Settings** again to close the panel.

## Start with Windows

- Enable **Start at Windows sign-in** to launch this EXE for your Windows account.
  Startup is off by default and does not need administrator rights.
- Enable **Start in tray when launched at sign-in** to keep that startup window hidden.
  Click its tray icon to show it. If the tray cannot be created, the window remains visible.
- Manual launches show the window, regardless of the saved tray preference.
- Tracking starts **Stopped**. Sign-in startup never resumes a focus interval.

Settings are separate for Production and Development. Enable the Production copy you
intend to use every day. Enabling Development registers only its separate test copy.

The startup command points to the EXE's current location. If you move it, run the new
copy, open this section and choose **Use this EXE for startup**. The app does not silently
change registration during launch. **Refresh startup status** reads the current entry.
Disable startup before removing the portable application. Windows can delay sign-in
programs; a startup item disabled in Windows Startup settings must be re-enabled there.

A very long EXE path can exceed Windows' startup command limit. The app reports an error;
move it to a shorter path and retry. Other applications' startup entries are unchanged.

## Understand the tracking animation

When Windows allows client-area animations, a green gradient trail travels clockwise
around the **Current interval** frame, completing one lap every eight seconds. Its
tail fades smoothly into the static frame. The tray light changes brightness by at
most twelve percent.
When Windows animation effects are disabled or unavailable, the running accent stays
static. Stopped tracking never animates. The app does not change Windows motion settings.
The border stops requesting animation frames when the timer card is outside the visible
scroll area or the window is minimized. Hiding to tray also suspends window rendering;
tracking continues. Animation returns when the window and card become visible again.

## Disk activity and saving

History checkpoints are written about every ten seconds while tracking (about six
updates per minute); timer repainting does not write history. UI preferences are checked
for saving every thirty seconds and on normal Exit; unchanged settings skip the file
write. A forced exit may lose the most recent UI changes. Start/stop and activity changes
are saved immediately.

<!-- pagebreak -->

# Read history and reports

Open **Focus history and reports** and choose a period:

1. Type **From** and **Until** dates as `YYYY-MM-DD`, or use either **Calendar** button.
   Enter times as `HH:MM`: From is included, Until is excluded. `24:00` for Until means
   the end of that date; defaults `00:00` to `24:00` select whole days.
2. Choose **Time zone** from the dropdown; search for a city/region or choose UTC.
3. Results load automatically. Today, Last 7 days, and Last 30 days set quick ranges;
   **Refresh report** is also available for an immediate refresh.

Presets use the time zone you selected. A report can cover at most 3,660 calendar days.
Invalid dates, reversed ranges, and unknown time zones produce an error; correct the
fields to retry. An earlier valid result keeps its own period and snapshot label.

## Understand the totals

- **Total (saved + running)** adds finalized and active time in the selected period,
  excluding Needs review. Project, task and day tables end with **Total saved**.
- **Saved focus** includes finalized sessions in the selected period.
- **Running (not finalized)** is the active interval's elapsed time inside this filter.
  It remains separate from Saved focus until the interval stops or switches.
- **Needs review (excluded)** is interrupted checkpoint time inside the filter, awaiting
  your recovery decision. Accept saved time moves it into Saved focus, excluding downtime.

Zero means no matching time, which can happen while tracking outside the filter; values
under one second also display as zero. Hover on either label for its explanation.
These categories are never combined into a misleading finalized total.

Visible expanded reports poll every two seconds only while active time can grow inside
the filter. Historical/future ranges and stopped tracking pause periodic queries. A
boundary exit gets one final snapshot; relevant stop/edit/recovery events refresh once
to reclassify time. Unrelated tracking changes do not reload historical results. Opening,
changing filters, or manual Refresh still loads a result. Failures retain the last valid
snapshot. Hidden/collapsed reports pause queries; typed filters remain debounced/editable.

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

<!-- pagebreak -->

# Export CSV and Markdown

Open **Focus history and reports**, set the period/timezone, then choose **Export CSV**
or **Export Markdown**. Choose a new filename in the Windows Save dialog. Canceling
creates no export; existing files are never overwritten. Export outside MetaPlan and
the application's data folders. Source files do not need to be currently available.

## What is exported

The export is a snapshot of every matching session, including entries beyond the
window's 200-row preview. It includes version, snapshot time, timezone, exact filter
boundaries, integer millisecond durations, source identities, activity names and notes.
Saved, Running checkpoint and Needs review totals remain separate. The combined total
adds Saved and Running checkpoint, excluding Needs review. Markdown grouping tables
end with Total saved; CSV adds Total saved plus running checkpoint ms.

Running time uses the last durable checkpoint, so it can lag the live window timer.
Stop tracking before export when you need a finalized current interval. Preparing
the snapshot and writing the file happen in the background; tracking has its own worker.

## Read the formats

CSV has a header, one **Summary** row and **Session** rows. Metadata and report totals
repeat on each row. Sum **Clipped ms** of Saved Session rows for the finalized total;
do not sum the repeated total columns. Formula-like text is prefixed with an apostrophe
for text import. This guard is not guaranteed after editing/re-saving in spreadsheet
software. UTF-8 text and quoted multiline cells preserve imported Unicode.

Markdown includes totals by project, project/task and day, plus all session details.
Imported formatting is escaped. A failed export shows an error, keeps tracking usable
and does not replace an existing file.

<!-- pagebreak -->

# Minute boundaries and time accounting

## Choose precise intervals

For example, From `2026-10-27 09:15` to Until `2026-10-27 10:30` includes exactly that
75-minute window in the chosen timezone. The **Time** button beside each time field
opens an hour/minute picker with Apply and Cancel. Until also offers End of day (24:00). Time at or after 10:30 is excluded. Adjacent
windows therefore do not count the same time twice. To include the entire last date,
use Until `24:00`; Until `00:00` means the start of that date.

Non-midnight times repeated or skipped by a daylight-saving change are rejected rather
than guessed. Choose another boundary or use UTC to specify the exact occurrence.
Midnight retains the first valid start-of-day behavior, including historical date skips.
This filtering never edits saved sessions or changes the tracking timer.

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
selected source, applied activity, window placement, and backups. Development is
identified in the header and has a distinct window title and tray badge. Building
development does not replace a running production EXE.

The fixed data directories are:

- Production: `%LOCALAPPDATA%/AttentionFocus/Production`
- Development: `%LOCALAPPDATA%/AttentionFocus/Development`

Each contains `focus.sqlite3` for history, `source.sqlite3` for source settings, and
`window.ron` for placement and Stay on top. SQLite may create journal files in the same folder.
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

## Tracking looks static

While tracking, **Current interval** has a green frame, a light tint and a green status
dot. A gradient trail travels around the frame over eight seconds. The filled green tray light varies
slightly in brightness on the same slow cycle, retaining its shape and Development badge.
Stopped tracking has no motion. Windows **Animation effects** preferences are respected;
with animations disabled, the green running accents stay static. Daily counters and
the main window boundary do not animate. Motion never changes the recorded time.

## What is not included yet

Manual time correction, global shortcuts,
an installer, and an automatic in-app updater are not implemented. Real Windows tray,
sleep/lock, multi-monitor geometry, and clean-machine compatibility still have manual
acceptance checks outstanding. Unit tests and simulated notifications do not prove
those operating-system behaviors on every computer.

When reporting a problem, include the app version, Production/Development profile,
what you clicked, and the visible error. Share a screenshot or logs only after checking
that they do not expose private project names, notes, paths, or history.
