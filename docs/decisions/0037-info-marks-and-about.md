# ADR-0037: Info Marks for Explanations, and an About Button

Date: 2026-11-05.
Status: accepted.
Basis: explicit user request to move the application's information and licenses to a separate
button, and to replace text that explains other text with "i" marks and balloons, to save space and
reduce the load on the reader.

## Context

The window carried many sentences that explained a label or a button, and each one took lines
whether or not the reader needed it. A few explanations were already tooltips, with nothing to show
that they existed.

## Decision

**About is its own button**, beside Settings. It opens a panel with the version, the license summary
and **Licenses**. One of the two panels is open at a time, and closing About drops the license text.

**A mark replaces explanatory text.** `ui_support::info` draws an "i" in a circle from shapes rather
than text, so it looks the same whichever font a language uses. `with_info` adds a control with its
mark and leaves it the rest of the width, and `info_after_header` places one beside a section header.
The balloon is egui's tooltip, at most 320 points wide and never wider than the window.

**What opens a balloon.** The pointer on the mark, at once, as the mark has no other purpose. The
keyboard focus on the mark or on the control it explains: egui gives buttons and checkboxes focus
only through the keyboard, so a click leaves no balloon behind.

**Every mark is a Tab stop.** The first version, 0.24.0, left marks out of the Tab order to save
stops, so explanations of labels opened only under the pointer. The owner then chose to add them, in
0.24.1, which makes every explanation reachable by keyboard. A mark that has the focus draws a
thicker ring.

**The window follows the focus.** egui moves the keyboard focus without bringing the widget into
view, so a Tab stop and its balloon could be off screen. The real window showed this. The body now
scrolls to a widget that gains focus when its clip cuts it, as does the session list. The fixed
header is never cut, so it moves nothing.

**What stays visible.** Errors, warnings, states that ask for an action such as interrupted time or
a startup entry that points elsewhere, the progress of an export, empty states and all figures. The
report's auto-refresh state and "Updating report" moved behind a mark beside Refresh report, so that
the layout no longer shifts when they change.

**Existing tooltips stay where they are.** Rename, Archive, Delete, Custom and the project search have
no visible text to replace, and a mark in every project row would add clutter. They now also open on
keyboard focus. The search field's balloon closes once it holds text, as it would cover the results.

**The change mode.** ADR-0034 kept its notice visible because that mode rewrites time. The notice now
sits behind a mark beside the project picker, which appears only in that mode, and the button that
applies a change still says that it changes the current interval.

## Alternatives

- Native Windows tooltips: egui draws the whole window, so nothing native could anchor to a control.
- Tab stops only on marks beside labels: fewer stops, but marks would behave differently from one
  another, and the owner chose marks in the Tab order without that exception.
- A delay before the balloon, as for ordinary tooltips: it only slows the one thing a mark does.
- Replacing state and warning text as well: a person has to notice those without looking for them.

## Consequences

The window is shorter, and the two lower counter cards no longer reserve empty rows. An explanation
now takes a deliberate step to read. Each mark adds a stop to the Tab order.
Marks and their balloons are drawn inside the window, so a balloon near an edge moves to stay inside.
