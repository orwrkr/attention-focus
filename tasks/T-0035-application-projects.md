# T-0035: Application-Owned Projects and Tasks

Status: done.
Dependencies: T-0033.

## Goal

Remove the dependency on the external MetaPlan directory and keep the project and task list
in the application, without losing or changing existing history.

## Scope

Schema four, conversion of existing history, the project list and its interface, and the
removal of the reader, source settings and folder picker. See ADR-0031.

## Acceptance criteria

- [x] Projects and tasks can be added, including several at once, and survive restart.
- [x] Projects can be renamed and archived or restored; tasks can be finished or reopened.
- [x] Only never-tracked projects and tasks can be deleted.
- [x] A rename moves the selection and running interval but not finished sessions.
- [x] Report totals show current names; session rows keep recorded names.
- [x] List changes leave the timer, history and recent-use order untouched.
- [x] Existing history converts with identical times and totals, after a backup.
- [x] A failed conversion leaves the previous schema and rows untouched.
- [x] Note suggestions saved by older versions follow their converted projects.
- [x] No reader code, source settings or source fields remain outside the conversion.
- [ ] Native interface check, see Handoff.

## Checks

`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and
`cargo test --locked` (127 tests) passed. Upgrade tests build older databases from their
original table definitions and activity format, rather than from today's schema.

The upgrade was also run once on temporary copies of the real Production backup (schema
two) and the Development database, through a throwaway test that printed only counts and was
not committed. The copies were deleted afterward, and the originals were not changed.

| Copy | Rows | Saved ms | Times unchanged | Unconverted | Projects |
| --- | --- | --- | --- | --- | --- |
| Production backup | 148 to 148 | 85549695 to 85549695 | yes | 0 | 2 |
| Development | 11 to 11 | 348684 to 348684 | yes | 0 | 1 |

Unverified: the project panel in the running application. It has headless pointer tests,
but `src/main.rs` integration, including the note conversion on first launch, has no
automated test.

## Handoff

Check in Development: the converted list, adding several projects, rename while tracking,
archive and restore, and finishing a task. Projects never tracked before do not carry over.
