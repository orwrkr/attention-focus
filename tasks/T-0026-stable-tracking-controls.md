# T-0026: Stable Tracking Controls and Settings Cleanup

Status: done.
Depends on: T-0025.

## Acceptance

- Preserve the current settings panel placement.
- Order header buttons Settings, Hide to tray, Exit.
- Remove the animation diagnostic from settings; retain guide explanations.
- Keep the tracking action at the same position across stopped/running states.
- Verify regression coverage and rebuild both canonical EXEs and the package.

## Plan

Use identical status-row geometry for both tracking states, verify actual painted
button positions, then run Rust/release checks. Native UI checks require fresh
computer-use permission. Refresh remains an explicit read of Windows registration.

## Verification

The extended actual-paint regression failed before the fix: the button text moved
from y=93.5 to y=98.5 when tracking began. Identical status-row layout now keeps the
button position unchanged in narrow and wide counter arrangements. All 74 Rust tests,
formatting and Clippy passed; Windows fixture tests used required permissions.

With fresh user permission, Development 0.14.3 confirmed Settings/Hide to tray/Exit
order, no animation diagnostic in settings, and stationary Start/Stop through a real
start/stop cycle. The approximately six-second synthetic Unassigned interval was
saved only in Development. Startup registration was unchanged. Normal Exit completed;
subsequent process lookup found no attention-focus process. The computer-use JS session
was reset immediately after UI work; no separate disconnect API is available.

Both canonical EXEs and the production archive were rebuilt through scripts/build.ps1.
All nine guide pages were rendered and visually reviewed. Release verification passed
source fingerprint, member hashes, exact ZIP contents, metadata and neutral icons.
No database schema change. Real sign-in remains an unexecuted T-0002 acceptance gate.

Next: T-0019 history correction/manual intervals, beginning with validation and
compatibility decisions. Obtain fresh permission before any new computer-use session.
