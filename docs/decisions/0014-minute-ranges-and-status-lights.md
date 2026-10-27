# ADR-0014: Minute Ranges and Status Lights

Date: 2026-10-27.
Status: accepted.
Basis: explicit user request. Replaces tray transport symbols from
[ADR-0011](0011-versioned-distribution.md) and unconditional report polling from
[ADR-0013](0013-live-reports-and-desktop-controls.md).

## Decision

Use a filled green status light while tracking and a hollow gray ring while stopped.
Neither depicts an available action. Keep neutral blue/purple application identity,
the Development D badge, explicit status tooltip, and existing double-click behavior.

Add HH:MM fields to report From/Until dates. Interpret filters as [From, Until), with
24:00 accepted only as Until's end of day. Whole-day presets use 00:00/24:00, preserving
existing date-range behavior. Resolve non-midnight civil minutes strictly: ambiguous
or skipped times produce a readable error and a UTC alternative. Midnight uses the
existing first-valid-start-of-day policy. Clip each daily bucket to the exact filter;
cumulative elapsed allocation keeps adjacent periods additive and prevents totals
outside the selected minutes. Filter changes never edit recorded sessions.

Only poll while tracking is running and current wall time lies inside the filter.
Past portions of a still-running interval cannot grow, so do not poll them. Send one
final snapshot on leaving the filter. Refresh once after an intersecting interval
stops, switches, or is reassigned, or when matching recovery time changes category.
Unrelated commits do not reload history. Initial load, reopening, changed filters,
presets rolling at midnight, and explicit Refresh may query without live overlap.
Retain single in-flight requests, parameter guards, debounce and failure backoff.

Expose actual active start in the worker snapshot through a read-only query, without
changing SQLite schema. Box report payloads to keep channel events bounded after the
request gains minute fields. No dependency changes or history migration are added.

Explain search as a case-insensitive project-name/folder-ID dropdown filter, without
changing active focus or searching task/history text. Explain Running and Needs review
as clipped, separate, not-yet-finalized/unconfirmed categories. Zero can mean no overlap
or less than one displayed second. Add hover help and explicit automatic-refresh status.

## Validation

Test minute boundaries, timezone equivalence, DST rejection, midnight/day clipping,
adjacent additivity, selective polling, final snapshots, relevant stop/recovery changes,
and unrelated commits. Review generated status artwork/PDF; native Windows interaction
remains a separate acceptance gate. Report existing deferred work without expanding it.
