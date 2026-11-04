# Agent Development Contract

## Start of session

Read `README.md`, `docs/CONTEXT.md`, `docs/WORKFLOW.md`, and `tasks/INDEX.md`,
then the selected task card and relevant records in `docs/decisions/`. Read `local/README.md`
if it exists. Check `git status --short`. Preserve unrelated and unfinished work.

## English-only rule

English is the only permitted language for project-authored content: documentation,
agent instructions, task cards, architectural decisions, code comments, identifiers,
test descriptions, synthetic fixtures, commit messages, and the source text of
application-owned UI, errors, logs, and export headings. Write stored development reports
in English. Conversational chat replies may use the user's language; this exception does
not apply to artifacts or content created and stored as part of the project.

The one exception is `locales/*.json`, which holds translations of the interface text, with
`locales/en.json` as the source; see ADR-0035. Add user-visible text there, through `i18n`, and
never directly in code. Do not add other translations or non-English copies without a new user
instruction.

This rule governs authored content, not user data. Preserve Unicode in project names,
task text, notes, and filesystem paths. Never translate, reject, or modify user data
merely because it is not English. Do not copy that data into the repository. If a test
needs non-English text, use Unicode escapes with an English explanation. Do not use
escapes to disguise non-English prose.

## Boundaries

- Build a local Rust application with a Windows system tray as its primary workflow.
- The application owns its project and task list. It does not depend on external sources.
- Keep settings and history in the profile directory. Validate storage paths using path
  normalization and directory link resolution, not just string comparisons.
- This repository is public. Do not commit personal projects, user-specific absolute paths,
  session history, or exports. Use synthetic examples.
- Git ignores `local/`. Keep useful context that does not belong in a public repository there,
  such as machine notes, prepared applications and retired logs, instead of deleting it. Clones
  do not have it, so nothing tracked may depend on it.
- Accepted requirements and proposals are separated in `docs/PRODUCT.md`.
- Research UI, database, and major dependency choices and record the rationale in
  an ADR. Make routine implementation decisions independently within task scope.

## Implementation

- Work on one explicitly selected task; track status and acceptance criteria.
- Make the minimum changes needed to complete it.
- Separate session logic from UI and OS integrations.
- Use temporary directories and synthetic fixtures in tests, never live user data.
- Verify meaningful invariants: no overlapping active sessions, durable history,
  and totals that add up across interval and calendar boundaries.
- Do not introduce cloud services, telemetry, window-content monitoring, or
  productivity scoring without a specific user request.
- Use UTF-8 without BOM and LF line endings. Use English file names and identifiers.
- Publish releases only through scripts/build.ps1 to dist/AttentionFocus/attention-focus.exe.
  The release workflow may then replace that EXE only with its validly signed copy, through
  scripts/use-signed-exe.ps1.
  On a locked EXE, report the error without creating alternate files or directories.
  Never terminate a running app automatically. If you launched it, offer to exit it
  and wait for user approval before closing it. Cargo target files are build intermediates.
- Use Development for agent work and manual test sessions. scripts/dev.ps1 and debug
  builds use LocalAppData/AttentionFocus/Development; the packaged release uses Production.
  Never reset, seed synthetic sessions into, or copy development history into Production.
  Legacy root history is retained untouched. Keep profile instance locks independent.
  Development builds must not replace the production EXE.
- Cargo.toml owns the release version. Bump it before publishing changed production
  inputs for an already archived version. Keep Windows resources, header, guide, manifest,
  and versioned ZIP/PDF names consistent. Never version only the ZIP around a stale EXE.
- Every production update must use scripts/build.ps1: build both EXEs, regenerate the
  guide/notices, and automatically rebuild/verify the production ZIP. PDF-only export
  uses scripts/export-user-guide.ps1, which also refreshes the ZIP. Never leave a stale
  archive after editing something shipped. Run scripts/verify-release.ps1 before handoff.
  Development-only iterations may use scripts/dev.ps1 without touching production.
- Permanent app icons are neutral identity marks, not running-state indicators. Only
  tray icons encode running/stopped; development tray states retain their D badge.
- Treat history conversions as the highest-risk changes: back up first, use one transaction,
  and test upgrades from databases built with each older schema.

## End of session

Update the task card, `tasks/INDEX.md`, and `docs/CONTEXT.md` with completed work,
checks performed, unverified behavior, limitations, and the exact next step.
Store architectural decisions in ADRs. Never claim an unexecuted check passed.

Ordinary local commits of completed work are authorized. Commits are signed; do not bypass
signing. Do not stage unrelated files automatically; inspect the staged diff before
committing. Pushing, rewriting history, deleting unrelated work, and publishing require
explicit user instructions. Execution environment permissions apply independently of
these rules.

Report the outcome, checks, limitations, and any new commit hash. Chat language may
follow the user; saved reports must be in English.
