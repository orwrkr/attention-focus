# ADR-0028: Verified Window Activation

Date: 2026-11-03.
Status: accepted.
Basis: user report that a tray click does not always bring the window to the front.

## Context

Show sent Visible(true), Minimized(false) and Focus, then assumed success. In the pinned
winit 0.30.13, Focus injects an Alt press and calls SetForegroundWindow once, without
checking the result. Windows may refuse that request: ADR-0006 delays single-click
opening by the double-click interval, so rights granted by the tray click can lapse, and
injected input is queued rather than applied synchronously. A refusal only flashes the
taskbar button and leaves the window behind others. The refusal could not be reproduced
through automation, so this explanation comes from reading the code path and API rules.

## Decision

Replace the Focus command with a native, verified sequence in platform.rs:

1. Wait until the window is natively visible and not minimized.
2. Pulse HWND_TOPMOST/HWND_NOTOPMOST without activation. Z-order needs no foreground
   rights, so the window ends up above ordinary windows even if activation is refused.
   Stay on top keeps its topmost state.
3. Call SetForegroundWindow. If refused, briefly attach to the foreground thread's
   input queue and repeat the request. Detach immediately.
4. Confirm with GetForegroundWindow. Retry at most eight times, 80 ms apart.

Retries are timed, so unrelated repaints such as the border animation cannot use them up.
Skip the input attachment when the foreground window is hung, since sharing its input could
stall this thread. Show errors once per distinct message, since stronger activation would
otherwise let a persistent per-second failure keep stealing focus. Forget the ignored final
release of a double click when a plain press arrives, so a release lost outside the icon
can no longer swallow the next single click, however long the second press was held.

## Alternatives

- Keep Focus and retry it: every retry injects Alt, which can open the menu bar of the
  application that keeps the foreground when the request is refused.
- Open on the first click: rejected by ADR-0006, a double click would flash the window.
- Stay always topmost: changes window behavior the user controls through Stay on top.

## Consequences

Elevated foreground windows block the input attachment, so the window is raised above
them but may remain unfocused. AttachThreadInput is a known workaround rather than a
recommended API; it is held only for one request. A window can take the foreground from
the user for up to about 0.6 seconds after a click if activation keeps being refused.
Automated tests cover Z-order, topmost preservation, hidden-window refusal and bounded
retries without activating anything. Actual foreground behavior needs a native check.
