# ADR-0013: Live Reports and Desktop Controls

Date: 2026-10-27.
Status: accepted.
Basis: explicit user request.

## Decision

An expanded visible report requests a worker snapshot every two seconds and after
committed tracking changes. Never query solely because a hidden/collapsed report exists.
Allow one in-flight request, debounce typed filters for 350 ms, and keep controls editable
during automatic queries. Tag requests and retain their parameters; discard responses
for superseded filters. Preserve the last good result with its own range/snapshot label
when a query fails. Rate-limit retries, including full queues. Presets recompute dates
in the selected zone on refresh, including after local midnight.

Calendar popups use existing Jiff civil-date arithmetic, beside both editable date
fields, with month/year navigation and Monday-first valid days. A searchable dropdown
lists the bundled timezone database, UTC first. Existing DST accounting and the report
range cap remain authoritative. No new date/widget dependency or notice input is needed.

Stay on top sets the native viewport WindowLevel and stores a boolean in the existing
per-profile window.ron storage. Save immediately when toggled and through the application
save hook. Default off when absent. Tracking schema remains two; upgrades from 0.2 still
use the previously implemented schema-one migration and backup.

Tooltips include the current project only while tracking. Reserve room for session and
total times; shorten long project names on Unicode scalar boundaries within Windows'
127 UTF-16-unit limit. Normalize control characters in titles, retaining multiline
structure and recovery flags. Normal headers show name/version without Production;
Development remains explicit. Use readable blue secondary text in dark/light themes
and emphasized activity captions, retaining distinct error/disabled states.

## Validation

Use synthetic channels/injected Instants for scheduling/staleness/backoff, civil dates
for leap/month boundaries, actual bundled timezone resolution, and Unicode tooltip
limits. These do not substitute for native calendar, topmost, tooltip, or theme checks.
Build both EXEs and automatically refresh the reviewed guide/production ZIP.
