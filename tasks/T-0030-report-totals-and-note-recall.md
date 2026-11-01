# T-0030: Report Totals, Project Notes and Time Pickers

Status: done.
Depends on: T-0029.

## Acceptance

- Show a combined saved/running total and saved table footers in reports/exports.
- Recall last confirmed note per project and offer eight filtered recent notes.
- Keep note memory profile-local, separate from history and uncommitted drafts.
- Offer minute pickers without changing report boundary or timezone rules.
- Skip motion work for clipped/hidden cards and invisible viewports.
- Build and verify both canonical EXEs and the production archive.

## Verification

81 Rust tests, formatting and Clippy passed. Tests cover project/source isolation,
eight-note deduplication, recall serialization, case-insensitive filtering, pointer
selection after focus loss, Time popup actions, minute boundaries, export totals and
hidden/clipped/minimized/occluded animation gating. The first restricted test run
could not create registry/junction fixtures; the permitted isolated rerun passed.
All nine PDF preview pages were rendered and visually reviewed.
Both canonical 0.17.0 EXEs and the production ZIP were built through scripts/build.ps1.
The build verified the archive allowlist, source fingerprint, member hashes and manifest.
Native UI acceptance is pending fresh permission;
no desktop control or Production data access was used.
Production history and MetaPlan remain untouched. History schema remains two.

Next: verify native note recall across restart, Time popup editing, and scroll/hide/restore
with fresh desktop permission. Hardware GPU usage remains unmeasured. Continue T-0019
after that acceptance check; keep disruptive OS checks under T-0002.

Final release gate: staged UTF-8/no-BOM/LF and diff checks passed.
scripts/verify-release.ps1 passed ZIP, fingerprint/hash/manifest, eight neutral icon
sizes per EXE, and Production/Development version metadata. Preview and shipped guide
SHA-256 match: 6258F6B4CB54071AD447735E7AE686BE26AF03B2245C1FF7256A70117A59B3FC.
