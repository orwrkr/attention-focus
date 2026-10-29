# T-0023: Header Dragging and Centered Window Defaults

Status: done.
Depends on: T-0022.

## Scope and acceptance

- [x] Drag the heading and free space above the scroll area using native window movement.
- [x] Keep Stay on top, Hide to tray and Exit interactive and independent of dragging.
- [x] Center new windows in the monitor work area; preserve ordinary saved positions.
- [x] Recenter inaccessible or legacy top-left positions and center constrained maximization.
- [x] Verify drag/restore/restart in Development and publish both EXEs and verified ZIP.

Implement as T-0023 before T-0019. No history or source changes. Tests use synthetic
geometry, including nonzero work-area origins and negative monitor coordinates.

## Verification

71 Rust tests passed with junction permissions; formatting and Clippy passed. Native
synthetic-window tests distinguish header HTCAPTION from control HTCLIENT hits and
exercise subclass cleanup. Centering tests cover work-area offsets and negative monitor
coordinates. No schema or source changes. All eight guide preview pages were rendered
and inspected; the final packaged PDF matched that reviewed preview byte for byte.

An early Development iteration verified centered first-run placement after removing
only its window entry; all other preferences were retained and backed up beside the
Development settings. Deferred egui dragging did not move the real window, so native
caption hit testing replaced it. An intermediate GetWindowSubclass import failed with
0xC0000139 and was removed in favor of shared UI-thread state. The user closed that
failed process; no forced process termination was performed by the agent.

The agent's final native capture was interrupted by physical Escape. The user then
reported checking the rebuilt Development version and explicitly approved committing
and building. Treat this as user-reported acceptance, not an agent-executed replay of
every native scenario. Production was not used for tests.

Release build initially stopped because Development still held its canonical EXE.
No alternate binary was created. After the user reported it was closed, the retry built
both canonical 0.13.0 EXEs, regenerated notices/PDF and automatically rebuilt the ZIP.
verify-release passed source fingerprint, ZIP allowlist/member hashes, version metadata
and eight neutral icon resource sizes per profile. Production was not closed or modified
for testing. No database migration was introduced.

Next: T-0019 history correction/manual intervals. Other monitor/DPI combinations and
disruptive OS lifecycle acceptance remain separate T-0002 gates.
