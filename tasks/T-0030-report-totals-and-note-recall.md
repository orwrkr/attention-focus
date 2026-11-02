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

## Native acceptance follow-up

Development 0.17.0 verified filtered note suggestions and mouse selection, retained
confirmed note after reopening, and correct live report saved/running total. The
border trail changed position while visible. Hide to tray removed the main window
from the window inventory; restoration through the tray was not accessible to the
helper. A second launch correctly reported the existing Development instance.
The user closed Development through Exit after the control session was reset.
Production was not operated. Physical GPU activity remains unmeasured.

Time popup inspection exposed automatic menu closure when clicking the hour editor.
The initial Cancel and end-of-day tests did not cover this interaction. Add a numeric
field interaction regression and use explicit popup closing behavior in 0.17.1.
Native editing and hide/restore acceptance remain pending after the fix.

0.17.1 follow-up completed: the numeric-field click regression failed on 0.17.0 and
passed with CloseOnClickOutside. All 81 tests, fmt and Clippy passed. Both canonical
EXEs, PDF/notices and ZIP were rebuilt; verify-release passed version/icon/hash/manifest
checks. The version-only guide preview matches the shipped PDF hash; its cover was
visually checked (the unchanged nine-page layout was reviewed for 0.17.0).

With fresh permission, native Development verified double-click text entry for hour
and minute, Apply setting 09:15, and Cancel retaining 09:15 after entering another
hour. Development exited through Exit and disappeared from the window inventory;
the control session was immediately reset. No tracking was started in that final
session. Native tray restoration, completely scrolled-out animation and hardware GPU
measurements remain unverified; headless invisibility checks passed. Next: those
remaining acceptance checks only when desktop access is authorized, or T-0019 planning.
