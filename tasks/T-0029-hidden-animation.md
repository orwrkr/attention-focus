# T-0029: Suspend Invisible Animation

Status: done.
Depends on: T-0028.

## Acceptance

- Stop generating border geometry and animation repaint requests outside the clip area.
- Skip animation for minimized or reported-occluded viewports.
- Make tray hiding minimize before hiding so the native backend skips GPU rendering.
- Preserve tracking and restore the window when requested.
- Verify tests and build both canonical EXEs and the production archive.

## Verification and limits

77 Rust tests, formatting and Clippy passed. The new headless rendering test checks
visible, disabled, clipped, minimized and reported-occluded cases after warmup.
Invisible cases emit zero border segments and no 50-ms repaint loop; visible motion
emits 96 segments and requests its next frame. Existing counter geometry tests pass.

Pinned egui-winit/eframe sources confirm Minimized commands update viewport state,
and invisible viewports use logic-only updates without UI/GPU painting. Worker state
and tray updates continue. Complete occlusion depends on OS reporting; focus loss
alone is not invisibility. Already queued transition frames may still settle.

Native hide/restore interaction and GPU counters were not measured in this task.
Desktop permission was requested; no new computer-use session was activated without
an answer. No Production data or tracking schema changed.

Both canonical 0.16.1 EXEs and the production archive were rebuilt via scripts/build.ps1.
All nine guide preview pages were rendered and visually reviewed. verify-release passed source fingerprint, ZIP hashes/allowlist, version metadata and
neutral icon resources. Preview and shipped PDF SHA-256 hashes match.

Next: native hide/restore and scrolled-card acceptance with fresh desktop permission,
then T-0019 validation/compatibility planning. Keep real OS checks under T-0002.
