# Current Context

Updated: 2026-11-09. History lives in Git and the task cards, and decisions in the ADRs.

## Current state

A local Windows tray application in Rust, using egui/eframe, tray-icon and bundled SQLite.
Current release: 0.26.3, the first published release (2026-11-06). Version 0.26.5 holds T-0046
and T-0047; 0.26.4 was built with T-0046 alone and never published. The owner runs 0.26.3 as Production
since that day, and it upgraded the real history to schema five. The application owns its
project and task list (ADR-0031); MetaPlan is gone, apart from the conversion of older history in
`src/tracking/migrate.rs`.

Recent work:

- T-0047: the history uses a write-ahead log with FULL sync: Start and Stop commit in 2 to 4 ms
  instead of about 9 ms, and no longer slow to seconds when memory is short (ADR-0043).
- T-0046: reduced report and snapshot costs as history grows, without a schema change (ADR-0042).
- T-0045: Dependabot and CodeQL set up, and the PDF tools updated for their alerts (ADR-0041).
- T-0044: a Project site link in About, opened in the default browser through the shell (ADR-0040),
  shipped in 0.26.3. The site is live at https://orwrkr.github.io/attention-focus/.
- T-0043: a static project site in `site/`, deployed by GitHub Pages once the repository is public
  (ADR-0040).
- T-0019: edit, delete and add intervals from menus on the timeline and session rows, with an audit
  table and schema five (ADR-0039).
- T-0042: neutral license wording in every language, aligned marks, complete file properties, and a
  notice for the statically linked Microsoft runtime (ADR-0038). MIT stays.
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
configuration, and GitHub shows them as verified. The repository `orwrkr/attention-focus` became
public on 2026-11-06. CI passed there on its first run. Version tags start at
`v0.1.0` and skip 0.20.0 to 0.24.0, whose commits hold the release workflow, so a tag would start a
build. `v0.24.1` was the first release build. Its draft and the `v0.24.2` draft were deleted, and
The `v0.25.2`, `v0.26.1` and `v0.26.2` drafts were deleted as well; their tags remain. `v0.26.3`
was verified and published, unsigned, as the latest release. 0.26.0 was built but never tagged.

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

- A long real history and a multi-year report period in the real UI (T-0046); By day still lays out
  every day each frame and was not measured.
- The theme following a change of the Windows app mode (T-0038).
- Six languages in the real window, the tray in other languages, and typed Arabic or Chinese (T-0039).
- The marks that appear only in the change mode, with no projects or with interrupted time (T-0041).
- The running interval and review outlines on charts in the real window (T-0036).
- The project panel and note conversion in the running application (T-0035).
- Real tray foreground behavior and a real midnight crossing (T-0031, T-0033).
- The real upgrade to schema four is confirmed only by the running version and its backup file.
- Signing in the release workflow (T-0037). Its unsigned path ran for `v0.24.1`, `v0.24.2`,
  `v0.25.2`, `v0.26.1`, `v0.26.2` and `v0.26.3`; `v0.26.3` was published unsigned.
- Whose license covers Visual Studio Enterprise on GitHub's runner (ADR-0038).
- Multiple monitors and DPI combinations, real sleep/hibernate/logoff/shutdown, clean machines,
  native reduced-motion switching and hardware GPU use (T-0002).

## Exact next step

Publish 0.26.5 through the release workflow; the owner approved it on 2026-11-09. See
[tasks/INDEX.md](../tasks/INDEX.md) for T-0034 and T-0037; the SignPath application
is postponed by the owner (2026-11-06). Research compatibility before any further migration.

Keep one primary task active. Publish only through scripts/build.ps1, preserving the canonical
EXE path; a locked output must fail without alternate copies. Every shipped change requires
a consistent Cargo version, both builds and a freshly verified release folder and PDF.
