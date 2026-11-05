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
| [T-0019](T-0019-history-correction.md) | done | History Correction and Manual Intervals | T-0018 |
| [T-0020](T-0020-ui-interaction-and-scroll.md) | done | Scroll Reflow, Window Edges and Selection Controls | T-0018 |
| [T-0021](T-0021-tracking-motion.md) | done | Calm Tracking Activity Indicators | T-0020 |
| [T-0022](T-0022-startup-window-recovery.md) | done | Recover Collapsed Startup Windows | T-0021 |
| [T-0023](T-0023-header-drag-and-centering.md) | done | Header Dragging and Centered Window Defaults | T-0022 |
| [T-0024](T-0024-autostart-and-tray-launch.md) | done | Windows Sign-In Startup and Tray Launch | T-0023 |
| [T-0025](T-0025-settings-entry.md) | done | Visible Settings Entry | T-0024 |
| [T-0026](T-0026-stable-tracking-controls.md) | done | Stable Tracking Controls and Settings Cleanup | T-0025 |
| [T-0027](T-0027-motion-and-write-cadence.md) | done | Visible Activity Motion and Write Cadence | T-0026 |
| [T-0028](T-0028-gradient-border.md) | done | Gradient Tracking Border | T-0027 |
| [T-0029](T-0029-hidden-animation.md) | done | Suspend Invisible Animation | T-0028 |
| [T-0030](T-0030-report-totals-and-note-recall.md) | done | Report Totals, Project Notes and Time Pickers | T-0029 |
| [T-0031](T-0031-tray-window-activation.md) | done | Bring the Window to the Front on Tray Click | T-0029 |
| [T-0032](T-0032-report-period-mode.md) | done | Report Period That Follows the Current Day | T-0030 |
| [T-0033](T-0033-day-boundary-accounting.md) | done | Day-Boundary Intervals and Verified Totals | T-0018 |
| [T-0034](T-0034-public-release-readiness.md) | in_progress | Public GitHub Release Readiness | T-0035 |
| [T-0035](T-0035-application-projects.md) | done | Application-Owned Projects and Tasks | T-0033 |
| [T-0036](T-0036-activity-visualization.md) | done | Graphical Activity Views | T-0035 |
| [T-0037](T-0037-code-signing.md) | blocked | Free Code Signing Through SignPath Foundation | T-0034; first public release |
| [T-0038](T-0038-scroll-bars-themes-settings.md) | done | Scroll Bars, Themes and Interval Setting | T-0036 |
| [T-0039](T-0039-interface-languages.md) | done | Interface Languages | T-0038 |
| [T-0040](T-0040-single-exe-release.md) | done | One EXE as the Release | T-0039 |
| [T-0041](T-0041-info-marks-and-about.md) | done | Info Marks and an About Button | T-0040 |
| [T-0042](T-0042-license-wording-and-file-properties.md) | done | License Wording, Microsoft Terms and File Properties | T-0041 |
| [T-0043](T-0043-project-site.md) | done | Project Site | T-0034 for going live |
| [T-0044](T-0044-site-link-in-the-app.md) | in_progress | A Link to the Project Site in the Application | T-0043; public repository |
| [T-0045](T-0045-dependency-alerts.md) | done | Dependency Alerts and Scanning Setup | public repository |

Task template: `templates/TASK.md` from the repository root.

Next check: the project panel (T-0035), native tray activation (T-0031) and a real midnight
crossing (T-0032, T-0033) in Development, using fresh desktop permission.
Next steps: publishing through [T-0034](T-0034-public-release-readiness.md) and then
[T-0037](T-0037-code-signing.md) needs the owner. T-0002 remains a separate desktop acceptance gate and does not authorize disruptive
system tests.
