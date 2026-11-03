# T-0034: Public GitHub Release Readiness

Status: backlog.
Dependencies: owner decisions marked below.

## Goal

Prepare the repository to become public on GitHub. This card records the first full
project review from 2026-11-04. Nothing here has been published, and no history was changed.

## Review scope

Read all application modules, the main documents, relevant ADRs and the release entry
scripts. Scanned tracked files and all history for personal paths, names, secrets and
committed binaries. Baseline before changes: 81 tests, formatting and Clippy passed.

## What is already in good shape

- History scan found no personal paths, secrets, databases, exports or binaries.
- `.gitignore` covers build output, local data, databases and environment files.
- MetaPlan access is read-only, with tests for links, junctions and storage overlap.
- Timing logic uses injected clocks; tests use temporary synthetic data.
- Third-party notices are generated, and packaging fails if a license input is missing.
- No telemetry, network access or cloud dependency in the application.

## Must resolve before publishing

- [ ] **License (owner decision).** No LICENSE file exists and Cargo.toml has no `license`
  field. A public repository without a license grants others no right to use the code.
- [ ] **Commit author email (owner decision).** All 31 existing commits show the author's
  personal address. Rewriting history is only practical before the first push, and needs
  explicit approval. Otherwise accept it, or use a GitHub no-reply address from now on.
- [ ] **MetaPlan dependency.** Project selection only works with one private directory
  format whose literal headings are non-English (`src/source.rs:11-15`). README never says
  what MetaPlan is. Without it, users only get Unassigned focus. At minimum document the
  format with a synthetic example, and state that the app works without a source.
- [ ] **README.** It reads as accumulated release notes. It needs a short description,
  a screenshot, Windows-only support, download and build steps, and links to the guide.
- [ ] **Continuous integration.** There is no `.github/workflows`. Run formatting, Clippy
  and tests on a Windows runner. Tests create junctions, HKCU test keys and real windows.
- [ ] **Agent process files (owner decision).** `AGENTS.md`, `docs/CONTEXT.md`, the task
  cards and verification logs narrate agent sessions. They contain no secrets found by the
  scan, but decide whether to keep, trim or move them before publishing.

## Should fix

- [ ] Add Cargo.toml `description`, `repository` and `rust-version`, and pin the toolchain.
  `docs/DEVELOPMENT.md:5` states 1.98.0 while the local toolchain is 1.99.1.
- [ ] Correct stale documents: the module table in `docs/ARCHITECTURE.md:12` names modules
  that do not exist, `docs/ARCHITECTURE.md:71` says there is no export, and
  `docs/DEVELOPMENT.md` still lists export and autostart as deferred.
- [ ] Add SECURITY and contribution notes, and issue templates if contributions are wanted.
- [ ] Decide on code signing: an unsigned EXE triggers Windows SmartScreen warnings.
- [ ] A second launch shows an error box instead of activating the running instance.
- [ ] `Desktop::command` silently drops commands while another is pending, including tray
  Exit. The user gets no feedback, and the action must be repeated.
- [ ] Report end reasons have no labels for `logoff`, `shutdown` and `controller_lost`,
  so they all read as Tracking interrupted (`src/report_ui.rs`).
- [ ] Any backward wall-clock step, even one millisecond, interrupts tracking for review,
  although forward drift tolerates five seconds. Decide whether this strictness is intended.

## Maintainability notes

- `duration` is implemented three times: `src/main.rs`, `src/report_ui.rs` and
  `src/presentation.rs`.
- `src/tracking.rs` combines migrations, storage, engine and worker. `src/main.rs` holds a
  large `Desktop` state and direct Win32 calls. Consider splitting along documented boundaries.
- The worker reruns aggregate queries every second while tracking, and report queries scan
  the whole table. Consider an index at the next schema change, likely T-0019.
- The Calendar fallback month uses the UTC date, not the selected time zone
  (`src/date_picker.rs:52`). It only applies when the date field is invalid.

## Handoff

Get the three owner decisions first, then do the rest as separate small tasks. Do not push
or change visibility without an explicit user instruction.
