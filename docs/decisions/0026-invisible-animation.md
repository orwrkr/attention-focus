# ADR-0026: Suspend Invisible Animation

Date: 2026-10-29.
Status: accepted.
Basis: user request; extends ADR-0025.

Gate both border geometry and its 50-ms repaint request with widget clip visibility
and ViewportInfo::visible. Minimized or OS-reported occluded windows do not animate.
Do not use focus as visibility: a visible unfocused window may still animate.

Pinned eframe 0.37 glow integration uses ViewportInfo::visible to skip UI and GPU
painting, while retaining logic updates. That predicate checks minimized/occluded,
not native WS_VISIBLE. Therefore send Minimized(true) before Visible(false) for all
tray-hide paths, including autostart reconciliation. Existing Show sends Visible(true),
Minimized(false), Focus. Keep native geometry recovery unchanged.

Worker wakeups and tray updates continue for accurate tracking. Scrolled-out cards
produce no border geometry or animation scheduling; other visible UI may still render
for events or timer/report updates. A previously queued frame may settle a transition.
Do not claim zero total GPU activity or detect full occlusion when Windows does not
report it. No new polling, tracking writes, dependencies or schema changes.
