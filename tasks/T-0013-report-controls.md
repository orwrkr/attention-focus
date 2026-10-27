# T-0013: Live Reports and Desktop Controls

Status: done.
Dependencies: T-0012.

## Scope and acceptance

Refresh expanded visible reports automatically, with one request in flight and stale
filter-response protection. Keep filters editable, debounce typing, and refresh after
committed tracking changes. Add calendar selection alongside editable date fields and
a searchable bundled IANA timezone list. Persist Stay on top with window settings,
without a tracking database migration. Include current project in bounded Unicode-safe
tray tooltips. Remove Production from the production header and improve secondary-text
contrast/style. Keep Development identification, read-only sources, and isolated data.

Verify refresh scheduling, stale replies, leap calendars/month boundaries, timezone
choices, tooltip length/state, and preference persistence. Update guide, build both EXEs
and versioned ZIP, and record manual Windows verification limits.

## Verification and limitations

- 45 Rust tests pass, including six new calendar, tooltip, refresh, stale-reply,
  debounce/commit, error/backoff, and bundled-timezone checks. Formatting and Clippy
  with warnings denied pass. Five synthetic packaging tests pass.
- Reviewed all six rendered guide PDF pages. Packaged PDF is byte-identical to the
  preview; page count/text/version metadata also verified.
- Both 0.4.0 EXEs built using scripts/build.ps1; the ZIP refreshed automatically.
  verify-release passed for versions, eight neutral icon sizes in each EXE, source
  fingerprint, build stamp, exact ZIP members, manifest, and payload hashes.
- Stay on top restore/save and viewport calls were reviewed against local eframe APIs.
  Native behavior and setting persistence were not exercised interactively. No history
  schema change was introduced; the setting uses existing per-profile window.ron storage.
- Windows discovery timed out twice, then JS initialization timed out after reset.
  No app was launched. Calendars, timezone search, live report interaction, native
  topmost behavior, tooltip display, and theme contrast still require manual review.
- No real history/settings database or MetaPlan file was changed by development tools.
  The external running portable copy was not stopped, replaced, or migrated.

## Outputs and next step

- Production: dist/AttentionFocus/attention-focus.exe.
- Development: target/debug/attention-focus.exe.
- Guide: dist/AttentionFocus/User-Guide-0.4.0.pdf.
- Package: dist/AttentionFocus-0.4.0-windows-x64.zip.

Use Development to verify open/collapsed/hidden report refresh, typing during queries,
calendar selection and month/year navigation, timezone search, Stay on top through
hide/show/restart, project tooltip and secondary text under dark/light themes. Keep
T-0005/T-0002 manual acceptance pending; never create synthetic Production time.
