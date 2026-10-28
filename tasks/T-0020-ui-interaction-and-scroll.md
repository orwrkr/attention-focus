# T-0020: Scroll Reflow, Window Edges and Selection Controls

Status: done.
Depends on: T-0018.

## Scope

Fix stale/incorrect scrolling after section changes, make the constrained maximized
window boundary visible, and give the primary action hover/pressed feedback. Move
project search inside its dropdown, distinguish Unassigned focus without losing recent
ordering, and provide a read-only folder-selection action beside the source path.
History editing T-0019 stays ready; this user-requested UI correction takes priority.

## Acceptance

- [x] Show a visible scrollbar only on overflow; retain stable content width and clear
  stale painting after section reflow. Test repeated expand/collapse transitions.
- [x] Draw a visible constrained-maximized boundary without dynamic window resizing.
- [x] Give Start/Stop distinct idle, hover, pressed and disabled behavior.
- [x] Put project search inside the popup and distinguish Unassigned focus.
- [x] Browse for an existing folder, preserve Unicode/cancel behavior and apply existing
  source/storage validation only through Save and refresh. Never write to MetaPlan.
- [x] Run Rust/layout/package checks and Development UI acceptance; build both 0.11.1
  EXEs, regenerate/review the guide and verify the automatically refreshed ZIP.

Record limitations honestly. Real OS power/session interruption and other native
monitor/DPI combinations remain T-0002 gates. Use synthetic Development data only.

## Results

66 Rust tests, Clippy and formatting passed. Five package tests passed. New actual-paint
tests distinguish idle/hover/pressed button colors and reject disabled clicks. Reflow
tests expand/collapse repeatedly at narrow/wide widths, scroll to the end, verify reset
and overflow state, and reject duplicate timer painting. Existing margin/tooltip/source
containment and session invariants continue passing.

Native Development 0.11.0 verified no bar when content fits, a visible bar after source
expansion, stable counter widths, and clean report/source collapse without stale cards.
The constrained maximized view has an explicit boundary. Popup search stayed open while
editing beta to alpha; the confirmed Beta activity stayed unchanged until selection.
The colored italic Unassigned entry closed the popup and changed project Today to zero
while global Today retained 15 seconds. No tracking session was created for these checks.

Browse opened on the existing synthetic Instance with New Folder hidden. Confirm filled
the field; Save and refresh applied the normalized source. Reopen/cancel kept the path.
No production data or live MetaPlan source was used. Native Unicode/UNC directory choices
were not separately exercised; the picker uses UTF-16 and existing boundary validation.

After a turn interruption a final hover capture reported "foreground window did not
report a process id". Fresh lookup and a process check found no Development process,
only the untouched external Production copy. No further UI input was sent. Native
hover/pressed capture was not completed; the actual-paint automated check passed.

Final review restored the pre-existing three-second hidden-window autosave repaint;
the immediate reflow repaint must not replace it. This and version metadata are the
only changes from native-checked 0.11.0 to final 0.11.1. Both final EXEs and the automatic
ZIP passed verify-release metadata/icons/fingerprint/hash/allowlist checks. The final
eight-page PDF was rendered and reviewed. No schema change. Next: T-0019.
