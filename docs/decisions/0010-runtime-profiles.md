# ADR-0010: Independent Production and Development

Date: 2026-10-27.
Status: accepted.
Basis: explicit user request.

## Decision

Use LocalAppData/AttentionFocus/Production and /Development. Each holds its own history,
source settings, schema backups, and window state. Debug builds default to Development
and reject explicit Production. Release builds default to Production and can explicitly
select Development. No arbitrary data-directory override is introduced.

Use the resolved profile directory in the Windows instance lock and distinct window/app
identities. Development has a visible label and distinct tray tooltip/menu text. It can
run while production is active. scripts/dev.ps1 builds/runs target/debug; production is
still published only through scripts/build.ps1 to the canonical dist EXE. Building or
running development never replaces that EXE. Locks on the same profile still reject duplicates.

Keep every legacy file directly under AttentionFocus untouched. Read only the old
source setting to seed a new profile. New production history is empty, not a migration
or a destructive reset. Subsequent launches keep its own history. Source selection
validates containment against the shared application root so neither profile can point
MetaPlan at the other profile's data. Linked profile directories are rejected.

Keep the heading and window actions above the scrolling content. Their visibility
does not depend on how many source/report sections are expanded.
