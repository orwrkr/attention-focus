# ADR-0007: Source Settings and Read-Only Preview

Date: 2026-10-27.
Status: accepted.

## Decision

Implement T-0003 under the user's instruction to continue development. Interactive
T-0002 validation remains pending; it does not prevent isolated reader and boundary tests.

Store source identities and the selected identity transactionally in source.sqlite3
beside focus.sqlite3. This avoids a migration of existing focus history before project
attribution is implemented. Read this settings database read-only at startup and validate
the selected source before initializing either writable history or window persistence.
This refines the settings location in ADR-0004; it does not change the storage root.

Resolve existing ancestors, junctions, and case aliases and reject containment in either
direction. Reject linked entries inside application storage and linked source containers,
projects, or Markdown files. The explicitly selected source root may itself be resolved.
Concurrent malicious replacement of filesystem links remains outside the threat model.

Read project metadata and current-cycle task snapshots on a background thread. Keep
diagnostics per project; exclude duplicate slugs across Active and Archive. A missing
tasks.md does not hide an otherwise valid project. Do not follow task text as paths.
Use escaped literals only for external protocol labels; preserve imported Unicode text.

## Consequences

The source is configured with an absolute path field and Save and refresh. Loading is
explicit, with an automatic refresh at startup. Source failure does not stop the timer.
Projects and tasks are a read-only preview in this step; sessions remain unassigned.
Changing to another canonical source path creates an independent local identity.
Reusing a previously selected path restores its identity. Moved-source remapping is deferred.
No changes to focus.sqlite3 schema, reports, or project-attributed sessions are included.
