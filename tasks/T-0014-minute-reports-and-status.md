# T-0014: Minute Reports and Unambiguous Status

Status: done.
Dependencies: T-0013.

Replace tray transport symbols with status lights (filled green running, hollow gray
stopped), keeping neutral identity and Development badges. Support minute-precise,
half-open report boundaries in the selected timezone and preserve full-day presets.
Only poll reports while active time can grow inside the filter; refresh affected
historical results once on stop/edit/recovery, not for unrelated activity. Explain
project search and unfinished/review totals in UI/guide, and report existing backlog
without implementing deferred work. Keep SQLite schema unchanged and test minute
allocation, DST, refresh relevance, and status artwork. Build both EXEs and guide/ZIP.

## Acceptance criteria

- [x] Replace action-like tray symbols while retaining neutral application identity.
- [x] Clip reports to minute boundaries and preserve whole-day presets and DST rules.
- [x] Poll only growing matching time; refresh affected classifications once.
- [x] Explain project search, Running, Needs review and existing deferred work.
- [x] Verify both versioned builds, guide and ZIP; record remaining native limitations.

## Verification and handoff

- 51 Rust tests and five package tests passed; formatting and Clippy passed.
- Both 0.5.0 EXEs, seven-page guide and production ZIP built through build.ps1.
  verify-release.ps1 passed versions, resources, fingerprints, members and hashes.
  All seven guide pages were rendered and visually inspected.
- Native Development checks used synthetic read-only projects: source settings,
  case-insensitive project filtering, stopped selection caption, calendar selection,
  minute entry, and start/stop reporting. A historical filter retained its snapshot
  during tracking. Today enabled live updates; stopping moved Running to Saved and
  paused polling. No Production history was used or changed.
- Development remains open, stopped, with the synthetic source under ignored
  target/manual-instance. Do not close it automatically. Future builds must report
  its locked EXE or obtain approval to exit it.
- Real tray interaction, topmost restoration, geometry, lock/sleep/recovery, and
  clean-machine runtime acceptance remain T-0002/T-0005 manual work. Automated icon
  checks do not substitute for observing tray states on Windows.

Next: complete the remaining Development-only Windows acceptance checklist, then
choose deferred product work separately. SQLite schema remains two.
