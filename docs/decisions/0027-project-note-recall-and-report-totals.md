# ADR-0027: Project Note Recall and Report Totals

Date: 2026-11-02.
Status: accepted.

Store note memory in profile-local eframe preferences, not the history database.
Identity uses source ID, canonical path and project slug, excluding task/title;
Unassigned has its own bucket. Remember confirmed activity snapshots only, with a
last note and eight distinct nonblank recent notes per project. Failed commands and
unapplied drafts do not update recall. Flush only changed memory; periodic worker
snapshots do not cause writes. Existing current selection seeds memory on first load.
No historical backfill or schema migration is needed; old schema-two apps remain
compatible. Preference loss removes suggestions, not recorded history.

Total means saved plus running, excluding review. Group footers sum saved time only.
Exports retain durable checkpoint semantics, with an added CSV total metadata column.
Use existing egui popups and controls for note suggestions and minute selection.
Time pickers preserve manual entry, explicit Apply/Cancel and exclusive 24:00 end.
No new dependency. Visibility gates precede accessibility queries and border geometry;
OS-reported occlusion is used, not focus loss. Physical GPU usage is not inferred
from headless geometry/repaint tests.
