# T-0027: Visible Activity Motion and Write Cadence

Status: done.
Depends on: T-0026.

## Acceptance

- Add visible, calm motion without moving controls or overriding reduced motion.
- Audit timer/checkpoint/UI persistence and verify idle ticks do not write history.
- Reduce UI persistence frequency while retaining normal-exit saving.
- Keep ten-second durable history checkpoints and profile/source boundaries intact.
- Build both EXEs, update the guide and verify the production archive.

## Plan

Add a small moving activity marker beside Tracking. Keep tray motion restrained.
Change UI autosave from two to thirty seconds. Test history writes over simulated
running/stopped periods and rendered motion geometry. Native checks need permission.

## Verification and limits

76 Rust tests, formatting and Clippy passed. The new rendering test verifies 48-point
marker travel inside fixed bounds; existing tests retain stable Start/Stop geometry.
The temporary SQLite fixture verifies no mutations during sixty stopped ticks,
exactly six checkpoint row updates during sixty running seconds with snapshots, and
no mutations after stopping. This counts logical row changes, not physical IO bytes.
No live Production IO measurement or hardware-wear estimate was performed.

Pinned eframe 0.37 source confirms dirty-value comparisons before file writes and
normal-exit persistence. UI save checks now run every thirty seconds; history
checkpoints remain ten seconds with synchronous=FULL. Explicit actions still save.

With fresh permission, native Development 0.15.1 showed the marker at different
positions while the timer advanced; stopping removed it without moving Start/Stop.
A short synthetic Unassigned interval was saved only in Development. Normal Exit
completed and subsequent process lookup found no attention-focus process. The
computer-use JS session was reset immediately afterward. Windows animation settings
were not changed, and native reduced-motion switching was not exercised.

Both canonical EXEs and the ZIP were built through scripts/build.ps1. Version 0.15.1
supersedes the intermediate 0.15.0 package after correcting guide pagination. All nine
final PDF pages were rendered and visually reviewed. verify-release passed hashes,
fingerprint, member allowlist, version metadata and icon resources. Schema remains two.

Next: T-0019 history correction/manual intervals; begin with validation and compatibility
planning. Real sign-in and system power/session checks remain separate T-0002 gates.
