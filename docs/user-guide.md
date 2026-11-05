# Attention Focus
## User Guide

Version {{VERSION}} | Windows x64

Attention Focus records time that you choose to spend on a project or task. It stays
in the Windows notification area (system tray) when its window is hidden. It does not
watch your screen or keyboard, score productivity, or use the network.

## Start in three steps

1. Put `attention-focus.exe` in a folder you will keep, and run it.
2. Open **Projects**, type a project name, and click **Add projects**.
3. Choose the project and optionally a task, then click **Start tracking**.

You can also track **Unassigned focus** at any time, without adding a project. A small **i**
beside a label or control explains it: rest the pointer on it, or move to the control with Tab.

## Manage projects and tasks

Open **Projects**. Paste several names at once, one per line, then click **Add projects**.
Names can use any language; names already in the list are skipped. Expand a project to:

- **Rename** it. Earlier sessions keep the name they recorded, and totals show the new one.
- Add tasks, or tick a task when it is finished. Adding its name again reopens it.
- **Archive** it, so it leaves the pickers. Its history stays in reports. Use **Show archived
  projects** to find it and **Restore** it.
- **Delete** a project or task that was never tracked. Anything with recorded time can only
  be archived or finished, so history always knows what it belongs to.

## Recognize the icons

The file, window, taskbar, Alt-Tab, and Task Manager use a neutral logo, which never means
that tracking is running. The tray status icon and the **Tracking / Stopped** label show state.

<!-- icon-legend -->

Development tray icons have a purple **D** badge, and a purple logo identifies the debug
build. Hover over a tray icon to confirm its profile and status before using it.

<!-- pagebreak -->

# Record and switch focus

## Choose an activity

- Open the Project dropdown and type in its **Search projects** field to filter by
  project name. The search stays inside the open list.
- Search ignores case, filters projects
  rather than tasks/history, and does not change the current tracked activity.
- Select a project and optionally one of its unfinished tasks.
- Choose the colored, bold italic **Unassigned focus (no project)** entry when no
  project is appropriate. Its position still follows recent-use ordering.
- Add an optional note to describe the session. Each project remembers its last applied
  note and eight distinct recent nonblank notes. Typing filters its suggestions; choosing
  one fills the draft. Project changes restore that project's note; task changes keep it.
  Unassigned focus has its own notes. A blank applied note clears the default but keeps
  suggestions. Notes are profile-local preferences, remembered from this version onward.

Archived projects and finished tasks cannot start a new interval. An interval already
running finishes normally. Choose another activity if the window reports that a selection
is unavailable.

## Change projects and tasks

- **Start tracking** starts with the activity currently shown in the picker.
- Choosing a project or task while stopped saves the next activity, including for tray
  starts. It does not start tracking.
- While tracking, choosing another project or task immediately saves the old interval
  and begins a new one at the same boundary. This is the default behavior.
- Disable **Start a new interval when project or task changes** in **Settings** to correct
  the entire current interval instead. All its elapsed time moves to the new project/task;
  its original start stays unchanged. Saved earlier intervals are never changed. A line above
  the pickers reminds you while this mode is on. The option is remembered separately for
  Production and Development.
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
Order survives restart. Production and Development keep independent usage.

<!-- pagebreak -->

# Tray controls and interruptions

**Current interval** in the window counts only the running interval and returns to
zero when stopped. Its large blue **Start tracking / Stop tracking** button is the
primary action, with distinct hover and pressed states. **Today for project** combines the named project's tasks;
**Today - all projects** includes every project and unassigned focus. Both use the local
day and include live time once. **All projects, all time (saved)** counts finalized
history only. The window and tray share both daily values.

## Use the tray and window controls

- A single left click on the tray icon shows the window and brings it to the front.
  Opening waits briefly for Windows to distinguish a single click from a double click.
- A double left click starts or stops tracking using the last applied selection.
- Hover shows status, project, **Session**, **Today/project** and **Today/all**.
  Today combines all its tasks and saved plus current time inside the Windows local
  calendar day, excluding interrupted time. Report filters do not affect this value.
  All-project Today also includes unassigned focus.
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
Scroll bars appear only on overflow, beside the content and never over it.
Expanded/collapsed sections, scroll positions, project and timezone searches, report filters and unapplied note drafts are restored on restart.
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

# Settings

Click **Settings**, the first button in the fixed top toolbar. Application settings open above the
counters and scroll into view. Click **Settings** again to close the panel.

## Choose a theme

**Theme** offers **Same as Windows**, **Light** and **Dark**. **Same as Windows** is the default
and switches by itself when Windows changes between its light and dark app modes. The title bar
follows the choice, which is remembered separately for Production and Development.

## Choose a language

**Language** offers **Same as Windows** and each supported language: English, Russian, German,
French, Spanish, Portuguese (Brazil), Italian, Polish, Turkish, Chinese (Simplified), Japanese,
Korean and Arabic. **Same as Windows** uses your first Windows display language that the
application supports, or else English. Translations are machine-made, and corrections are welcome.
This guide and the contents of exports stay in English.

Chinese, Japanese, Korean and Arabic use fonts included with Windows, as do project names in those
scripts. In Arabic, text reads from right to left and aligns to the right, while checkboxes, section
headings, date fields, scroll bars and the chart time axis stay where they are, and text fields show
typed words in the order you type them.

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

<!-- pagebreak -->

# Read history and reports

Open **Focus history and reports** and choose a period:

1. Type **From** and **Until** dates as `YYYY-MM-DD`, or use either **Calendar** button.
   Enter times as `HH:MM`: From is included, Until is excluded. `24:00` for Until means
   the end of that date; defaults `00:00` to `24:00` select whole days.
2. Choose **Time zone** from the dropdown; search for a city/region or choose UTC.
3. Results load automatically. **Refresh report** is available for an immediate refresh.

## Follow the current day

**Today**, **Last 7 days** and **Last 30 days** follow the current day. The highlighted period
moves to the new date at midnight in the selected time zone, even while the app stays open.
**Custom** keeps the dates as entered. Editing a date or time switches to Custom, including
when you pick today's date by hand, so choose **Today** again to follow the current day. The
line under the buttons states which mode is active.

A report can cover at most 3,660 calendar days.
Invalid dates, reversed ranges, and unknown time zones produce an error; correct the
fields to retry. An earlier valid result keeps its own period and snapshot label.

## Understand the totals

- **Total (saved + running)** adds finalized and active time in the selected period,
  excluding interrupted time. Project, task and day tables end with **Total saved**.
- **Saved focus** includes finalized sessions in the selected period.
- **Running now** is the active interval's elapsed time inside this filter. It remains
  separate from Saved focus until the interval stops or switches.
- **Interrupted, not counted yet** appears only when there is such time inside the filter:
  the part of a cut-off interval saved up to its last checkpoint. **Accept saved time** in
  the main window moves it into Saved focus.

Zero means no matching time, which can happen while tracking outside the filter; values
under one second also display as zero. The **i** beside either label explains it.
These categories are never combined into a misleading finalized total.

Visible expanded reports poll every two seconds only while active time can grow inside
the filter. Historical/future ranges and stopped tracking pause periodic queries. A
boundary exit gets one final snapshot; relevant stop/edit/recovery events refresh once
to reclassify time. Unrelated tracking changes do not reload historical results. Opening,
changing filters, or manual Refresh still loads a result. Failures retain the last valid
snapshot. Hidden/collapsed reports pause queries; typed filters remain debounced/editable.

Expand **By project**, **By project and task**, or **By day** for breakdowns. Totals show
each project's current name. Notes do not create duplicate totals.

## Inspect sessions

Expand **Sessions**, then a session to see its activity, optional note, start/end times,
full recorded duration, and why it ended. The time in the list is clipped to the chosen
period. Start/end labels show the selected time zone's UTC offset, including DST changes.

Details show up to 200 recent matching sessions. Totals include every matching session,
including older ones not shown in that list. Renamed or archived projects do not erase
existing history; sessions keep the names they recorded.

<!-- pagebreak -->

# See activity in charts

**Charts** sit above the tables and follow the selected period and time zone. Choose a view:

- **Timeline** shows when you worked. Each day is a row on a 24-hour axis, with one block per
  session and the day's total. Time crossing midnight appears on both days.
- **By day** shows how much you worked. Each day is a column divided by project. A single
  day becomes one bar with each project's share.
- **Weekly rhythm** shows when you usually work: time per weekday and hour, where a stronger
  color means more time.

Solid marks are finished time. A lighter mark with an outline is the running interval. An
outline alone is interrupted time, which stays out of day totals and Weekly rhythm,
just as it stays out of Total. Hover over a mark for its details; the tables hold the same
numbers, so nothing depends on the pointer.

Each project keeps an automatic color across periods. With more than eight active projects,
two can share a color: the chart gives it to the larger, and the smaller joins a neutral
**Other**, while hover still names it. Timeline and By day cover up to 92 days and say so
for longer periods; Weekly rhythm has no such limit. A chart never changes recorded time.

# Export CSV and Markdown

Open **Focus history and reports**, set the period/timezone, then choose **Export CSV**
or **Export Markdown**. Choose a new filename in the Windows Save dialog. Canceling
creates no export; existing files are never overwritten. Export outside the application's
data folders.

## What is exported

The export is a snapshot of every matching session, including entries beyond the
window's 200-row preview. It includes version, snapshot time, timezone, exact filter
boundaries, integer millisecond durations, project IDs, activity names and notes.
Saved, Running checkpoint and Needs review (interrupted time) totals remain separate. The combined total
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
software. UTF-8 text and quoted multiline cells preserve Unicode.

Markdown includes totals by project, project/task and day, plus all session details.
Formatting characters in names and notes are escaped. A failed export shows an error,
keeps tracking usable and does not replace an existing file.

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

## Intervals at midnight

Tracking that continues past midnight ends the running interval at the start of the new
local day and continues as a new interval with the same activity. No time is lost and
totals do not change. The finished part appears in history with the end reason **New day
started** (`day_boundary` in exports). **Current interval** and the tray **Session** value
restart from zero, and the finished part moves into **All projects, all time (saved)**.

Disable **Start a new interval at midnight** in **Settings** to keep one interval across
days; reports and Today still divide its time by day. This option is remembered per profile.
If you enable it during a long interval, that interval is divided at every midnight it has
already passed. The Windows time zone decides when midnight occurs. Editing the current
interval changes only the part after the last split.

## Disk activity and saving

History checkpoints are written about every ten seconds while tracking (about six
updates per minute); timer repainting does not write history. UI preferences are checked
for saving every thirty seconds and on normal Exit; unchanged settings skip the file
write. A forced exit may lose the most recent UI changes. Start/stop and activity changes
are saved immediately.

<!-- pagebreak -->

# Data, profiles, and updates

## Production and Development are separate

Use the packaged EXE for normal **Production** use. For development, run
`scripts/dev.ps1` from a source checkout, or launch `target/debug/attention-focus.exe`.
Debug builds always default to Development and reject selecting Production.

Both profiles can run together. Each permits one instance and has its own history,
project list, applied activity, window placement, and backups. Development is
identified in the header and has a distinct window title and tray badge. Building
development does not replace a running production EXE.

The fixed data directories are:

- Production: `%LOCALAPPDATA%/AttentionFocus/Production`
- Development: `%LOCALAPPDATA%/AttentionFocus/Development`

Each contains `focus.sqlite3` for history and projects, and `window.ron` for placement and
Stay on top. SQLite may create journal files in the same folder. Moving the EXE does not
move or reset this data. Development history is never copied into Production automatically.

## What the application needs

`attention-focus.exe` is the whole application. Icons, fonts, SQLite, the C runtime and the
license texts are inside it, so a copy of the file carries them. To read them, click **About**,
then **Licenses**. No Rust, Python, .NET, Node, or WebView2 installation is needed. Windows system components and a
working graphics driver are required. Clean-machine compatibility is still being verified.

## Protect and back up your data

To back up history and settings, exit the relevant profile first and copy that entire
profile's data directory to a safe location. Do not copy only an open SQLite database
while it may have a journal. Schema upgrades make a local backup before migrating old
history. These backups complement, rather than replace, your own backups.

Version 0.18 upgrades history to schema three, with a `focus-before-v3-*.sqlite3` backup.
Version 0.19 upgrades it to schema four, with a `focus-before-v4-*.sqlite3` backup. It lists
every project you have tracked, with its tasks; add any others yourself. Earlier EXEs cannot
open upgraded history. To return to one, restore its backup while the app is closed; sessions
recorded after the upgrade are not in it.

## Install an update

1. Exit Production through its window or tray menu. Close an older development build
   too if you intend to replace that development EXE.
2. Replace `attention-focus.exe` with the new file. Keeping the same folder keeps sign-in startup.
3. Start the EXE. Your production data remains in the fixed data directory.

The app version appears in the window header, Windows file properties, this guide and its
filename. Each release's `release.json` records the version and SHA-256 hashes.
No asset files or third-party runtime DLLs need to be copied beside the EXE.

<!-- pagebreak -->

# Troubleshooting and current limits

## The app is already running

Launching the EXE again shows the window of the copy already running. An app hidden in the
tray is still running; choose Exit from its tray menu to end it. Production and Development
are independent, so one of each is allowed. The tray icon and a second launch raise the
window above ordinary windows, but Windows can keep keyboard focus in a program running as
administrator.

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
