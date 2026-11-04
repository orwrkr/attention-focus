# Current Context

Updated: 2026-11-04. History lives in Git and the task cards, and decisions in the ADRs.

## Current state

A local Windows tray application in Rust, using egui/eframe, tray-icon and bundled SQLite.
Current release: 0.19.0, built and verified but not yet run by the owner. Production still
runs 0.18.1 (schema three) until the owner replaces it. Current schema: four. The application owns its project and task list (ADR-0031); MetaPlan
is gone, apart from the conversion of older history in `src/tracking/migrate.rs`.

Recent work:

- T-0031: tray clicks bring the window to the front, with verification (ADR-0028).
- T-0032: report periods follow the current day, with a visible Custom mode.
- T-0033: intervals split at the local day start by default; totals audited (ADR-0029).
- T-0034: first project review and preparation for publishing, in progress.
- T-0035: application-owned projects and tasks, replacing MetaPlan (ADR-0031).
- Second launches show the window, queued commands, small clock corrections (ADR-0030).

Commits are signed with the owner's SSH key through an `includeIf` in the global Git
configuration. Nothing has been pushed; the repository has no remote yet.

## Environment notes

Production runs from a separate folder outside this repository, and is upgraded by the owner.
Never operate it; use Development. Its data directory may be read only for a specific reason,
preferably from a static backup, and such checks must print only counts.

Ask for fresh permission before each computer-use session, even for Development. End it
immediately after UI checks. Never force-kill or close Production. Do not interrupt the
owner's working machine to test power or session events.

## Unverified behavior

- The project panel and note conversion in the running application (T-0035).
- Real tray foreground behavior and a real midnight crossing (T-0031, T-0033).
- A second launch bringing up the running window (ADR-0030).
- Upgrading the real Production database to schema four. A temporary copy was tested.
- The CI workflow, which has never run because nothing has been pushed.
- Multiple monitors and DPI combinations, real sleep/hibernate/logoff/shutdown, clean machines,
  native reduced-motion switching and hardware GPU use (T-0002).

## Exact next step

See [tasks/INDEX.md](../tasks/INDEX.md). The owner checks the items above in Development, or
grants desktop permission for an agent check. Then finish T-0034: choose the repository name,
take a screenshot for the README and push when instructed. The visualization options in
T-0036 are waiting for a decision. T-0019 stays the next larger feature; research
compatibility before any further migration.

Keep one primary task active. Publish only through scripts/build.ps1, preserving the canonical
EXE path; a locked output must fail without alternate copies. Every shipped change requires
a consistent Cargo version, both builds and a freshly verified ZIP and PDF.
