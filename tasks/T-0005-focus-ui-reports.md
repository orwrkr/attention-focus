# T-0005: Focus Workflow and Time Reports

Status: backlog.
Dependencies: T-0004.

## Goal

Deliver focus workflow and time reports within the initial architecture.

## Scope

Follow [the architecture](../docs/ARCHITECTURE.md) and ADR-0004. Keep all authored
content and UI in English. Preserve external Unicode data and the read-only boundary.
Do not include later tasks or deferred product features.

## Plan

Implement the acceptance criteria in order, then verify and update project context.

## Acceptance criteria

- [ ] Connect source settings, project search, optional task, Start/Stop/Switch, and tray controls to the controller.
- [ ] Show confirmed timer state and actionable storage/source errors in English.
- [ ] Display session history and project/task totals for a selected date range with uncertain time separated.
- [ ] Test clipped intervals, midnight and daylight-saving boundaries, Unicode titles, and project archival.
- [ ] Manually verify the full settings-to-report workflow and record release build instructions.
- [ ] Document deferred export, editing, shortcuts, autostart, and installer work without presenting them as delivered.

## Checks

Not started. Record commands, results, and manual evidence here.

## Handoff

Start after T-0004 is complete. Update this card and the task index together.
