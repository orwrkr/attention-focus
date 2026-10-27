# T-0007: Tray Single Click and Tracked-Time Tooltip

Status: done.
Dependencies: T-0006.

## Scope

Single left click opens the window; native double click still toggles tracking.
Wait for the Windows double-click interval before opening on a single click, so
a double click does not accidentally open the window. Test the surrounding releases.
Show a native hover tooltip with current session and total time, including the active
session once. Refresh each worker snapshot even while the main window is hidden.
Exclude unreviewed interrupted time and indicate when review is needed.

## Checks

Ten tests passed, including single-click activation and cancellation around a double
click. Formatting, Clippy with -D warnings, and release build passed. Only the known
environment-related incremental-cache hard-link fallback warnings were emitted.
The distributable EXE is updated in dist/AttentionFocus. Documentation links and
encoding checks passed. Interactive Windows hover/click behavior has not been verified;
the previously unavailable helper remains a limitation, not a claimed test pass.
