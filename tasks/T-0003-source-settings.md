# T-0003: Read-Only Source and Settings

Status: done.
Dependencies: T-0002 implementation; user authorized continued development.

## Goal

Deliver read-only source and settings within the initial architecture.

## Scope

Follow [the architecture](../docs/ARCHITECTURE.md) and ADR-0004. Keep all authored
content and UI in English. Preserve external Unicode data and the read-only boundary.
Do not include later tasks or deferred product features.

## Plan

Implement the acceptance criteria in order, then verify and update project context.

## Acceptance criteria

- [x] Implement source selection and fixed external storage with canonical path overlap checks before any writes.
- [x] Read project names, status, priority, and nonempty unfinished current-cycle tasks from synthetic MetaPlan fixtures.
- [x] Handle missing files, duplicate slugs, archived projects, and malformed data without writing to the source.
- [x] Preserve Unicode through escaped protocol tokens and synthetic test data; keep authored text English.
- [x] Test source tree byte-for-byte integrity, nesting, sibling prefixes, case aliases, and supported junction cases.
- [x] Persist the source selection outside MetaPlan and restore it safely on restart.

## Checks

Fifteen tests passed with cargo test --offline --locked. New checks cover whole-source
file paths/bytes, Unicode, task filtering and section context, malformed/missing files,
archives, duplicate slugs, source identities, rejected updates, missing sources, corrupt
settings, nesting, sibling prefixes, case aliases, real junction aliases, and linked
project exclusion. Junction creation was denied in restricted execution; rerunning with
approved permissions passed. Test data is synthetic; no test reads live MetaPlan.

Formatting and Clippy with -D warnings passed. Release build passed. Only the known
environment-related incremental-cache hard-link fallback warnings occurred.
The EXE is copied to dist/AttentionFocus. The Windows UI helper now enumerates windows
and captures the app. Launch reported a timeout but window recovery succeeded. Opening
the source panel was visually verified. Two focused type_text attempts produced no
visible text, so no source selection was saved through the UI. Exit succeeded and the
process ended. A bottom background gap observed in the first capture was corrected by
disabling outer scroll-area shrinking; the final fix has not been visually rechecked.
Source entry/refresh, Unicode rendering, startup loading, and tray interaction still
need interactive verification. These limitations remain tracked with T-0002.

## Handoff

Implement T-0004: attributed durable sessions and lifecycle handling. The new preview
does not change the current unassigned timer. See ADR-0007 for source settings isolation.
