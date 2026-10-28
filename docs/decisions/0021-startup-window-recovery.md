# ADR-0021: Safe Startup Window Geometry

Date: 2026-10-28.
Status: accepted.
Extends ADR-0018 restored maximization.

Never synthesize WM_GETMINMAXINFO with zeroed fields to determine startup window size.
Windows seeds real messages with defaults; pinned winit consumes the message and only
conditionally sets min/max tracking values. At early startup, the former manual query
could leave a zero width which the application applied through SetWindowPos.

Calculate restored maximized bounds directly from monitor work area, DPI frame and
the 760-logical-pixel width cap. Keep original-handler chaining for real native messages,
using the same explicit cap if the native tracking limit is absent. Preserve native
height sizing and avoid per-frame resize commands.

At startup, independently repair any restored normal client axis smaller than the
configured minimum (420 by 360 logical pixels) to its initial default (620 by 640).
Retain valid axes, position, sections/filters and profile data. Keep the existing normal
width ceiling. Do not clear or rewrite preferences before eframe loads them; normal
autosave records the corrected geometry. No schema or storage migration is needed.
