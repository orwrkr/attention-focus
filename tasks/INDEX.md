# Tasks

| ID | Status | Task | Dependencies |
| --- | --- | --- | --- |
| [T-0001](T-0001-architecture.md) | done | Architecture and first working slice | None |
| [T-0002](T-0002-desktop-spike.md) | blocked | Desktop and Storage Integration Spike | T-0001 |
| [T-0003](T-0003-source-settings.md) | done | Read-Only Source and Settings | T-0002 implementation; user override |
| [T-0004](T-0004-session-engine.md) | done | Durable Sessions and Recovery | T-0003 |
| [T-0005](T-0005-focus-ui-reports.md) | backlog | Focus Workflow and Time Reports | T-0004 |
| [T-0006](T-0006-desktop-state-branding.md) | done | Window Persistence, Branding, and Tray Toggle | T-0002 implementation; user override |
| [T-0007](T-0007-tray-click-tooltip.md) | done | Tray Single Click and Tracked-Time Tooltip | T-0006 |
| [T-0008](T-0008-system-clipboard.md) | done | System Clipboard | T-0003 |
| [T-0009](T-0009-release-output.md) | done | One Release Output | T-0008 |

Task template: `templates/TASK.md` from the repository root.
