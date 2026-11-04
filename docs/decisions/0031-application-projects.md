# ADR-0031: Application-Owned Projects and Tasks

Date: 2026-11-04.
Status: accepted.
Basis: explicit user request. Supersedes the read-only source integration in ADR-0001 and
ADR-0007, and the source identity parts of ADR-0009, ADR-0010, ADR-0012 and ADR-0017.

## Context

Project selection depended on one private directory format with non-English literal
headings, so nobody else could use it without recreating that format. The owner wants to
remove this dependency before publishing and keep the project list in the application.

## Decision

The history database owns the list. Schema four adds `focus_projects` (title, archived) and
`focus_tasks` (project, title, done). The reader, source settings and folder picker are
removed. `source.sqlite3` files from older versions are left in place, unread.

- **Identity:** a project is identified by a permanent numeric ID. Activity snapshots store
  that ID with the title and task text as recorded. Tasks stay identified by their text.
- **Names:** titles keep any language and characters, with surrounding space and control
  characters removed, up to 200 characters. Project names and task names within a project
  must differ by more than case, and archived names stay reserved so restoring is safe.
- **Rename:** the saved selection and a running interval follow the new title. Finished
  sessions keep what they recorded. Report totals show the current title, session rows the
  recorded one, so a rename neither rewrites history nor splits totals.
- **Archive and finish:** these only stop a project or task from being offered for new
  intervals. A running interval may finish on it, and its history stays in reports.
- **Delete:** allowed only when nothing was recorded, and not for the saved selection.
  This keeps every session's project ID meaningful.
- **Lists:** adding accepts several names, one per line, in one transaction. Names that
  already exist are skipped, and a finished task is reopened. Adding only existing names is
  an error, so a single addition cannot silently do nothing.
- **Exports:** session rows gain Project ID and lose source ID, source path and slug.
  Destinations are now only checked against application storage.

The upgrade backs up to `focus-before-v4-*.sqlite3` and converts in one transaction. It walks
sessions in recorded order, then the selection, creating one project per former identity
(source ID, source path, folder name) with its newest recorded title and every recorded
task. Recent-use order carries over. A name without any session or selection is dropped,
since it has no title to show. `focus_project_aliases` keeps former identities only so
note suggestions saved in preferences can move to the new IDs on first launch. A
malformed session fails the upgrade and leaves schema three, as earlier upgrades did.

An index on `ended_ms` is added in the same upgrade, which lets report and daily queries
skip finished history outside their range.

## Alternatives

- Keep the reader as an optional import: this keeps a private format in a public project
  for the benefit of one existing user, whose history converts anyway.
- Treat application projects as a special source: minimal code change, but every identity,
  report and export would keep carrying meaningless source fields.
- Rewrite finished sessions on rename: simpler totals, but it destroys what was recorded.
- Identify tasks by ID as well: more correct for renames, but tasks cannot be renamed yet,
  so it would add a second conversion without a visible benefit today.

## Consequences

Projects that were listed in the former source but never tracked do not carry over; add
them by pasting a list. Archive state from the former source is not known either, so
converted projects start active. Older EXEs refuse a schema-four database, and returning
to one requires restoring the backup, losing sessions recorded since. Former source
paths remain in the local alias table. Older decision records and task cards still describe
the source integration as history. T-0019 can rely on project IDs for corrections.
