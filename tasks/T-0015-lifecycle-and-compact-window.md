# T-0015: Confirmed Shutdown and Compact Window

Status: done.
Dependencies: T-0014.

## Scope and plan

Explain window close, explicit exit and interruption recovery. Add durable handling
of confirmed Windows logoff/shutdown without stopping on a canceled shutdown query.
Bound window width and adapt its height to content; maximize should fit useful content
instead of creating a mostly empty monitor-sized window. Preserve scrolling, pinned
actions, per-profile geometry, read-only source boundaries and existing tracking rules.
Build version 0.6.1 and continue the existing native acceptance checklist afterward.

## Acceptance criteria

- [x] Confirmed end-session saves at its captured boundary; canceled queries do not stop.
- [x] Synthetic native message and durable engine tests cover logoff/shutdown.
- [x] Window resizing/maximizing avoids excessive blank space without hiding controls.
- [x] Explain clean/forced exit, sleep/hibernate and recovery in the user guide.
- [x] Both EXEs, PDF and automatic production ZIP pass release checks.
- [x] Record native acceptance evidence and remaining gates without real system disruption.

## Verification and handoff

52 Rust tests, five packaging tests, formatting and Clippy passed. Junction isolation
tests needed permission to create synthetic temporary links; the rerun passed all tests.
Synthetic messages tested confirmed/canceled session end; engine tests reopened durable
logoff/shutdown/suspend rows. These are not real system shutdown tests.

Both 0.6.1 EXEs and the automatic ZIP passed verify-release. All seven PDF pages were
rendered and visually reviewed. Initial 0.6.0 native testing found that Windows size
constraints alone did not shrink an existing tall window after collapse. Version 0.6.1
explicitly clamps its size. Maximize fitting and automatic collapse shrinking passed
native checks. Header actions remained accessible with scrolling.

Exit during synthetic tracking finalized the interval; read-only Development inspection
found no unfinished or overlapping rows. Restart remained stopped. A moved window
restored identical placement/size; Stay on top restored its checked preference. Restored
that preference to off afterward. The user authorized repeated agent-launched Development
test runs and their normal UI Exit; no repeat approval is needed for the same scope.
Development was exited normally at handoff; external Production remained untouched.

T-0005 also verified task switching, EditCurrent preserving elapsed time, recent task
ordering, task totals and UTC selection. Actual tray input/rendering, topmost Z-order,
minimum-width resizing, multiple monitors and real sleep/hibernate/logoff/shutdown are
still T-0002 limits. The helper exposes app windows but no taskbar/tray window. Do not
disrupt the user's machine to simulate a real OS transition. Next: T-0002 and T-0016.

Pre-commit follow-up: after physical Escape interrupted the last Computer Use check,
read-only process inspection confirmed Development exited and external Production
remained running. Repeated all 52 Rust tests, five package tests, formatting, Clippy
and verify-release successfully. No additional GUI or Production data access was needed.
