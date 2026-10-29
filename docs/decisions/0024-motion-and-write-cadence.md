# ADR-0024: Visible Activity Motion and Bounded UI Persistence

Date: 2026-10-29.
Status: accepted.
Basis: user feedback; extends ADR-0020.

Add a 48-point horizontal marker movement inside a fixed 56-by-16 status region.
Use the existing eight-second cosine and twenty-fps visible-card repaint schedule.
Keep the line and marker static at the center when Windows disables animation or
its preference cannot be read. Do not change tray brightness, timekeeping, frame
geometry or input coordinates. Paint operations have no storage calls.

Increase UI persistence checks from two to thirty seconds. Pinned eframe 0.37
serializes geometry, egui memory and application preferences, then FileStorage writes
only when serialized values changed. Normal Exit still saves; explicit Stay on top
changes retain their immediate flush. This reduces periodic opportunities from thirty
to two per minute, not a claim about physical disk writes or hardware wear.

Keep history checkpoints at ten seconds and SQLite synchronous=FULL. One logical
checkpoint update can involve several journal/filesystem writes. Start, stop, switches,
recovery and confirmed lifecycle boundaries remain durable immediately. Worker snapshots
and report reads may perform IO but do not write history. A simulated fixture checks
zero idle mutations and six checkpoint row updates in sixty running seconds.

Tradeoff: a forced exit can lose up to roughly thirty seconds of recent UI preferences;
this does not increase the history checkpoint recovery window. No schema migration,
new dependency, real-user history instrumentation or Production mutation is required.
