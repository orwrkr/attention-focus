# ADR-0005: Persistent Desktop State and Tray Tracking

Date: 2026-10-27.
Status: accepted.
Basis: explicit user request to continue, remember window placement, add application
icons and running/stopped tray states, and toggle tracking on double click.

## Decision

Continue the desktop work despite the previously unavailable UI verification helper.
Keep interactive checks explicit; the request authorizes implementation, not a claim
that the previous checks passed.

Use a shared procedural focus-mark design for the embedded executable icon, window
icon, and tray states. Include multiple ICO sizes in the PE resources using winresource
at build time. No external icon files are required at runtime. Running/stopped states
differ by both color and center symbol. Handle only native left-button double-click
events for toggling; a single click does not open the window. Show Focus is in the menu.

Enable eframe window persistence, including position, size, and maximization, in a
fixed LocalAppData application directory. UI geometry uses eframe's RON format; session
data remains SQLite. This is a deliberate exception to ADR-0004's single settings DB
so tested framework geometry handling can be reused. Save geometry periodically and
on clean exit, including while hidden. Do not persist a hidden startup state.

Provide a minimal durable Unassigned focus timer before MetaPlan integration so the
requested toggle controls real recorded time rather than just changing an icon.
A worker owns SQLite, serializes commands, and confirms changes before UI/tray updates.
Interrupted sessions require review and are never resumed automatically.

The requested desktop settings and unassigned sessions may initialize before source
selection, superseding the original first-launch ordering for this bootstrap stage.
No source is configured or accessed here. T-0003 must reject a source that overlaps
the fixed storage root and must preserve this restriction for all writable files.

Use the static MSVC runtime for the Windows x64 target. Inspect the built executable's
imports before documenting deployment requirements. Clean-machine and graphics-driver
compatibility still require testing; a successful local build is not sufficient.

## Sources

- [eframe native persistence](https://docs.rs/eframe/latest/eframe/struct.NativeOptions.html)
- [winresource PE resource embedding](https://docs.rs/winresource/latest/winresource/)

## Consequences

The app needs only its executable beside it; generated data belongs in the user's
profile. Project/task attribution and the full OS interruption policy remain later
work. Runtime errors must not silently flip the icon or discard session state.
