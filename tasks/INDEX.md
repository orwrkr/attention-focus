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
| [T-0016](T-0016-report-export.md) | ready | CSV and Markdown Report Export | T-0005 |

Task template: `templates/TASK.md` from the repository root.

Next implementation task: [T-0016](T-0016-report-export.md). T-0002 remains a separate
desktop acceptance gate and does not authorize disruptive real system tests.
