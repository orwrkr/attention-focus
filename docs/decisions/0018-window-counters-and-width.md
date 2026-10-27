# ADR-0018: Responsive Counters and Native Maximized Bounds

Date: 2026-10-28.
Status: accepted.

Show Current interval, Today for project and Today - all projects as distinct cards.
The large blue Start/Stop action belongs to the interval card. Below 600 logical pixels,
place Current interval above the two daily cards; otherwise use three equal columns.
Measure Frame.total_margin including stroke rather than assuming its width. Reserve
a solid scrollbar gutter and mirror its width at the left content margin, so borders
stay equally inset and the bar cannot cover a card. Truncated project names use egui's own single hint.
Headless rendering tests cover equal edges at 100/125/150/200 percent scale and one hint.

Compute both daily sums in the same cached read of finalized history, clipped to the
Windows local calendar day. Project identity remains source/path/slug scoped. The all
sum includes every source, project and unassigned focus. Add active clipped time once
to both sums; exclude review time. Date/zone/project changes and successful commands
invalidate the cache. Failures publish unavailable without blocking tracking state.
No schema change. Preserve all three durations within Windows' 127 UTF-16 tooltip limit,
shortening the project name and using a compact Development profile label as needed.

Keep a static 760-logical-pixel width limit and generous 16,384 height ceiling. winit
0.30.13 sets only ptMaxTrackSize. Delegate native WM_GETMINMAXINFO first, then constrain
maximized width and explicitly account for monitor work area and DPI-scaled invisible
resize borders. Normalize restored maximized geometry once before the first rendered
frame. The user found caption/frame clipping in the initial width-only implementation;
preserving its default maximized position was insufficient. Ordinary height resizing
remains native; never send measured UI height/resize commands each frame.

Use a static SetWindowSubclass callback on the UI thread, removed at WM_NCDESTROY.
raw-window-handle 0.6 was already transitive and is now direct for the root handle.
Existing windows-sys features expose monitor/HiDpi/Shell functions; no extra DLL ships.
Native synthetic-window tests verify original-handler chaining and monitor/frame bounds.
Multiple-monitor and other native DPI combinations remain explicit acceptance gates.

References: [MINMAXINFO](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-minmaxinfo),
[WM_GETMINMAXINFO](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-getminmaxinfo).
