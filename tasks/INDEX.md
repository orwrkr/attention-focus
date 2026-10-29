# Tasks

| ID | Status | Task | Dependencies |
| --- | --- | --- | --- |
| [T-0001](T-0001-architecture.md) | done | Architecture and first working slice | None |
| [T-0002](T-0002-desktop-spike.md) | ready | Desktop and Storage Integration Spike | T-0001 |
| [T-0003](T-0003-source-settings.md) | done | Read-Only Source and Settings | T-0002 implementation; user override |
| [T-0004](T-0004-session-engine.md) | done | Durable Sessions and Recovery | T-0003 |
| [T-0005](T-0005-focus-ui-reports.md) | done | Focus Workflow and Time Reports | T-0004 |
| [T-0006](T-0006-desktop-state-branding.md) | done | Window Persistence, Branding, and Tray Toggle | T-0002 implementation; user override |
| [T-0007](T-0007-tray-click-tooltip.md) | done | Tray Single Click and Tracked-Time Tooltip | T-0006 |
| [T-0008](T-0008-system-clipboard.md) | done | System Clipboard | T-0003 |
| [T-0009](T-0009-release-output.md) | done | One Release Output | T-0008 |
| [T-0010](T-0010-runtime-profiles.md) | done | Pinned Window Actions and Runtime Profiles | T-0009 |
| [T-0011](T-0011-release-package-guide.md) | done | Status Icons, User Guide, and Versioned Packages | T-0010 |
| [T-0012](T-0012-selection-switching.md) | done | Immediate Activity Changes and Recent Pickers | T-0011 |
| [T-0013](T-0013-report-controls.md) | done | Live Reports and Desktop Controls | T-0012 |
| [T-0014](T-0014-minute-reports-and-status.md) | done | Minute Reports and Unambiguous Status | T-0013 |
| [T-0015](T-0015-lifecycle-and-compact-window.md) | done | Confirmed Shutdown and Compact Window | T-0014 |
| [T-0016](T-0016-report-export.md) | done | CSV and Markdown Report Export | T-0005 |
| [T-0017](T-0017-ui-state-and-daily-tray.md) | done | Restored UI State, Stable Resize and Daily Tray Total | T-0015 |
| [T-0018](T-0018-window-counters-and-width.md) | done | Explicit Window Counters and Stable Width Limit | T-0017 |
| [T-0019](T-0019-history-correction.md) | ready | History Correction and Manual Intervals | T-0018 |
| [T-0020](T-0020-ui-interaction-and-scroll.md) | done | Scroll Reflow, Window Edges and Selection Controls | T-0018 |
| [T-0021](T-0021-tracking-motion.md) | done | Calm Tracking Activity Indicators | T-0020 |
| [T-0022](T-0022-startup-window-recovery.md) | done | Recover Collapsed Startup Windows | T-0021 |
| [T-0023](T-0023-header-drag-and-centering.md) | done | Header Dragging and Centered Window Defaults | T-0022 |
| [T-0024](T-0024-autostart-and-tray-launch.md) | done | Windows Sign-In Startup and Tray Launch | T-0023 |
| [T-0025](T-0025-settings-entry.md) | done | Visible Settings Entry | T-0024 |
| [T-0026](T-0026-stable-tracking-controls.md) | done | Stable Tracking Controls and Settings Cleanup | T-0025 |
| [T-0027](T-0027-motion-and-write-cadence.md) | done | Visible Activity Motion and Write Cadence | T-0026 |
| [T-0028](T-0028-gradient-border.md) | done | Gradient Tracking Border | T-0027 |

Task template: `templates/TASK.md` from the repository root.

Next task: [T-0019](T-0019-history-correction.md). T-0002 remains a separate
desktop acceptance gate and does not authorize disruptive real system tests.
