# Product Scope

## Confirmed by the owner

- A desktop focus-switching tool that stays in the system tray.
- Project and task selection, with timer start and stop.
- A project and task list kept in the application, without any external source.
- Reports showing how much focus time was spent on each activity.
- Rust implementation, under the MIT License.
- English-only project-authored content and application UI.
- Remember window placement between launches.
- Use a shared application logo for the executable, window, taskbar, and tray.
- Distinguish running/stopped tracking in the tray; double click toggles tracking.
- Single-click the tray to show the window and bring it to the front; hover to see session
  and total tracked time.
- Keep Hide to tray and Exit visible above scrolling content.
- Separate production/development data and allow simultaneous independent instances.
- Permanent icons must be neutral or state-correct; never imply running while stopped.
- Main builds must also produce the development EXE for testing.
- Provide an English user guide and a PDF export script.
- Automatically refresh a production ZIP with the EXE, guide PDF, and required notices.
- Maintain consistent version metadata and appropriate versioned distribution filenames.
- Clearly label the confirmed activity above the draft controls.
- Project/task changes start a new interval by default, with a persistent opt-out to
  edit the entire current interval without resetting its start.
- A tracked interval crossing local midnight ends with the old day and continues as a new
  interval at the same boundary, by default, with a persistent opt-out.
- Sort project/task choices by last tracking use, newest first.
- Automatically refresh open reports; offer calendar date selection and timezone lists.
  Report periods can follow the current day, and the active mode is visible.
- Offer Stay on top, include the active project in tray tooltips, improve secondary-text
  distinction, and omit Production from the normal header.
- Use status lights without action-like play/pause symbols. Allow minute boundaries
  and limit periodic refresh to live time growing inside the report filter.
- Fit window size to useful content and save tracking on confirmed Windows session end.
- Restore sections, filters, searches and note drafts between launches without restarting tracking.
- Show distinct current-interval and project-day window counters, using the tray's
  daily accounting. Limit width to useful content without dynamic height changes.
- Show all-project Today in both window and tray, including unassigned focus; make
  Start/Stop the visually prominent action. Keep frame margins and name hints consistent.
- Keep scrolling correct after section reflow and expose a visible maximized boundary.
  Give Start/Stop hover/pressed feedback, search within the project dropdown and
  distinguish Unassigned focus.
- Make running tracking clear with calm window/tray motion that does not distract.
  Preserve unambiguous stopped status and neutral permanent application icons.
- Allow dragging from the app header above scrolling content and center default placement.
- Offer Windows sign-in startup and optional tray-only startup for that invocation.
- Combined saved/running report total, saved table footers and matching export totals.
- Per-project last confirmed note and eight distinct recent nonblank suggestions.
- Minute picker dialogs alongside manual time entry, preserving range validation.
- No border geometry/repaint loop for clipped, hidden or OS-reported invisible areas.
- Prepare the project for a public GitHub repository.

Windows is the only target. Other operating systems are not currently required.

## Under consideration

- Graphical views of activity over a day, several days or a week. See T-0036.

## Deferred

- Correcting recorded sessions and adding missed intervals by hand (T-0019).
- Global shortcuts, an installer, and an automatic updater.
- Renaming tasks with their history.
- Real Windows validation of sleep, multi-monitor geometry and clean machines (T-0002).

## Accounting principles

Only one session is active at a time; switching must not double-count time. Time represents
user-declared attention, not automatically measured concentration. The application adds no
cloud services, telemetry, window-content monitoring or productivity scoring.

Project names and notes keep their language and characters; application-owned controls,
messages, and report headings are English.
