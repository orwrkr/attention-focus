# T-0016: CSV and Markdown Report Export

Status: ready.
Dependencies: T-0005.

## Scope and plan

Implement previously deferred export as the next product task. Research native file
selection and record an ADR before choosing a dependency. Export an immutable snapshot
of the selected minute filter/timezone, saved totals, separate Running/Needs review
categories and session details. Keep export independent of source availability.

## Acceptance criteria

- [ ] Provide English CSV/Markdown actions and clear success/error feedback.
- [ ] Include every match, not only the UI's 200-row preview; retain precise durations,
      filter boundaries, timezone, snapshot time and app version.
- [ ] Preserve Unicode, escape CSV/Markdown and define safe handling of formula-like text.
- [ ] Resolve paths/links and reject destinations inside MetaPlan or application storage.
      Do not silently overwrite files; avoid blocking tracking on large exports.
- [ ] Test clipping, all matching rows, category separation, escaping, source integrity,
      failures and profile isolation using temporary synthetic data.
- [ ] Update the guide/version, build both EXEs and refresh/verify the production ZIP.

No history editing, shortcuts, autostart or installer in this card. Real tray/system
lifecycle validation remains T-0002; do not claim those checks passed.
