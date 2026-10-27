# T-0018: Explicit Window Counters and Stable Width Limit

Status: done.
Depends on: T-0017.

## Scope

Display current-interval time separately from today's selected-project total. Prevent
horizontal expansion beyond useful content without restoring dynamic height clamps.
Add all-project Today in the window/tray, one full-name hint, symmetric card edges and
a prominent Start/Stop action. Extend cached daily accounting; no schema changes or
Production data writes. Final version: 0.10.0 (0.9.0 was archived during verification).

## Acceptance

- [x] Label Current interval and Today for project separately; name the daily project
  and local-day/all-task scope. Keep all-time saved totals explicitly distinct.
- [x] Reuse the tray's daily value including live time once and excluding review time.
- [x] Add all-project Today to both surfaces and emphasize Start/Stop with a large blue action.
- [x] Keep one full-name hint and symmetric frame/window edges, including scrollbar space.
- [x] Apply a static native width limit; preserve ordinary vertical resize and maximize.
- [x] Check stopped/running counters, narrow/wide/maximized layouts and section reflow
  in Development on synthetic data. Never interrupt the user's Production copy.
- [x] Update guide/version, run formatting/tests/Clippy and packaging tests; build both
  EXEs, review PDF pages and verify the refreshed production ZIP before committing.

## Plan

Use two compact counter cards and the confirmed activity snapshot. Set a native width
limit once in the viewport builder and cap native maximize width via a window subclass,
with a generous height ceiling and no per-frame resize. The initial builder-only limit
failed to constrain maximize; the user supplied a screenshot showing the failure.
Verify through normal Windows input. Keep T-0002 real lifecycle/multiple-monitor gates
explicit; prepare history correction as the next product task after this refinement.

## Verification and limitations

Completed on 2026-10-28 after earlier Escape interruptions and a locked dev EXE. The
locked build failed without alternate output; the user exited that dev instance.
64 Rust tests, formatting, Clippy and five package tests passed. A restricted rerun
failed three junction-creation checks; the permitted Windows-link rerun passed all 64.
Tests cover equal frame edges at 100/125/150/200 percent scale, mirrored solid-scrollbar
gutters at 420/620/760 widths, exactly one elided-name hint, native subclass chaining,
monitor/frame bounds, all-project/unassigned live counting, midnight and review exclusion.

Native Development verified prominent Start and Stop, 11 seconds of live time appearing
once in both daily values, zero interval after Stop with 15-second daily totals retained,
and a previous-day custom report remaining independent of today's counters. Maximize
and restored maximized startup kept the caption visible within the current monitor's
work area. The final narrow and wide views have symmetric gutters without scrollbar
overlap. Prior downward sizing remained stable. Development exited normally; a process
check confirmed only the external Production copy remained running and untouched.

scripts/build.ps1 built both 0.10.0 EXEs, eight-page guide/notices and the automatic ZIP.
verify-release passed fingerprint, hashes, allowlist, metadata and neutral icon checks.
All eight rendered guide pages were visually reviewed. Poppler emitted fallback-font
warnings; the rendered pages have no missing or clipped text.

Actual tray-shell hover/rendering, multiple native monitor/DPI combinations, real
lock/sleep/hibernate/logoff/shutdown and clean-machine checks remain T-0002 gates.
No real OS interruption was performed on the user's working machine.
Next implementation task: T-0019, history correction and manual intervals.
