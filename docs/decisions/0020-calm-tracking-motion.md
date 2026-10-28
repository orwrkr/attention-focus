# ADR-0020: Calm Tracking Motion

Date: 2026-10-28.
Status: accepted.
Basis: explicit user request; extends ADR-0014 status lights.

## Decision

Keep filled green running and hollow gray stopped tray states. Add an eight-second
brightness cycle with five precomputed frames, at most one update per second and
twelve percent amplitude. The green center, silhouette, transparency and Development
D badge remain unchanged. Permanent file/window/taskbar identity icons stay neutral.

Only Current interval receives a green border, low-opacity tint and explicit green
Tracking light/text. A cosine changes the accent slowly over eight seconds, requesting
at most twenty frames per second only when the card is visible. No window-edge motion,
flashing, spinning or layout changes. Stopped remains static. Animation is decorative,
never evidence of a successful database write and never the timekeeping clock.

Read Windows SPI_GETCLIENTAREAANIMATION without changing system settings. Disabled
animations or lookup failure give static status. Existing worker state events drive
tray updates even while hidden; do not add a polling thread or database queries.

## Alternatives and consequences

Animating the entire window edge or blinking the tray is more distracting. A rotating
symbol risks appearing like a loading/error indicator at small sizes. Restrained color
motion preserves recognition; actual shell visibility still depends on tray settings
and display rendering. Text and filled/hollow shape convey state without motion/color.
No dependencies, settings migration or database schema changes are required.
