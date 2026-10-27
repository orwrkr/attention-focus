# Initial Architecture

Status: target design with desktop persistence, read-only source settings, and
attributed sessions implemented. See [ADR-0008](decisions/0008-attributed-session-engine.md)
for the clock-injected engine and Windows lifecycle integration. ADR-0009 adds date-range
reports and timezone boundaries. Recovery adjustment remains target work. Recovery currently
supports accepting the last durable checkpoint; editing and discarding time are deferred.
Decision: [ADR-0004](decisions/0004-desktop-stack.md).

## Modules and ownership

| Module | Responsibility | Boundary |
| --- | --- | --- |
| domain | Session states, commands, duration invariants | No filesystem, database, or UI imports |
| metaplan | Read project/task snapshots and diagnostics | Read-only filesystem access; never follow task links |
| storage | Schema migrations, settings, session transactions, queries | Only validated application storage |
| controller | Serialize commands, coordinate reader and storage, publish snapshots | UI cannot bypass session invariants |
| platform | Tray, single-instance lock, sleep/lock events, validated paths | Windows-specific code isolated here |
| ui | Settings, project/task picker, timer, history | English labels; imported text unchanged |
| reports | Date-range clipping and aggregation | Counts each interval once |

Use a fake clock and temporary storage in tests. OS events are converted into domain
commands so timing policies can be tested without a desktop.

## Source identity and reading

Assign each configured source a local ID. Store its canonical path in application
settings. Changing source creates a separate identity by default; reattaching a moved
source requires explicit selection of the existing identity. Do not guess from slugs.

Project identity is source ID plus slug, independent of Active/Archive placement.
A renamed slug is treated as a new project until an explicit remapping feature exists.
Duplicate slugs across both directories produce a diagnostic and are not selectable.
Store title snapshots in sessions so later renaming or deletion does not erase history.

Read only project.md and tasks.md under direct project directories. Skip reparse-point
project directories and linked files in the first implementation. The configured root
itself may be resolved once, then validated. Ignore blank task items and completed
checkboxes; restrict suggestions to the current cycle. Preserve section context to
distinguish duplicate task text. Treat task references as snapshot text, not stable IDs.
Do not follow external task paths or interpret Markdown as commands.

Show parse diagnostics and allow other valid projects to load. A missing source does
not prevent stopping an already running session or viewing saved history.

## Storage boundary

Use a fixed per-user local application directory, resolved through the Windows known
folder API, with independent Production/Development subdirectories (ADR-0010). Each
contains focus.sqlite3 and its own settings/window state. Source containment checks use
the shared parent root. Debug builds cannot select Production. No arbitrary storage-directory
setting is exposed. Journals, backups, and migrations stay within the selected profile.

Before creating directories or opening a writable database:
- Resolve existing ancestors, junctions, symbolic links, and case-insensitive identities.
- Reject source/storage equality or containment in either direction.
- Validate the canonical destination, not a raw path prefix.
- On first launch, choose and validate the source before persistent initialization.
  ADR-0005 allows desktop geometry and unassigned sessions to initialize before any
  source is configured; source selection must subsequently reject storage overlap.
- On later launches, open an existing database read-only to read the source setting,
  validate paths, then reopen for writes; do not run migrations before validation.
- Reject unsafe source changes before saving them or switching a session.
- Revalidate before writes after source configuration changes.

Use resolved-path handles where practical. Untrusted concurrent junction replacement
is outside the initial threat model; do not claim string checks eliminate that race.
Tests must cover normal nesting, similarly prefixed sibling names, case variants,
and junction aliases when the environment supports them.

No export feature is included in the initial slice. Future export destinations must
pass the same boundary checks, including temporary files.

## Persistence model

- sources: local ID and canonical path.
- settings: schema-versioned application options and selected source.
- recent use: source/project/task identity and increasing committed-use order; updated
  transactionally on start/switch/reassignment, never by browsing or stopped selection.
- sessions: ID, source ID, project slug/title snapshot, optional task snapshot,
  start/end UTC timestamps, start offset, elapsed milliseconds, end reason,
  last checkpoint, and recovery status.
- checkpoints: either fields on the active row or small segments, as determined by
  the timing implementation; retain enough information to split clock discontinuities.

Migration versions are explicit and transactional. Back up an existing database using
the SQLite backup API before a schema upgrade; never copy an open database blindly.
On storage failure, retain the last committed state, display an error, and allow retry.
Do not claim a successful stop or switch until the transaction commits.

## Timer semantics

| State / command | Result |
| --- | --- |
| Idle / Start | Commit a new session, then show Running |
| Running / Stop | Close the session; retain selection for a later Start |
| Running / Switch | Close old and open new at one boundary in one transaction |
| Idle / project or task picker change | Save the next activity; remain stopped |
| Running / picker change, default mode | Atomically switch at one boundary |
| Running / picker change, EditCurrent mode | Reassign the entire active interval; preserve ID/start/timer and finalized history |
| Running / Start | Reject duplicate start; never create a second active row |
| Running / lock or suspend | Close with an interruption reason; retain selection |
| Running / confirmed Windows session end | Save at the event boundary with a bounded confirmation wait |
| Any / canceled shutdown query | Keep tracking state unchanged |
| Interrupted / resume or unlock | Remain stopped until the user explicitly starts |
| Running / explicit Exit | Stop durably, then exit; stay open on save failure |
| Any / window close | Hide only when the tray is available; never stop implicitly |
| Startup / unfinished session | Enter recovery review before allowing another Start |

No idle-input auto-stop: reading and thinking may involve no input. No automatic
restart after unlock. Source priority and lifecycle statuses are never changed.
The hidden listener agrees immediately to WM_QUERYENDSESSION and saves only for a
confirmed WM_ENDSESSION. Force termination/power loss still use checkpoint recovery;
see ADR-0015. ADR-0016 replaces native size clamps with normal Windows resizing and
maximization, a bounded content column and persisted section/filter/search/draft state.

The upper UI caption shows the committed activity, distinct from pending note drafts.
Persist the picker mode independently per profile. Schema two backs up schema one and
seeds recent use from existing history once; failed backfill rolls back. See ADR-0012.

Use a monotonic clock for elapsed time and UTC for placement on the timeline.
Persist checkpoints at most 15 seconds apart while running. On a missed heartbeat,
sleep notification failure, or substantial wall/monotonic divergence, mark the
unconfirmed interval for review rather than silently counting it. Exact thresholds
are implementation constants documented and tested in T-0004.

After a crash, propose ending at the last durable checkpoint, not at the next launch.
The user may accept, adjust, or discard the uncertain tail; record the choice.
Do not silently resume a persisted running session.

## Reporting

Visible expanded reports poll periodically only while active time grows inside the
filter, with one query in flight. Refresh affected historical classifications
once on stop/edit/recovery, not on unrelated commits. Minute filters are half-open;
24:00 denotes end of day. Debounce edits, reject superseded parameters, retain the last
good snapshot on error, and pause queries while collapsed/hidden. See ADR-0014.
Calendar pickers share Jiff civil-date arithmetic with accounting; timezone choices
come from the bundled database. Stay on top is a window.ron option, not a history schema
change. See ADR-0013.

Use half-open intervals [start, end), and clip to the selected range before summing.
Resolve local calendar boundaries to UTC using the selected reporting time zone;
a day is not assumed to last 24 hours. Split intervals at midnight for daily totals.
Store UTC plus offsets and retain enough timing information to avoid recalculating
elapsed duration from a changed wall clock. Clock discontinuities require segmented
intervals or recovery review, not invented continuity.

Group by source/project identity and optional task snapshot. Show unfinished or
unreviewed records separately; do not silently include uncertain time in finalized
totals. Project relationships never recursively add the same time again.

## First release checks

Verify ordinary start/stop, rapid switching, process restart, sleep/lock, source
unavailability, read-only source integrity, database failure, Unicode titles, and
cross-midnight reports. Verify hidden-window wakeup and idle CPU behavior on Windows.
Do not claim packaging, timing, or tray acceptance from unit tests alone.
