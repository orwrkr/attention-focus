# T-0012: Immediate Activity Changes and Recent Pickers

Status: done.
Dependencies: T-0011.

## Outcome

Explain confirmed versus draft activity. Project/task changes immediately start a new
interval by default while running, or save the next activity while stopped. Persist an
option to reassign the entire current interval instead, preserving its start and elapsed
time. Keep notes explicit. Sort project/task choices by last successful tracking use,
including old history and after restart, with deterministic ties and source isolation.

## Plan and acceptance

- Add transactional interval reassignment, per-profile mode, and durable recency.
- Back up schema-one databases before additive migration; never modify MetaPlan.
- Label confirmed state and pending/error states; disable conflicting edits while saving.
- Test default switching boundaries, edit semantics, rollback, migration, recency, and restart.
- Update guide/ADR, bump release version, build both EXEs and verified production ZIP.
- Record manual UI verification limits honestly; never stop an app automatically.

## Verification

- 39 Rust tests pass. Seven new checks cover default split boundaries and unchanged
  selection, idle selection without starting, active reassignment with original elapsed
  time, finalized-row isolation, corrected crash checkpoints, mode/recency persistence,
  actual project/task ordering and source isolation, transactional usage-write failure,
  schema-one backup/backfill, failed-backfill rollback, and recovery without auto-resume.
- Formatting and Clippy with warnings denied pass. Five synthetic packaging tests pass.
- All six current guide PDF pages visually inspected after rendering with Poppler;
  no clipping, missing text, or unwanted pages found.
- Windows Computer Use list_windows timed out three times including session reset.
  No app was launched. Native picker interaction/header rendering remains unverified;
  engine/order tests are not a claim of interactive acceptance.
- No real profile database or MetaPlan file was changed by development tools.

- Both 0.3.0 EXEs built through scripts/build.ps1. Production ZIP refreshed automatically
  and verify-release passed: version metadata, all eight neutral icon sizes per EXE,
  exact ZIP allowlist, EXE build stamp, source fingerprint, and payload SHA-256 hashes.
- Final guide metadata/page text checked; packaged PDF matches the six visually
  inspected preview pages byte-for-byte. The previous 0.2.0 ZIP remains release history.

## Outputs and next step

- Production EXE: dist/AttentionFocus/attention-focus.exe.
- Development EXE: target/debug/attention-focus.exe.
- Guide: dist/AttentionFocus/User-Guide-0.3.0.pdf.
- Package: dist/AttentionFocus-0.3.0-windows-x64.zip.

In Development, verify that project/task selection updates the confirmed caption,
defaults to a new interval, preserves elapsed time with the checkbox disabled, and
orders previously tracked choices first after restart. The UI acceptance gate remains
with T-0005/T-0002 because the Windows helper failed before any app launch. Never
insert synthetic production time or stop an app automatically.
