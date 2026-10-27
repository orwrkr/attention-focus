# ADR-0008: Attributed Session Engine and Lifecycle

Date: 2026-10-27.
Status: accepted.

## Decision

Serialize tracking commands through one worker and inject wall/monotonic timestamps
into an engine so tests do not sleep or change the real system clock. Each session
stores an immutable activity snapshot: source identity and canonical path, project
slug/title, optional task text, and optional note. A separate singleton retains the
last committed activity for tray starts and restart. Editing the picker is a draft;
Apply selection, Start, or Switch makes it the committed selection.

Switch closes the old row and starts the next at one shared wall/monotonic boundary
inside one transaction. A failed write retains both the old timer and selection.
The unique running-row index remains the final protection against overlap.

Version focus.sqlite3 with PRAGMA user_version. Before upgrading legacy version zero,
create a unique same-directory backup using SQLite's online backup API, then migrate
in a transaction. Preserve existing unassigned history. Reject unsupported future versions.
Activity snapshots use serde JSON stored as SQLite text; malformed snapshots fail visibly.

A native hidden top-level window receives Windows session-lock and suspend events.
These events close the active session; unlock and resume do not restart it. Missed
heartbeats over 30 seconds or wall/monotonic divergence over five seconds instead end
at the last durable checkpoint and require review. Checkpoint at ten-second intervals.
Reject new starts while the wall clock precedes the latest recorded end, avoiding
overlapping finalized timelines after a clock rewind. Do not invent a corrected UTC time.
Capture the event's wall/monotonic boundary before notifying the worker, so delayed
processing does not include time asleep. Wait at most 1.5 seconds for durable confirmation;
surface a failure instead of claiming the event was saved. Disable new starts if native
listener initialization fails; stopping and exit remain available.

## Consequences

Project/task controls are a minimal way to use and verify this engine. Search and
date-range reports remain T-0005. OS-event simulations and fake-clock tests are
separate from real Windows lock/suspend checks; neither substitutes for the other.
The existing clipboard fix uses eframe's documented clipboard feature and native
backend rather than adding an application-specific paste handler.

## Sources reviewed

- [Windows session-state messages](https://learn.microsoft.com/en-us/windows/win32/termserv/wm-wtssession-change)
- [Windows power broadcasts](https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast-messages)
- [Windows sleep notification budget](https://github.com/MicrosoftDocs/win32/blob/docs/desktop-src/Power/system-sleep-criteria.md)
- Local rusqlite 0.41 backup API and eframe 0.37 clipboard feature definitions.
