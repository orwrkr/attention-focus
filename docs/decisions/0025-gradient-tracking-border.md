# ADR-0025: Gradient Tracking Border

Date: 2026-10-29.
Status: accepted.
Basis: explicit user request; replaces the marker presentation in ADR-0024.

Replace the status-row marker with a clockwise fading trail on Current interval's
rounded frame. A lap takes eight seconds at constant perimeter speed. The tail covers
one quarter of the perimeter, using 96 short segments with increasing opacity toward
the head. Inset the 2.5-point stroke to stay inside the card. Rounded-corner distances
are included in the perimeter, so there is no corner jump or wrap discontinuity.

Keep the base frame and tint static. No widget allocation depends on animation phase.
Use the existing twenty-fps visible-card repaint schedule; hidden/minimized cards do
not schedule animation. When Windows disables motion or its preference cannot be read,
show the static tracking frame. Stopped frames and permanent icons remain static.
Tray motion, ten-second history checkpoints and thirty-second UI persistence remain
unchanged. This is decorative painting, with no new storage calls or dependencies.
