# T-0034: Public GitHub Release Readiness

Status: in_progress.
Dependencies: T-0035.

## Goal

Prepare the repository to become public on GitHub. This card records the first full project
review from 2026-11-04 and what was done about it. Nothing has been pushed, and no history
was rewritten.

## Review scope

Read all application modules, the main documents, relevant ADRs and the release entry
scripts. Scanned tracked files and all history for personal paths, names, secrets and
committed binaries; the scan found none. The author address in commit metadata is public
by the owner's decision.

## Must resolve before publishing

- [x] **License.** MIT, decided by the owner. `LICENSE`, the Cargo manifest and the top of the
  notices file in every release archive carry it.
- [x] **Commit author email.** Kept, decided by the owner.
- [x] **Commit signing.** Earlier commits are unsigned and stay that way, as rewriting history
  was not requested. New commits are signed with the owner's existing SSH signing key, through
  the same `includeIf` mechanism as the owner's other projects. Local verification shows `G`.
- [x] **MetaPlan dependency.** Removed; see T-0035 and ADR-0031.
- [x] **README.** Rewritten for new readers.
- [ ] **README screenshot.** Needs a desktop capture of Development with synthetic data.
- [x] **Continuous integration.** Added on a Windows runner, with the action pinned by commit.
  It has never run, because nothing has been pushed.
- [x] **Agent process files.** The owner chose to publish them. Trimming proposals are below.

## Should fix

- [x] Cargo manifest `description`, `license` and `rust-version`; toolchain pinned.
- [ ] Cargo manifest `repository`, once the repository name is chosen.
- [x] Corrected stale documents: `docs/ARCHITECTURE.md`, `docs/DEVELOPMENT.md`,
  `docs/PRODUCT.md`, `docs/CONTEXT.md` and the ADR index.
- [x] `SECURITY.md`, `CONTRIBUTING.md`, issue and pull request templates.
- [ ] Code signing (owner decision). Releases are unsigned, so SmartScreen warns. README and
  `SECURITY.md` state this. Free signing for open source projects exists, such as SignPath.
- [x] A second launch activates the running instance; see ADR-0030.
- [x] Commands wait in a queue instead of being dropped; see ADR-0030.
- [x] End reasons for `logoff`, `shutdown` and `controller_lost` have labels.
- [x] Small backward clock corrections no longer interrupt tracking; see ADR-0030.

## Maintainability notes

- [x] `duration` has one implementation.
- [x] `src/tracking.rs` split into storage, migrations, catalog, engine and worker.
- [x] Direct Win32 calls moved from `src/main.rs` to `src/platform.rs`.
- [x] The worker reuses stored state between ticks, and history has an index on `ended_ms`.
- [x] The Calendar fallback month uses the selected time zone.
- [ ] `Desktop` in `src/main.rs` is still large. It has no automated tests to protect a
  restructuring and it would change no behavior, so it is left for when that code next changes.

## Remaining steps to publish

These need the owner, in this order:

1. Choose the repository name, then add `repository` to `Cargo.toml`.
2. Add the SSH signing key to the GitHub account as a signing key, if it is not there yet,
   so signed commits show as Verified. The local token cannot read this setting.
3. Capture the README screenshot.
4. Create the repository, enable private vulnerability reporting, push, and confirm CI passes.
   The tests create real windows and junctions, which has not been tried on a hosted runner.
5. Attach the release archive to a first GitHub release.

## Trimming proposals

For the owner to accept or reject. All content stays available in Git history.

1. **Remove "Verification history" from `docs/DEVELOPMENT.md`** (about 70 lines). These dated
   logs describe old versions and tool failures, and newer records are in task cards.
2. **Condense finished task cards into one `tasks/HISTORY.md`.** The 30 older cards are mostly
   implementation logs. One summary line per task would be easier to read and lose little.
3. **Shorten "Confirmed by the owner" in `docs/PRODUCT.md`.** Much of it repeats the user guide.
4. **Merge ADR-0003 into ADR-0002.** Both cover the language rule.

`docs/CONTEXT.md` was already cut from about 210 lines to about 50, because most of it had to
change for T-0035 and `docs/WORKFLOW.md` requires it to stay concise.
ADRs should stay: they are the most useful of these records for new readers.

## Handoff

Do not push or change visibility without an explicit user instruction.
