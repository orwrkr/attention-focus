# T-0001: Architecture and First Working Slice

Status: done.
Dependencies: none.

## Goal

Select a minimal Rust stack for a Windows tray application and define a verifiable
path from reading a project to saving a session and displaying a report.

## Scope

UI/tray and storage comparison, module boundaries, packaging, reliability criteria,
and implementation tasks. Full application implementation is out of scope.

## Completed work

Selected egui/eframe, tray-icon, and rusqlite with bundled SQLite in ADR-0004.
Compared Tauri and direct Windows controls, and JSON/text storage alternatives.
Defined source identity, path safeguards, atomic switching, recovery, OS interruption
semantics, and range-based reports in docs/ARCHITECTURE.md.
Created T-0002 through T-0005 with dependencies and acceptance criteria.
Recorded the user's chat-language clarification in ADR-0003.

## Acceptance criteria

- [x] UI/tray and storage choices have rationale, alternatives, and limitations.
- [x] Source reader, sessions, storage, UI, and OS integration modules are defined.
- [x] Time invariants and crash/sleep behavior are documented.
- [x] External storage location and source write-protection checks are defined.
- [x] Small implementation tasks have verifiable outcomes.
- [x] The English-only UI and repository policy is included in implementation planning.
- [x] Context and task index are updated.

## Checks

Reviewed primary documentation linked in ADR-0004. Verified local Rust/Cargo versions
and the MSVC target. All 19 Markdown files passed link and ASCII/LF checks; task
statuses match and git diff --check passed. No executable spike or tests were run in this
architecture-only task; compatibility and OS behavior remain T-0002 acceptance gates.

## Handoff

T-0002 is ready. Prove tray wakeup and bundled SQLite before building domain features.
