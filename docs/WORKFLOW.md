# Development Workflow

## Task cycle

1. **Restore context.** Read the entry documents and inspect the working tree.
2. **Select a task.** Choose a ready task, check dependencies, and set its status to
   `in_progress`. Keep one primary task active at a time.
3. **Define the outcome.** Record scope, acceptance criteria, and a short plan.
   Ask the user only about material ambiguity that context cannot resolve.
   Do not require approval at every transition.
4. **Implement.** Deliver the bounded scope. Record significant architectural
   choices in ADRs; do not disguise product expansion as a technical decision.
5. **Verify.** Review criteria and the diff, then run checks appropriate to the change.
   Once a Cargo project exists, use `cargo fmt --check`, `cargo test`, and
   `cargo clippy --all-targets -- -D warnings`. Document any architecture-specific
   alternatives. Manually verify affected UI scenarios.
   For documentation, check links, task status consistency, English-only compliance,
   encoding, and `git diff --check`. Review prose as well as scanning scripts:
   Latin characters alone do not prove that content is English.
   For production changes, bump an already archived Cargo version, then use
   scripts/build.ps1 to build both EXEs and refresh the guide/notices/ZIP automatically.
   Run scripts/verify-release.ps1 and review all PDF pages when the guide/layout changes.
   Use export-user-guide.ps1 for PDF re-export with automatic archive refresh. Never
   hand off a production change with an outdated archive. See ADR-0011.
6. **Save the result.** Update the task card, index, and context. Commit only relevant
   files locally, then report the result and limitations.

## Statuses

`backlog` -> `ready` -> `in_progress` -> `done`.

Use `blocked` for a concrete external dependency, recording the reason and the
condition for resuming work. Incomplete implementation alone is not a blocker.
Keep task card and index statuses consistent.

## Definition of done

- Acceptance criteria are met without accidental scope expansion.
- Checks and results are recorded; unverified scenarios are explicit.
- No personal data, secrets, time databases, or generated builds are committed.
- All authored content and application UI follow the English-only rule in AGENTS.md.
- Another agent can resume without access to chat history.

Keep `docs/CONTEXT.md` concise and current. Git and task cards hold implementation
history; ADRs hold decision rationale.
