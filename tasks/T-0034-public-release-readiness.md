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

- [x] **License.** MIT, decided by the owner. `LICENSE`, the Cargo manifest and the licenses
  shown inside the application carry it.
- [x] **Commit author email.** Kept, decided by the owner.
- [x] **Commit signing.** Earlier commits are unsigned and stay that way, as rewriting history
  was not requested. New commits are signed with the owner's existing SSH signing key, through
  the same `includeIf` mechanism as the owner's other projects. Local verification shows `G`.
- [x] **MetaPlan dependency.** Removed; see T-0035 and ADR-0031.
- [x] **README.** Rewritten for new readers.
- [x] **README screenshot.** Two captures of Development with synthetic data, in `docs/images`.
- [x] **Continuous integration.** Added on a Windows runner, with the action pinned by commit. The
  first run on GitHub, on 2026-11-04, passed, including the tests with real windows and junctions.
- [x] **Agent process files.** The owner chose to publish them. Trimming proposals are below.

## Should fix

- [x] Cargo manifest `description`, `license` and `rust-version`; toolchain pinned.
- [x] Cargo manifest `repository`. The owner agreed on the name `attention-focus`.
- [x] Corrected stale documents: `docs/ARCHITECTURE.md`, `docs/DEVELOPMENT.md`,
  `docs/PRODUCT.md`, `docs/CONTEXT.md` and the ADR index.
- [x] `SECURITY.md`, `CONTRIBUTING.md`, issue and pull request templates.
- [x] Code signing. The owner chose the free SignPath Foundation program. Everything possible
  before publishing is prepared; the rest waits on a first public release. See T-0037.
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

The repository `orwrkr/attention-focus` became public on 2026-11-06, by the owner's decision. A
scan of all history for personal paths and addresses before the first push found nothing.

1. Done: GitHub reports signed commits as verified.
2. Done: the repository is public, and the owner enabled private vulnerability reporting, Dependabot
   and CodeQL; the first alerts are handled in T-0045.
3. Done: the site link is in 0.26.3 (T-0044).
4. Done: `v0.26.3` was checked and published on 2026-11-06 as the first public release.
5. Only then set Settings, Pages, Source to GitHub Actions, and run the Pages workflow (T-0043).
6. Continue with the code signing steps in T-0037.

## Trimming

The owner agreed to shorten the logs and asked to keep useful context out of the repository
rather than delete it, so Git now ignores a `local/` directory for this; see AGENTS.md.

- **Done:** "Verification history" moved from `docs/DEVELOPMENT.md` to `local/archive/`.
- **Done:** "Confirmed by the owner" in `docs/PRODUCT.md` shortened; the full list moved there too.
  Both also remain in Git history.
- **Not done:** condensing finished task cards, and merging ADR-0003 into ADR-0002. Neither was
  recommended: the cards and ADRs are the records that new readers can actually use.

`docs/CONTEXT.md` was already cut from about 210 lines to about 50 during T-0035.

## Handoff

Do not push or change visibility without an explicit user instruction.
