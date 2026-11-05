# Current Context

Updated: 2026-11-05. History lives in Git and the task cards, and decisions in the ADRs.

## Current state

A local Windows tray application in Rust, using egui/eframe, tray-icon and bundled SQLite.
Current release: 0.24.1, built and verified but not yet run by the owner, and it keeps schema
four. Production runs an earlier version until the owner replaces it. The application owns its
project and task list (ADR-0031); MetaPlan is gone, apart from the conversion of older history in
`src/tracking/migrate.rs`.

Recent work:

- T-0041: an About button, and "i" marks with balloons in place of explanatory text (ADR-0037). Use
  `ui_support::with_info` for a new explanation.
- T-0040: a release is one EXE that holds its licenses and shows them, with no ZIP (ADR-0036). An
  installer is deferred.
- T-0039: thirteen interface languages including Arabic (ADR-0035). Text is composed in logical
  order and painted for egui at the last moment; see `src/i18n.rs` before touching interface text.
- T-0038: one scroll bar style, light, dark and system themes, the interval option in Settings
  (ADR-0034), checked in the real window.
- T-0036: timeline, by-day and weekly rhythm charts in reports (ADR-0032).
- T-0037: release workflow and preparation for free signing through SignPath Foundation
  (ADR-0033). Blocked until the repository is public and has a first release.
- T-0034: preparation for publishing. What remains needs the owner.
- T-0035: application-owned projects and tasks, replacing MetaPlan (ADR-0031).
- T-0033: intervals split at the local day start by default; totals audited (ADR-0029).
- Second launches show the window, queued commands, small clock corrections (ADR-0030).

Commits are signed with the owner's SSH key through an `includeIf` in the global Git
configuration. The repository `orwrkr/attention-focus` exists on GitHub and is private for now,
and the owner plans to make it public. CI passed there on its first run. Version tags start at
`v0.1.0` and skip 0.20.0 to 0.24.0, whose commits hold the release workflow, so a tag would start a
build. `v0.24.1` is the first release build.

The ignored `local/` directory holds useful context that does not belong in a public repository:
machine notes, prepared application answers, helper tools and retired logs. Clones do not have
it, and nothing tracked depends on it.

## Environment notes

Production runs from a separate folder outside this repository, and is upgraded by the owner.
Never operate it; use Development. Its data directory may be read only for a specific reason,
preferably from a static backup, and such checks must print only counts.

Ask the owner before each session of desktop work and screenshots, and whether right now is a good
moment, even for Development. End it immediately after that set of actions. Never force-kill or
close Production. Do not interrupt the owner's working machine to test power or session events, and
do not change the Windows app mode.

## Unverified behavior

- The theme following a change of the Windows app mode (T-0038).
- Six languages in the real window, the tray in other languages, and typed Arabic or Chinese (T-0039).
- Byte-identical notices on a hosted runner, shown by the next CI run (T-0040).
- The marks that appear only in the change mode, with no projects or with interrupted time (T-0041).
- The running interval and review outlines on charts in the real window (T-0036).
- The project panel and note conversion in the running application (T-0035).
- Real tray foreground behavior and a real midnight crossing (T-0031, T-0033).
- The real upgrade to schema four is confirmed only by the running version and its backup file.
- The release workflow, which needs a version tag (T-0037).
- Multiple monitors and DPI combinations, real sleep/hibernate/logoff/shutdown, clean machines,
  native reduced-motion switching and hardware GPU use (T-0002).

## Exact next step

See [tasks/INDEX.md](../tasks/INDEX.md). Publishing is the owner's decision: T-0034 and then T-0037.
T-0019 stays the next larger feature; research compatibility before any further migration.

Keep one primary task active. Publish only through scripts/build.ps1, preserving the canonical
EXE path; a locked output must fail without alternate copies. Every shipped change requires
a consistent Cargo version, both builds and a freshly verified release folder and PDF.
