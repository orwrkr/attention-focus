# T-0001: Architecture and First Working Slice

Status: ready.
Dependencies: none.

## Goal

Select a minimal Rust stack for a Windows tray application and define a verifiable
path from reading a project to saving a session and displaying a report.

## Scope

Compare UI/tray and storage options, define module boundaries, packaging, reliability
criteria, and implementation tasks. Full application implementation is out of scope.

## Plan

1. Check current official documentation and required scenario support.
2. Compare native Rust UI and Tauri for complexity, distribution, and tray behavior.
3. Select storage and safeguards for the read-only MetaPlan boundary.
4. Define timer states, recovery, and source/project identity.
5. Write ADRs and ready task cards for the first working slice.

## Acceptance criteria

- [ ] UI/tray and storage choices have rationale, alternatives, and limitations.
- [ ] Source reader, sessions, storage, UI, and OS integration modules are defined.
- [ ] Time invariants and crash/sleep behavior are documented.
- [ ] External storage location and source write-protection checks are defined.
- [ ] Small implementation tasks have verifiable outcomes.
- [ ] The English-only UI and repository policy is included in implementation planning.
- [ ] Context and task index are updated.

## Checks

Not performed; the task has not started.

## Handoff

Start with `docs/PRODUCT.md` and ADR-0001/0002.
Do not copy real MetaPlan data into fixtures.
