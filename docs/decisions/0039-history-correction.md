# ADR-0039: Correcting History by Hand

Date: 2026-11-05.
Status: accepted.
Basis: explicit user request for a context menu on chart intervals and session rows, with delete after
confirmation, and create and edit through a dialog; extends ADR-0008, ADR-0029 and ADR-0030. T-0019.

## Context

History has only ever been written by the tracker. Reports, Today and the charts trust `focus_sessions`,
and starting a new interval rejects a clock earlier than the latest recorded end. Hand edits must not
break either, and a person must be able to see what was changed if a total looks wrong later.

## Decision

**What can change.** Only finished intervals (`state='stopped'`). A running interval, and an interrupted
one awaiting review, are protected: the menu shows why, and the worker refuses them as well. Adding an
interval is always possible, as long as it passes the rules below.

**Validation lives in the worker**, inside the transaction, and the interface only previews:

- The end is after the start, and is not later than now. A future end would block every later start.
- An interval whose boundaries are new or changed must not overlap another one. Touching is allowed.
  Running time counts up to now, interrupted time counts, and rows without positive length do not. The
  rejection names the interval in the way, and the dialog stays open.
- There is no automatic trimming or moving of neighbours: changing another interval silently is the
  kind of surprise that history must not have.
- Changing only the project, task or note skips the overlap check, so an older overlap left by a clock
  correction does not block it.

**Precision.** A boundary whose text was not edited keeps its exact stored millisecond. An edited
boundary comes from `HH:MM` or `HH:MM:SS` in the report's selected time zone, and `24:00` is allowed for
an end. A local time that a daylight-saving change skips or repeats is rejected with a message, as in
report filters. Seconds are accepted because recorded intervals have them, so a person can make an
interval touch its neighbour exactly.

**Duration.** When a boundary changes, `elapsed_ms` becomes the wall-clock length, and any earlier
difference from a clock correction ends there. When neither changes, `elapsed_ms` stays. A manual row has
`end_reason='manual'`; an edited row keeps its reason.

**Activity.** An unchanged activity keeps its recorded snapshot, even for a renamed, archived or deleted
project. A changed one must name a project and task in the list, including archived projects and finished
tasks, because history may correct what was true then. Editing history does not change the current
selection or recent use, and does not count as a new use.

**Midnight.** A manual interval may cross a day start and stays one row, whatever the split option says.
Reports already divide such rows by day. A chart draws it in parts, and the menu on any part acts on the
whole interval, which the dialogs state by showing full dates.

**Audit.** Schema five adds `focus_session_edits`, with the session ID, action, time, and the row before
and after as JSON. It is written in the same transaction as the change. A deleted interval therefore
stays in the database file, without a viewer in the application. Upgrading makes a backup first, like
earlier schema changes, in one transaction.

**Interface.** The timeline's menu and each session row's menu offer Edit, Delete and Add; empty space on
the timeline offers Add with that day and time filled in. The target is fixed when the menu opens, not
followed by the pointer. Aggregated chart views have no menu, because a column is not one interval.
Expanded session rows and the section itself also have ordinary buttons, because a context menu cannot be
reached by keyboard here. Edits go through the same one-at-a-time outbox as other commands.

**Stale views.** `focus_sessions.id` can be reused after the newest row is deleted, and a report on screen
can be briefly out of date. So a command carries the boundaries that the interface showed, and the worker
refuses a row with other boundaries as no longer existing, instead of changing a different interval.

## Alternatives

- Soft deletion with a flag on the session row: every query in reports, Today and the catalog would have
  to respect it, and one that forgets changes totals. The audit table keeps the same information apart.
- Keeping schema four and adding the table quietly: an older EXE could then keep writing to a history
  whose edits it does not understand, with no way to tell. A version bump is the existing, clear signal.
- Allowing overlaps with a warning: totals across projects could then exceed real time, which the project
  verifies against.
- Splitting manual intervals at midnight: more rows and more audit entries for what reports already
  handle, and a single edit would then have to manage several rows.

## Consequences

An older EXE refuses a history upgraded to schema five with the existing "newer history" error. Going back
needs the `focus-before-v5-` backup, and loses anything recorded since. Production upgrades only when the
owner replaces its EXE; the Development profile is separate. There is no undo in the application and no
"edited" mark on rows, both of which the audit table would allow later. Exports show the corrected values,
with `manual` as a new end reason.
