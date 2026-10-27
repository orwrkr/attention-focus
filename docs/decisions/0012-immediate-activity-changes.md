# ADR-0012: Immediate Activity Changes and Recent Pickers

Date: 2026-10-27.
Status: accepted.
Basis: explicit user request; supersedes the draft-only project/task picker in
[ADR-0008](0008-attributed-session-engine.md).

## Decision

Label the header as the confirmed tracking activity while running, or the saved next
activity while stopped. Only a committed worker snapshot updates it. Keep a distinct
draft for notes and unsuccessful changes; expose pending and retry states.

A project/task picker change sends ChangeActivity immediately. While stopped this only
saves the next selection. While running, the default NewInterval mode closes the old
interval and opens the next at the same wall/monotonic boundary using the existing
atomic switch. Selecting the same project preserves its task and does nothing.

An opt-out checkbox selects EditCurrent mode, persisted independently per profile.
It explicitly reassigns the entire active interval, including elapsed time. In one
transaction update only its activity/checkpoint, the saved selection, and usage. Keep
its ID, original start, and monotonic timer. Finalized/review history remains unchanged.
This is a deliberate exception to active-snapshot immutability in ADR-0008; it does not
add general history correction. Never restart after a lifecycle interruption or clock
gap merely because a queued picker change arrives. Recovery still requires review.
Notes alone remain draft until explicitly applied or included in another picker change.

Sort projects and tasks by last successful tracking use, newest first. A monotonic
database sequence avoids clock-change ambiguity. Opening/searching a picker or choosing
an activity while stopped does not count as use. Track Unassigned and No task too.
Break unused/tied choices alphabetically and deterministically. Identity is source ID,
canonical source path, slug, and task text, excluding display title and note. Project
rank aggregates all of its tasks. Different sources never share recency.

Schema version two adds preferences and keyed recent-use rows. Before migrating a
schema-one database, use SQLite backup to create focus-before-v2-*.sqlite3 in its own
profile. Seed recent use from existing session ID order once, preserving sessions and
selection. Migration and backfill are transactional. Starting, switching, and reassigning
update recency in the same transaction as history, so failed writes cannot reorder
choices or confirm a different activity. Older EXEs reject the newer schema.

## Verification

Use temporary synthetic databases and clock-injected commands for shared boundaries,
duration preservation, finalized-history isolation, write rollback, recovery safety,
source-scoped ordering, restart persistence, and backup/backfill. Real desktop behavior
remains a separate interactive check. Never exercise synthetic tracking in Production.
