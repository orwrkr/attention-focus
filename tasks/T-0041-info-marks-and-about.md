# T-0041: Info Marks and an About Button

Status: done.
Dependencies: T-0040.

## Goal

Move the application's information and licenses from Settings to a separate About button. Replace
text that explains other text with "i" marks whose balloon opens on hover, or on keyboard focus when
the control the hint is about can take focus. See ADR-0037.

## Acceptance criteria

- [x] A toolbar button opens About, with the version, the license summary and Licenses (real window).
- [x] Settings no longer holds them, and only one of the two panels is open at a time.
- [x] Explanatory text is behind marks: the change mode, adding projects, the tray, both interval
  options, manual launches, both counters, the report period, auto-refresh, running and interrupted
  time, and the session list.
- [x] Errors, warnings, states that need an action, empty states and figures stay visible.
- [x] A balloon opens while the pointer is on the mark (test, real window).
- [x] A balloon opens while the keyboard focus is on the control, and closes when focus moves on or a
  click lands elsewhere (test, real window).
- [x] A click on a button leaves no balloon open (test).
- [x] Every mark is a Tab stop, so explanations of labels open by keyboard too (test, real window). The
  owner chose this after 0.24.0, for 0.24.1.
- [x] The window scrolls to a widget that takes the keyboard focus, in both directions (test, real
  window). The real window showed that egui does not do this itself.
- [x] Marks stay beside their control at the narrowest width in every language (test).
- [x] A long balloon fits a narrow window in every language (test).
- [x] Right-to-left: marks follow their control on its left, and balloons align right (real window).

## Checks

`cargo fmt --check`, Clippy, `cargo test --locked` (208 tests in 0.24.1),
`scripts/test_release_tools.py`, `scripts/test-release-output.ps1`, `scripts/build.ps1`,
`scripts/verify-release.ps1` and `git diff --check` passed. The guide stays at nine pages. A temporary
failure confirmed that the scrolling test detects a focused widget left out of view.

Checked in the real Development window on 2026-11-05, at 150% scale: About and its width, marks on
the toolbar, counters, Settings, report period, refresh, running time and the Sessions header, hover
and Tab, a narrow window, Arabic, and the dark and light themes. The README's main screenshot was
retaken from synthetic data, and the chart screenshot stays because the chart did not change.

For 0.24.1, also in the real window: Tab stops on marks, a thicker ring on the focused mark, and the
window scrolling down to a mark below its edge, with the balloon opening upward. That check found
that egui does not follow the focus by itself.

Unverified in the real window: the marks for the change mode, adding projects and interrupted time,
which appear only in those states, and Licenses opened from About, whose viewer T-0040 checked before
it moved. Scrolling backward with Shift+Tab, and within the session list, has only the test or no
real-window check. The balloons of languages other than Russian, English and Arabic have tests only.
No independent review was run on this change, which touches only the interface.

`v0.24.1` then ran through the release workflow on a hosted runner, unsigned. The draft's files match
their manifest and its source fingerprint matches the local build. Its EXE and PDF are not
byte-identical to the local ones, and the cause was not examined. The PDF has the same nine pages.

## Handoff

To explain something new, use `ui_support::with_info` or `info`, and keep text visible only when a
person must notice it without looking for it.
