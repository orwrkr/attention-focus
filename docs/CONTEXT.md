# Current Context

Updated: 2026-11-04. History lives in Git and the task cards, and decisions in the ADRs.

## Current state

A local Windows tray application in Rust, using egui/eframe, tray-icon and bundled SQLite.
Current release: 0.20.0, built and verified but not yet run by the owner, and it keeps schema
four. Production still runs 0.19.0 until the owner replaces it. The application owns its project
and task list (ADR-0031); MetaPlan is gone, apart from the conversion of older history in
`src/tracking/migrate.rs`.

Recent work:

- T-0036: timeline, by-day and weekly rhythm charts in reports (ADR-0032).
- T-0037: release workflow and preparation for free signing through SignPath Foundation
  (ADR-0033). Blocked until the repository is public and has a first release.
- T-0034: preparation for publishing. What remains needs the owner.
- T-0035: application-owned projects and tasks, replacing MetaPlan (ADR-0031).
- T-0033: intervals split at the local day start by default; totals audited (ADR-0029).
- T-0032: report periods follow the current day, with a visible Custom mode.
- T-0031: tray clicks bring the window to the front, with verification (ADR-0028).
- Second launches show the window, queued commands, small clock corrections (ADR-0030).

Commits are signed with the owner's SSH key through an `includeIf` in the global Git
configuration. Nothing has been pushed; the repository has no remote yet. The agreed repository
is `orwrkr/attention-focus`, and it has not been created.

The ignored `local/` directory holds useful context that does not belong in a public repository:
machine notes, prepared application answers and retired logs. Clones do not have it, and nothing
tracked depends on it.

## Environment notes

Production runs from a separate folder outside this repository, and is upgraded by the owner.
Never operate it; use Development. Its data directory may be read only for a specific reason,
preferably from a static backup, and such checks must print only counts.

Ask for fresh permission before each computer-use session, even for Development. End it
immediately after UI checks. Never force-kill or close Production. Do not interrupt the
owner's working machine to test power or session events.

## Unverified behavior

- The charts in the running application (T-0036). Their shapes were rendered to images instead.
- The project panel and note conversion in the running application (T-0035).
- Real tray foreground behavior and a real midnight crossing (T-0031, T-0033).
- A second launch bringing up the running window (ADR-0030).
- The real upgrade to schema four is confirmed only by the running version and its backup file.
- The CI and release workflows, which have never run because nothing has been pushed (T-0037).
- Multiple monitors and DPI combinations, real sleep/hibernate/logoff/shutdown, clean machines,
  native reduced-motion switching and hardware GPU use (T-0002).

## Exact next step

See [tasks/INDEX.md](../tasks/INDEX.md). The owner checks the items above in Development, or
grants desktop permission for an agent check, and takes a screenshot for the README. Then finish
T-0034 when instructed: create the repository, push, confirm CI, and publish a first release from
a tag. After that, T-0037 lists the steps to apply for signing. T-0019 stays the next larger
feature; research compatibility before any further migration.

Keep one primary task active. Publish only through scripts/build.ps1, preserving the canonical
EXE path; a locked output must fail without alternate copies. Every shipped change requires
a consistent Cargo version, both builds and a freshly verified ZIP and PDF.
