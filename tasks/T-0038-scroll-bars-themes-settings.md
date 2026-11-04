# T-0038: Scroll Bars, Themes and Interval Setting

Status: done.
Dependencies: T-0036.

## Goal

Make scroll bars consistent and keep them off the content everywhere. Offer light, dark and system
themes. Move the interval option to the other settings. See ADR-0034.

## Acceptance criteria

- [x] Every scroll area, including popups, uses one style with theme-derived colors.
- [x] A scroll bar never covers content, in the window body or inside it (test and real window).
- [x] The body's content width does not change with overflow (existing test).
- [x] Areas inside the body keep a usable height, and their bars line up at the right.
- [x] Settings offers Same as Windows, Light and Dark, and the choice survives a restart.
- [x] The title bar follows the theme, including from Dark back to Same as Windows.
- [x] Errors and warnings use the theme's own colors.
- [x] "Start a new interval when project or task changes" is in Settings, and a line above the
  pickers stays visible while it is off.

## Checks

`cargo fmt --check`, Clippy and `cargo test --locked --lib` (169 tests) passed. Three regression
tests were each confirmed by a temporary mutation that made them fail.

Checked in the real Development window on 2026-11-04, on one machine at 150% scale with Windows in
light mode: both themes with the title bar, the restart, the return to Same as Windows, the scroll
bars in the body, By day, Sessions and the time zone list, and the settings panel. An independent
review of the first version found no defect, and an upgrade of an old Development database from
schema two to four also ran.

Those window checks found two defects, both fixed: inner areas had shrunk to about two rows, and
the legend could push the chart past the margin. They are recorded in ADR-0034.

Unverified: switching while Windows changes its app mode, which an agent must not do on the owner's
machine; the reminder line while the interval option is off; other scales and monitors.

## Handoff

The owner can change the Windows app mode with Same as Windows selected, and the window and its
title bar should follow.
