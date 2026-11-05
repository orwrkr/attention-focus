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
- [x] Marks stay beside their control at the narrowest width in every language (test).
- [x] A long balloon fits a narrow window in every language (test).
- [x] Right-to-left: marks follow their control on its left, and balloons align right (real window).

## Checks

`cargo fmt --check`, Clippy, `cargo test --locked` (206 tests), `scripts/test_release_tools.py`,
`scripts/test-release-output.ps1`, `scripts/build.ps1`, `scripts/verify-release.ps1` and
`git diff --check` passed. The guide stays at nine pages, with pages 1, 5 and 8 changed.

Checked in the real Development window on 2026-11-05, at 150% scale: About and its width, marks on
the toolbar, counters, Settings, report period, refresh, running time and the Sessions header, hover
and Tab, a narrow window, Arabic, and the dark and light themes. The README's main screenshot was
retaken from synthetic data, and the chart screenshot stays because the chart did not change.

Unverified in the real window: the marks for the change mode, adding projects and interrupted time,
which appear only in those states, and Licenses opened from About, whose viewer T-0040 checked before
it moved. The other seven languages' balloons have tests only. No independent review was run on this
change, which touches only the interface.

## Handoff

To explain something new, use `ui_support::with_info` or `info`, and keep text visible only when a
person must notice it without looking for it.
