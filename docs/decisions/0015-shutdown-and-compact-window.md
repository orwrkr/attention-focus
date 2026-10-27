# ADR-0015: Confirmed Session End and Compact Window

Date: 2026-10-27.
Status: accepted.
Basis: user request to clarify interruption behavior, reduce blank window space and
continue planned acceptance work. Extends ADR-0008 and refines ADR-0005 sizing.

## Decision

The hidden native lifecycle window returns TRUE immediately for WM_QUERYENDSESSION.
Only WM_ENDSESSION with nonzero wParam saves an active interval. Test ENDSESSION_LOGOFF
as a bit mask; record logoff or shutdown as the end reason. A canceled query leaves
tracking unchanged. Reuse the captured wall/monotonic boundary and existing worker
confirmation with a 1.5-second wait. Do not block or veto Windows shutdown. No database
migration is needed. Native hidden-window messages are tested synthetically; do not
shut down, log off, lock or suspend the user's machine as an automated test.

Window close with a working tray keeps tracking; explicit Exit commits before closing.
Sleep/hibernate notifications stop, and resume never restarts automatically. Force-kill,
power loss, missed notifications or failed saving use durable checkpoint review. The
ten-second checkpoint cadence is not a guaranteed upper bound on data loss under IO
failures or forced termination.

Cap client width at 760 logical pixels. Measure actual scroll content and pinned header
to adapt the height limit, reserving space for desktop chrome. Maximize restores and
fits that content rather than leaving a monitor-sized mostly blank window. Keep vertical
scrolling for larger content, the minimum usable size and per-profile persistence.
Use installed egui/eframe 0.37 ViewportCommand sizing and scroll output APIs; add no
dependencies. Check native maximize, expanding/collapsing content and narrow layout.

## Sources

- [WM_QUERYENDSESSION](https://learn.microsoft.com/en-us/windows/win32/shutdown/wm-queryendsession)
- [WM_ENDSESSION](https://learn.microsoft.com/en-us/windows/win32/shutdown/wm-endsession)
- [PBT_APMSUSPEND](https://learn.microsoft.com/en-us/windows/win32/power/pbt-apmsuspend)
- Installed egui 0.37 scroll-area/viewport and winit 0.30 Windows sizing implementations.
