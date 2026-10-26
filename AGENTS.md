# Agent Development Contract

## Start of session

Read `README.md`, `docs/CONTEXT.md`, `docs/WORKFLOW.md`, and `tasks/INDEX.md`,
then the selected task card and relevant records in `docs/decisions/`.
Check `git status --short`. Preserve unrelated and unfinished work.

## English-only rule

English is the only permitted language for project-authored content: documentation,
agent instructions, task cards, architectural decisions, code comments, identifiers,
test descriptions, synthetic fixtures, commit messages, and application-owned UI
text, errors, logs, and export headings. Write stored development reports in English.
Conversational chat replies may use the user's language; this exception does not
apply to artifacts or content created and stored as part of the project.
Do not add translations or non-English copies without a new user instruction.

This rule governs authored content, not external data. Preserve Unicode in imported
MetaPlan names, task text, user notes, and filesystem paths. Never translate, reject,
or modify external data merely because it is not English. Do not copy that data into
the repository. Document external format labels in English; if exact non-English
protocol tokens are necessary in code, represent them using Unicode escapes and
explain their meaning in English. Do not use escapes to disguise non-English prose.

## Boundaries

- Build a local Rust application with a Windows system tray as its primary workflow.
- MetaPlan is an external, read-only source; the path to `Instance` is configurable.
- Never modify the source through the application, migrations, tests, scripts, or exports.
- Keep settings and history outside the source. Validate storage paths using path
  normalization and directory link resolution, not just string comparisons.
- Do not commit personal projects, user-specific absolute paths, or session history.
  Use synthetic examples.
- Accepted requirements and proposals are separated in `docs/PRODUCT.md`.
- Research UI, database, and major dependency choices and record the rationale in
  an ADR. Make routine implementation decisions independently within task scope.

## Implementation

- Work on one explicitly selected task; track status and acceptance criteria.
- Make the minimum changes needed to complete it.
- Separate session logic and MetaPlan reading from UI and OS integrations.
- Use temporary directories and synthetic fixtures in tests, never live MetaPlan data.
- Verify meaningful invariants: no overlapping active sessions, durable history,
  and no writes to the source.
- Do not introduce cloud services, telemetry, window-content monitoring, or
  productivity scoring without a specific user request.
- Use UTF-8 without BOM and LF line endings. Use English file names and identifiers.

## End of session

Update the task card, `tasks/INDEX.md`, and `docs/CONTEXT.md` with completed work,
checks performed, unverified behavior, limitations, and the exact next step.
Store architectural decisions in ADRs. Never claim an unexecuted check passed.

Ordinary local commits of completed work are authorized. Do not stage unrelated
files automatically; inspect the staged diff before committing. Pushing, rewriting
history, deleting unrelated work, and publishing require explicit user instructions.
Execution environment permissions apply independently of these rules.

Report the outcome, checks, limitations, and any new commit hash. Chat language may
follow the user; saved reports must be in English.
