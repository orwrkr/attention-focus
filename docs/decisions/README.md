# Architecture Decisions

Each ADR records context, decision, alternatives, and consequences. Statuses:
`proposed`, `accepted`, `superseded`. Do not silently change an accepted decision;
record a replacement and link to the superseded record. Translation preserves meaning.

| ADR | Status | Topic |
| --- | --- | --- |
| [0001](0001-product-boundaries.md) | superseded in part | Rust and read-only MetaPlan integration |
| [0002](0002-english-only.md) | superseded in part | English-only project and application UI |
| [0003](0003-chat-language.md) | accepted | Chat language clarification |
| [0004](0004-desktop-stack.md) | superseded in part | Native Rust UI and transactional storage |
| [0005](0005-desktop-state-and-branding.md) | accepted | Persistent desktop state and tray tracking |
| [0006](0006-tray-single-click.md) | accepted | Single-click window access and hover time |
| [0007](0007-source-settings.md) | superseded | Source settings and read-only preview |
| [0008](0008-attributed-session-engine.md) | accepted | Attributed sessions, migration, and lifecycle |
| [0009](0009-focus-reports.md) | accepted | Date-range focus reports and timezone accounting |
| [0010](0010-runtime-profiles.md) | accepted | Independent production/development and pinned actions |
| [0011](0011-versioned-distribution.md) | superseded in part | Neutral status icons and versioned production packages |
| [0012](0012-immediate-activity-changes.md) | accepted | Immediate interval changes and durable recent pickers |
| [0013](0013-live-reports-and-desktop-controls.md) | accepted | Live reports, calendars, timezone list, and window controls |
| [0014](0014-minute-ranges-and-status-lights.md) | accepted | Minute boundaries, relevant refresh, and tray status lights |
| [0015](0015-shutdown-and-compact-window.md) | accepted | Confirmed session end and content-fitted window limits |
| [0016](0016-ui-state-and-daily-tray.md) | accepted | UI restoration, native sizing and project-day tray totals |
| [0017](0017-report-export.md) | accepted | Read-only background snapshots and protected CSV/Markdown output |
| [0018](0018-window-counters-and-width.md) | accepted | Explicit window counters and a static native width limit |
| [0019](0019-scroll-and-selection-controls.md) | superseded in part | Stable scroll reflow, visible window edges and selection controls |
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
| [0030](0030-launch-commands-and-clock-corrections.md) | accepted | Repeated launches, queued commands and small clock corrections |
| [0031](0031-application-projects.md) | accepted | Application-owned projects and tasks |
| [0032](0032-activity-charts.md) | accepted | Activity charts in reports |
| [0033](0033-release-workflow-and-signing.md) | accepted | Release workflow and free code signing |
| [0034](0034-scroll-bars-and-themes.md) | accepted | One scroll bar style and selectable themes |
| [0035](0035-interface-languages.md) | accepted | Interface languages and right-to-left text |
| [0036](0036-single-exe-release.md) | accepted | One EXE as the release, with licenses inside it |
| [0037](0037-info-marks-and-about.md) | accepted | Info marks for explanations, and an About button |
| [0038](0038-microsoft-runtime-and-file-properties.md) | accepted | Microsoft runtime terms, license wording and file properties |
| [0039](0039-history-correction.md) | accepted | Correcting history by hand |
| [0040](0040-project-site.md) | accepted | A static project site on GitHub Pages |
| [0041](0041-dependency-and-code-scanning.md) | accepted | Dependency and code scanning on GitHub |
| [0042](0042-history-growth-and-query-cost.md) | accepted | Query cost that stays flat as the history grows |
| [0043](0043-write-ahead-log.md) | accepted | A write-ahead log for the history |

Next number: `0044`. Template: `templates/ADR.md` from the repository root.
