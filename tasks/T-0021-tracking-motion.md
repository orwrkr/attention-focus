# T-0021: Calm Tracking Activity Indicators

Status: done.
Depends on: T-0020.

## Scope

Make ongoing tracking easy to recognize with a slow green interval-card accent and
a restrained tray brightness cycle. Preserve static identity icons, explicit text,
filled running and hollow stopped shapes. Do not animate the entire window or change
tracking logic, history, saved geometry, or source access.

## Acceptance

- [x] Only the running interval has a green frame, subtle fill and status light.
- [x] Smooth eight-second window motion; five cached tray levels change at most once
  per second, within twelve percent brightness. Never blink or resemble stopped.
- [x] Respect Windows client-area animation preference; static status on lookup failure.
- [x] Stopped tracking has no animation; hidden UI needs no animation repaint loop.
- [x] Verify status/shape/badge invariants, layout, Rust checks and release package.

## Validation

69 Rust tests passed with junction permissions, including bounded periodic motion,
static reduced motion, running green/alpha/badge invariants, unchanged identity/stopped
icons and identical narrow/wide counter geometry across start/stop. A restricted test
run could not create three junction fixtures; the permitted full run passed all tests.

Native Development 0.12.0 on synthetic data verified running green frame/tint/dot/text,
advancing counters across multiple cycles, unchanged daily cards and clean stopped
styling. One short Unassigned synthetic interval was saved in Development only. Normal
Exit completed; process lookup found only the untouched external Production copy.
The native tray-shell animation and changing Windows accessibility preferences were
not exercised. Deterministic frames and preference-off policy are covered by tests;
no system preference was changed. No schema, source or Production history changes.

Formatting, Clippy and five packaging tests passed. Both canonical 0.12.1 EXEs and
the automatically rebuilt production ZIP passed verify-release source fingerprint,
allowlist/member hashes, metadata, and eight neutral icon resource sizes per profile.
All eight guide pages were rendered and inspected. The first 0.12.0 guide had an
orphaned paragraph; final 0.12.1 moves the explanation into troubleshooting and restores
eight-page pagination. Native UI was checked on 0.12.0; final 0.12.1 changes only guide
placement and version metadata. No application behavior changed after native testing.

Next: T-0019, history correction/manual intervals. T-0002 native tray/monitor/power/
clean-machine acceptance gates remain separate.
