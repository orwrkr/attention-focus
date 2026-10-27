# T-0017: Restored UI State, Stable Resize and Daily Tray Total

Status: done.
Dependencies: T-0015.

## Scope and acceptance

- [x] Persist expanded sections, scroll state, report minute/timezone filters, presets,
      searches and unapplied note draft per profile without resuming tracking.
- [x] Remove forced sizing/maximize interception; cap content width, not native geometry.
- [x] Show today's saved plus active time for the selected project across all its tasks,
      by source identity and local calendar day. Exclude unreviewed time, handle midnight,
      and keep failed historical reads from blocking Start/Stop state publication.
- [x] Verify restoration, daily accounting and resize behavior; publish both EXEs/guide/ZIP.

Next implementation task is T-0016 export. T-0002 retains real tray/OS acceptance.

## Verification

54 Rust tests, formatting and Clippy passed. Tests cover custom/preset preference
round trips, corrupt preference fallback, project/source/task isolation, midnight,
review acceptance, rollover and unavailable daily totals without blocking state.
Both 0.7.1 EXEs and automatic ZIP passed release verification. All seven PDF pages
were rendered and visually checked. 0.7.0 was retained as an intermediate archive;
0.7.1 fixes the exposed background outside the content column when maximized.

Native Development checks restored an open report/task section, custom 09:15 boundary,
project search and unapplied synthetic note after Exit/relaunch. Tracking stayed stopped.
Maximize remained maximized; restore returned normally. Windows System menu Size/Down
increased height in steps without snapping back. Mouse capture cannot target the
invisible resize border outside its screenshot, so the keyboard sizing path was used.
Source remained synthetic; Production was untouched. Development exited via approved
normal Exit. Actual tray rendering/hover and multi-monitor checks remain T-0002.
