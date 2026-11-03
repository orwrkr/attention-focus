# T-0031: Bring the Window to the Front on Tray Click

Status: done.
Dependencies: T-0029.

## Goal

A tray click must reliably place the main window above other windows. The user reported
that it does not always happen.

## Scope

Verified native activation, its bounded retries and one tray-click edge case. No change
to the delayed single-click rule from ADR-0006. See ADR-0028.

## Acceptance criteria

- [x] Show raises the window above ordinary windows even when activation is refused.
- [x] Foreground state is confirmed and retried a bounded number of times.
- [x] Stay on top survives raising; the raise needs no synthetic key press.
- [x] Persistent errors do not keep stealing focus.
- [x] A double-click release lost outside the icon cannot swallow a later click.
- [ ] Native tray check by the user, see Handoff.

## Checks

`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and
`cargo test --locked` passed. Off-screen, non-activating test windows cover Z-order,
topmost preservation and refusal while hidden. `WindowRaise` tests cover retry bounds and
timing, and `Clicks` tests cover a lost final release and a long-held second press. An
independent read of the first version found untimed retries and a regression with a
long-held second press; both were fixed.

Unverified: SetForegroundWindow and AttachThreadInput behavior against a real tray
click. Automated tests deliberately never activate a window, and the original refusal
was not reproduced. The cause in ADR-0028 comes from reading the pinned code and API rules.

## Handoff

Check in Development: cover the window with another application, then single-click the
tray; repeat from hidden, minimized and overflow-flyout states and with Stay on top. If
it still stays behind, record which foreground application was active and whether it
was elevated.
