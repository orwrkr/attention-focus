# T-0003: Read-Only Source and Settings

Status: backlog.
Dependencies: T-0002.

## Goal

Deliver read-only source and settings within the initial architecture.

## Scope

Follow [the architecture](../docs/ARCHITECTURE.md) and ADR-0004. Keep all authored
content and UI in English. Preserve external Unicode data and the read-only boundary.
Do not include later tasks or deferred product features.

## Plan

Implement the acceptance criteria in order, then verify and update project context.

## Acceptance criteria

- [ ] Implement source selection and fixed external storage with canonical path overlap checks before any writes.
- [ ] Read project names, status, priority, and nonempty unfinished current-cycle tasks from synthetic MetaPlan fixtures.
- [ ] Handle missing files, duplicate slugs, archived projects, and malformed data without writing to the source.
- [ ] Preserve Unicode through escaped protocol tokens and synthetic test data; keep authored text English.
- [ ] Test source tree byte-for-byte integrity, nesting, sibling prefixes, case aliases, and supported junction cases.
- [ ] Persist the source selection outside MetaPlan and restore it safely on restart.

## Checks

Not started. Record commands, results, and manual evidence here.

## Handoff

Start after T-0002 is complete. Update this card and the task index together.
