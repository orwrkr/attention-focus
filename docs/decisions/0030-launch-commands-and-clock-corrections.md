# ADR-0030: Repeated Launches, Queued Commands and Small Clock Corrections

Date: 2026-11-04.
Status: accepted.
Basis: first project review (T-0034); refines ADR-0008.

## Decision

**Repeated launches.** A manual launch of a profile that is already running signals the
running instance through a per-profile named event, passes on its foreground right with
AllowSetForegroundWindow, and exits. The instance shows its window as if the tray had been
clicked. The error message remains only when no instance is listening yet. Sign-in startup
stays quiet, as in ADR-0023.

**Queued commands.** The desktop previously dropped a command while another was in flight,
so a tray Exit or toggle could silently do nothing. One command stays in flight, and later
ones wait in order, up to eight. Interface controls remain disabled while a command runs.
Each interface command gets exactly one answer, Committed or Rejected. Windows interruptions,
reports and timer ticks report separately, so they cannot release the next command early.
A command decided before a list change can run after it, so the engine itself refuses
to start on an archived project or a finished task.

**Small clock corrections.** Any backward wall-clock step previously interrupted tracking,
while forward drift of up to five seconds was tolerated. Windows can step a clock back by
a small amount when it synchronizes, which turned an ordinary correction into a stopped
timer and a review prompt. Backward steps now share the five-second allowance. Within it, wall
time continues from the latest observation by monotonic time until the interval ends. Every
interval therefore spans its elapsed time and intervals never overlap, so reports, Today and
saved totals agree. Larger steps still end at the last checkpoint and require review.

**Stored-state cache.** The worker reuses its stored snapshot between commands, so each
tick no longer repeats every aggregate query. Commands, day splits and interruptions refresh it.

## Alternatives

- Keep strict backward detection: simple, but it penalizes routine time synchronization.
- Accept a backward step as it is: a later interval could then overlap.
- Freeze wall time until the clock catches up: an interval inside the correction would have no
  wall span, and reports exclude those as invalid while saved totals count them.
- Find the running window from the second process: its windowing state would go stale, because
  the owning process would not know the window had been shown.

## Consequences

Saved wall times can run up to five seconds ahead of the corrected clock until the interval
ends. A new start while the clock is earlier than the latest saved end is still rejected, as
in ADR-0008, for up to those seconds. Any same-user process can ask
the instance to show its window, which has no effect beyond that.
