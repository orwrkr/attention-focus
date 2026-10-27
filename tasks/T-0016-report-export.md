# T-0016: CSV and Markdown Report Export

Status: done.
Dependencies: T-0005.

## Scope and plan

Implement previously deferred export as the next product task. Research native file
selection and record an ADR before choosing a dependency. Export an immutable snapshot
of the selected minute filter/timezone, saved totals, separate Running/Needs review
categories and session details. Keep export independent of source availability.

## Acceptance criteria

- [x] Provide English CSV/Markdown actions and clear success/error feedback.
- [x] Include every match, not only the UI's 200-row preview; retain precise durations,
      filter boundaries, timezone, snapshot time and app version.
- [x] Preserve Unicode, escape CSV/Markdown and define safe handling of formula-like text.
- [x] Resolve paths/links and reject destinations inside MetaPlan or application storage.
      Do not silently overwrite files; avoid blocking tracking on large exports.
- [x] Test clipping, all matching rows, category separation, escaping, source integrity,
      failures and profile isolation using temporary synthetic data.
- [x] Update the guide/version, build both EXEs and refresh/verify the production ZIP.

No history editing, shortcuts, autostart or installer in this card. Real tray/system
lifecycle validation remains T-0002; do not claim those checks passed.

## Verification and handoff

59 Rust tests and Clippy passed. Export tests cover 207 rows beyond the preview limit,
running checkpoint/review separation, unchanged history bytes, formula/Markdown escaping,
protected source/storage/junction destinations, no-overwrite, IO errors and a 5,001-row
snapshot concurrent with Stop. Both 0.8.0 EXEs and automatic ZIP passed verify-release;
all eight PDF pages were rendered and visually inspected.

Native verification resumed after the physical-Escape interruption: CSV Save selected
a new file under ignored target/manual-exports. The app reported success; Python CSV
parsing verified one Summary and four synthetic Session rows, exact saved total and
version. Markdown opened a .md Save dialog and cancellation returned Export canceled
without creating a file. Development exited through approved normal Exit. Production
was not touched. Full native Markdown save and real tray/OS gates are not claimed.

Next: T-0018 adds explicit window counters and a stable horizontal size limit.
