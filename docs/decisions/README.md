# Architecture Decisions

Each ADR records context, decision, alternatives, and consequences. Statuses:
`proposed`, `accepted`, `superseded`. Do not silently change an accepted decision;
record a replacement and link to the superseded record. Translation preserves meaning.

| ADR | Status | Topic |
| --- | --- | --- |
| [0001](0001-product-boundaries.md) | accepted | Rust and read-only MetaPlan integration |
| [0002](0002-english-only.md) | accepted | English-only project and application UI |
| [0003](0003-chat-language.md) | accepted | Chat language clarification |
| [0004](0004-desktop-stack.md) | accepted | Native Rust UI and transactional storage |
| [0005](0005-desktop-state-and-branding.md) | accepted | Persistent desktop state and tray tracking |
| [0006](0006-tray-single-click.md) | accepted | Single-click window access and hover time |
| [0007](0007-source-settings.md) | accepted | Source settings and read-only preview |
| [0008](0008-attributed-session-engine.md) | accepted | Attributed sessions, migration, and lifecycle |
| [0009](0009-focus-reports.md) | accepted | Date-range focus reports and timezone accounting |
| [0010](0010-runtime-profiles.md) | accepted | Independent production/development and pinned actions |
| [0011](0011-versioned-distribution.md) | accepted | Neutral status icons and versioned production packages |
| [0012](0012-immediate-activity-changes.md) | accepted | Immediate interval changes and durable recent pickers |
| [0013](0013-live-reports-and-desktop-controls.md) | accepted | Live reports, calendars, timezone list, and window controls |
| [0014](0014-minute-ranges-and-status-lights.md) | accepted | Minute boundaries, relevant refresh, and tray status lights |
| [0015](0015-shutdown-and-compact-window.md) | accepted | Confirmed session end and content-fitted window limits |
| [0016](0016-ui-state-and-daily-tray.md) | accepted | UI restoration, native sizing and project-day tray totals |
| [0017](0017-report-export.md) | accepted | Read-only background snapshots and protected CSV/Markdown output |
| [0018](0018-window-counters-and-width.md) | accepted | Explicit window counters and a static native width limit |
| [0019](0019-scroll-and-selection-controls.md) | accepted | Stable scroll reflow, visible window edges and selection controls |
| [0020](0020-calm-tracking-motion.md) | accepted | Restrained running indicators and Windows motion preferences |
| [0021](0021-startup-window-recovery.md) | accepted | Safe startup maximization and collapsed-size recovery |
| [0022](0022-header-drag-and-centering.md) | accepted | Native header dragging and centered default placement |
| [0023](0023-autostart-and-tray-launch.md) | accepted | Explicit per-user sign-in startup and hidden tray launch |
| [0024](0024-motion-and-write-cadence.md) | accepted | Visible activity marker and bounded UI persistence |
| [0025](0025-gradient-tracking-border.md) | accepted | Rounded gradient tracking trail without layout changes |
| [0026](0026-invisible-animation.md) | accepted | Suspend hidden and clipped animation rendering |
| [0027](0027-project-note-recall-and-report-totals.md) | accepted | Project note recall and report totals |
| [0028](0028-verified-window-activation.md) | accepted | Verified window activation on tray click |
| [0029](0029-day-boundary-intervals.md) | accepted | Split intervals at the local day start |

Next number: `0030`. Template: `templates/ADR.md` from the repository root.
