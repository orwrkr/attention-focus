# ADR-0022: Header Dragging and Centered Placement

Date: 2026-10-29.
Status: accepted.
Basis: user request; extends ADR-0021.

The app heading and unused space beside pinned header controls are native caption
hit-test regions. The UI publishes their physical client rectangles to its existing
Windows subclass. Only HTCLIENT hits inside those rectangles become HTCAPTION; controls
and native resize borders keep their original result. UI-thread-owned rectangles are
freed on native destruction. Native movement handles restoring a maximized window
during drag; do not implement per-frame position arithmetic or a pointer capture loop.
The app and subclass share UI-thread-only reference-counted state. Avoid importing
GetWindowSubclass: its entry point was unavailable in the loaded common-controls DLL
on the target system, causing Development startup to fail with 0xC0000139.
The deferred egui StartDrag approach passed synthetic input checks but did not move
the real window in native testing; native caption hit testing avoids that timing issue.

Center the first window in its monitor work area. Retain ordinary saved positions.
At startup, recenter collapsed sizes, inaccessible captions, completely offscreen
horizontal placement and legacy top-left anchoring. Do not reset other preferences.
Center the constrained maximized width horizontally, retaining work-area height and
DPI frame handling. This keeps the native caption within the reserved OS work area;
third-party overlays that do not reserve a work area can still cover it, so the client
header provides an additional drag surface.

Do not use eframe NativeOptions.centered unconditionally: pinned eframe applies it
after loading geometry and would override every user-saved position. Use the existing
one-time native startup correction and the presence of its saved window entry instead.
