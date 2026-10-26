# ADR-0002: English-Only Project and Application UI

Date: 2026-10-26.
Status: accepted.
Basis: explicit user instructions to translate the project, allow only English,
and clean the repository, recreating it if necessary.

## Context

The initial documentation was written in Russian. The repository contains only
one local initial commit, has no remote, and has no unrelated uncommitted changes.
Keeping that commit would retain non-English project content in history.

## Decision

Translate all authored content into English and require English for future project
content, development reports, commit messages, and application-owned UI strings.
Recreate local Git metadata and create a clean initial commit after validation.
Do not preserve the superseded history in repository branches, tags, or reflogs.

Keep external MetaPlan data read-only and preserve Unicode values. The English-only
policy is not an instruction to translate user data or limit accepted characters.
Document source format semantics in English. Exact foreign-language protocol tokens,
if required by a future parser, can use Unicode escapes with English explanations.

## Alternatives

- Adding a translation commit leaves the original content in history.
- Keeping bilingual documentation conflicts with the user's language rule.
- Translating MetaPlan violates the source boundary and is outside this request.

## Consequences

Old local commit IDs become obsolete. The repository starts with an English-only
baseline. Future reviews must check language as well as functionality; script checks
are useful but cannot identify every non-English sentence written in Latin letters.
