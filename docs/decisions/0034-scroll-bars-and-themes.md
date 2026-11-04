# ADR-0034: One Scroll Bar Style and Selectable Themes

Date: 2026-11-04.
Status: accepted.
Basis: explicit user request to correct scroll bars everywhere and to offer light, dark and
system themes.

## Context

ADR-0019 set a floating scroll bar with no allocated width for the window body and reserved a
margin for it. That style was set on the body's `Ui`, so every scroll area inside inherited it:
By day, Sessions and the long Timeline painted their bars over their content, such as the day
totals at the right edge. Popups used egui's default instead, so the lists looked different. The
body's bar stood eight points away from the window edge. Handles used full text color. egui 0.37
also fades text at the edges of a scroll area.

The application followed the Windows theme with no choice. Some error and warning labels used
fixed colors chosen for a dark background.

## Decision

**One style, set globally.** `ui_support::scroll_style` applies to both themes, and so to popups
too. It stays in egui's floating mode because only that mode allows a handle in a muted shade of
the theme's text color, which suits both themes without changing other widgets. The handle's
opacity rises as the pointer nears it. Edge fading is off, because it dims rows that are readable.

**Beside the content, never over it.** Scroll areas reserve room for the widest bar and its margin
when they overflow. The window body keeps ADR-0019's stable width: its bar lies in a margin
reserved in advance, so content does not move when overflow changes. Scroll areas inside the body
get the shared style back. The bar now lies at the window edge, which is where people expect it,
while the left and right content margins stay equal.

**Inner areas keep their height.** A scroll area inside a scrolling body is limited by egui to the
height left in the window, which is usually nothing, and so falls to its 64-point minimum. By day,
Sessions and the long Timeline showed about two rows. `ui_support::inner_area` sets a real minimum
and spans the column, so its bar lines up at the right. The Timeline fits a whole month without a
bar of its own.

**The legend cannot widen the chart.** A trailing gap after a legend entry, or a long project name,
could push past the margin. egui then widens the parent layout, so the chart below overran the
margin and reached the scroll bar. Each entry is now one widget, and long names are truncated.

**Themes.** Settings offers Same as Windows, Light and Dark, using egui's `ThemePreference`. Same
as Windows is the default, matching earlier behavior. egui follows Windows changes and keeps the
native title bar in step. The choice is an interface preference in `window.ron`. Errors and
warnings use the theme's own colors.

**The interval option moves to Settings**, next to the midnight option. While it is off, a line
stays above the pickers, because in that mode a change rewrites the current interval's time.

## Alternatives

- egui's solid scroll bars: they reserve space, but color the handle with the widget fill, which
  has very low contrast in the light theme. Changing that fill would affect every button.
- A scroll bar always visible in the body: width would be stable without a margin, but it would
  show a track with nothing to scroll, which ADR-0019 rejected.
- A separate theme for each window state or custom palettes: more than was asked for.

## Consequences

Scroll areas lose 14 points of width only while they overflow. Regression tests check that bars
in the body and inside it never intersect text, that inner areas keep their height, that the legend
never widens the layout, and that the style is the same in both themes. Both layout defects were
found only by looking at the real window: the shape-level checks and two code reviews had missed
them. Automatic switching relies on the Windows notification that egui receives, so it needs a check
by someone who can change the Windows app mode.
