# ADR-0004: Native Rust UI and Transactional Local Storage

Date: 2026-10-26.
Status: superseded in part by [ADR-0043](0043-write-ahead-log.md), for the rollback
journal.
Basis: architecture task T-0001; implementation decision within the approved scope.

## Context

The application needs a compact project picker, timer, history, and Windows tray.
It has no existing web frontend. Low interaction cost and reliable session records
matter more than elaborate visual styling.

## Options

| Option | Benefits | Costs and constraints |
| --- | --- | --- |
| egui/eframe plus tray-icon | UI and domain code in Rust; native application without a web frontend | Custom-drawn controls; explicit tray/event-loop integration; accessibility and idle behavior require validation |
| Tauri 2 | Documented tray support; web styling and application packaging | Adds a web frontend and Windows WebView2 deployment considerations |
| Direct Windows controls | Close integration with Windows controls and lifecycle | More platform-specific UI and lifecycle code for the same first slice |

These are architectural judgments, not measured performance or size comparisons.

### Selection rationale clarification

The choice prioritizes a small implementation entirely in Rust, not proven superiority
in appearance, memory use, startup time, or accessibility. No competing prototypes or
comparative benchmarks were built. Rust also permits Tauri; it does not require egui.
Treat this UI choice as a working hypothesis until the integration gate passes.
Revisit it if visual polish or native interaction quality outweighs implementation
simplicity. The user accepted proceeding on this basis.

## Decision

Use egui/eframe for the desktop window and tray-icon for the Windows tray.
Use rusqlite with bundled SQLite for transactional history and settings.
Start with one Cargo package containing a library and desktop binary; keep modules
separate without prematurely splitting the project into multiple crates.

Use synchronous domain commands through a single application controller. Put database
ownership on one worker thread; the UI receives results and only changes confirmed
state after successful commits. Use bounded commands and handle duplicate clicks.
Do not introduce an async runtime for the initial local workflow.

SQLite transactions make stopping one session and starting the next atomic.
A unique partial index on a constant for open sessions enforces at most one open row.
Enable foreign keys on every connection. Use a conservative rollback journal and
FULL synchronous mode initially; do not introduce WAL until a demonstrated need.
Store settings in the same database to avoid coordinating multiple persistent files.

## Alternatives for storage

- JSON is easy to inspect but requires custom atomic replacement, validation, and
  history queries. Crash recovery and switching would add application complexity.
- An append-only text log requires replay, compaction, and correction semantics.
- SQLite provides transactions and queries without a server. Bundling requires a
  C build toolchain and increases build work; validate this in the first spike.

## Distribution and validation gate

Initial target: Windows x64 MSVC. Start with a release executable, not an installer,
service, updater, or automatic startup registration. Check runtime DLL requirements
on a clean Windows environment before calling a build portable.

The first implementation task must prove tray creation, hide/show, event wakeup,
explicit exit, and a bundled SQLite round trip. Keep the window visible if tray
initialization fails. Do not ship a hidden application that cannot be reopened.
If this gate fails, record evidence and revise this ADR rather than layering on hacks.

Resolve compatible dependency versions during T-0002 and commit Cargo.lock.
Documentation currently exposes eframe 0.37.0, tray-icon 0.26.1, and rusqlite 0.41.0;
these are research observations, not a tested combination or dependency pins.
The local Rust 1.98.0 MSVC toolchain is available; linking is not yet verified.

## Sources reviewed

- [eframe native application documentation](https://docs.rs/eframe/latest/eframe/)
- [tray-icon Windows event loop and event handling](https://docs.rs/tray-icon/latest/tray_icon/)
- [Tauri tray integration](https://v2.tauri.app/learn/system-tray/)
- [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/)
- [rusqlite and bundled SQLite options](https://github.com/rusqlite/rusqlite)
- [SQLite unique partial indexes](https://www.sqlite.org/partialindex.html)

## Consequences

The application remains local with English UI and Unicode data support. UI and tray
behavior still need real Windows verification. Memory use, startup time, accessibility,
font coverage, and hidden-window CPU use are not yet measured.
