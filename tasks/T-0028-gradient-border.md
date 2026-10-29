# T-0028: Gradient Tracking Border

Status: done.
Depends on: T-0027.

## Acceptance

- Replace the short-line marker with a fading trail around Current interval.
- Follow rounded corners continuously with an eight-second clockwise lap.
- Preserve layout, reduced-motion behavior, tray states and persistence cadence.
- Verify rendered geometry, both EXEs, guide and production package.

## Plan

Paint a quarter-perimeter gradient trail over the fixed card frame. Keep the base
frame static and use the existing visible-only repaint schedule. No new storage calls.

## Verification and limits

76 Rust tests, formatting and Clippy passed. Rendering tests verify a graded tail,
continuous adjacent segments, card containment, changed position across phases and
an identical frame after eight seconds. Existing narrow/wide counter and Start/Stop
geometry tests pass. No tracking, storage schema or save cadence changes.

Both canonical 0.16.0 EXEs and the production ZIP were built through scripts/build.ps1.
All nine guide preview pages were rendered and visually reviewed. The shipped PDF
matches the preview hash. Release verification passed package hashes, fingerprint,
allowlist, metadata and neutral icon resources.

The user approved a short native check. Development was launched, but list_windows
timed out on the initial call, the retry and a reset/reinitialize retry. No UI input
or tracking start was sent. The JS session was reset and the user was told that
Development may remain open and can be closed via Exit. No process was terminated.
Native visual acceptance remains unverified because the computer-use helper failed.

Next: user review of the gradient border, or a fresh authorized desktop session when
the helper is responsive. T-0019 remains the next feature task; define validation and
compatibility before changing history. Real OS acceptance remains under T-0002.
