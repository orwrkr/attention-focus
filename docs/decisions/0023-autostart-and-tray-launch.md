# ADR-0023: Explicit Per-User Autostart

Date: 2026-10-29.
Status: accepted.
Basis: user request.

Use the current user's Windows Run key with separate AttentionFocus and
AttentionFocusDevelopment values. Write/delete only the selected profile's value after
an explicit settings action. Never auto-register or repair registration at startup.
Read the actual value for status; allow explicit refresh and registration repair after
moving the portable EXE. Quote its absolute path, include its explicit profile and
--autostart marker, and optionally --start-in-tray. Reject commands beyond the documented
260-character limit rather than silently creating unreliable registration.

Start hidden only when both launch flags are present and a tray was successfully
created. Manual launches ignore the remembered tray preference. Never resume tracking.
Duplicate autostart invocations exit quietly; ordinary duplicates retain their message.

Pinned eframe 0.37 unconditionally shows the first rendered window, even with a hidden
viewport builder. Suppress that one native SWP_SHOWWINDOW operation through the existing
UI-thread subclass for tray startup, then reconcile winit's cached visibility through
Visible(false) at the next logic pass before handling Show/error actions. Later explicit
Show and error recovery work normally.
No UI resize loop or additional polling thread is introduced.

Keep the existing eight-second status motion and Windows client-area preference. Add
an explanatory settings status rather than overriding accessibility preferences.

Tests use an isolated temporary HKCU subkey, never real Run values or Production data.
Real Windows logon remains a non-disruptive-user acceptance gate; do not log off to test.

References: [Microsoft Run keys](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys),
[Windows animation preference](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow).
