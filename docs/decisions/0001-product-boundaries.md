# ADR-0001: Rust and Independent Time Tracking

Date: 2026-10-26.
Status: accepted.
Basis: explicit user instructions.

## Context

The user needs a simple desktop focus tracker with a tray interface, project/task
selection, start/stop, and time reports. Projects already exist in MetaPlan.
The user selected Rust and prohibited modifications to the source.

## Decision

Implement the application in Rust. Configure the MetaPlan/Instance directory in
settings. Make the integration read-only. Store sessions and settings separately
from MetaPlan; exports must also remain outside the source directory.

## Alternatives

- C# / .NET was discussed but superseded by the explicit choice of Rust.
- Saving sessions directly in MetaPlan violates the read-only boundary.

## Consequences

MetaPlan remains independent. The timer does not modify cycles, statuses, or tasks.
The application needs its own storage and historical project/task titles.
This ADR does not select the UI toolkit, tray library, or storage format.
