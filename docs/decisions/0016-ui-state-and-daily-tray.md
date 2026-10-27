# ADR-0016: UI State, Native Sizing and Project-Day Tray Totals

Date: 2026-10-27.
Status: accepted.

Persist egui memory for section/scroll state and explicit versioned report controls,
searches and note draft in the profile's existing window.ron. Never serialize reports,
pending commands or active timer state. Restored relative presets roll to today's dates;
custom dates/minutes remain exact. First launch and unreadable preferences use defaults.

Supersede ADR-0015 automatic native size clamps: resizing and maximization belong to
Windows and the user. Limit the content column to 744 logical pixels with vertical
scrolling, without sending size commands each frame. Native geometry remains persisted.
This avoids reflow/resize feedback and permits ordinary downward resizing.

Replace tray Total with Today for the selected project (also named while stopped).
Use the Windows local day independently of report timezone/filter, source ID/path/slug
identity and all tasks, including Unassigned focus. Clip saved and current elapsed time
to day boundaries using the same cumulative allocation as reports. Exclude review time.
Cache saved totals per day/project, invalidate on commands, and add live time once.
Idle workers wake every 30 seconds for date/zone rollover; tracking updates every second.
Failed daily accounting shows unavailable rather than blocking successful tracking state.
No schema or dependency change. Validate midnight, project/source/task isolation,
reclassification, corruption and preference round trips with synthetic tests.
