# T-0006: Window Persistence, Branding, and Tray Toggle

Status: done.
Dependencies: T-0002 implementation; user-authorized continuation before manual gate.

## Scope

Remember window geometry, embed a shared icon family, and implement durable unassigned
focus sessions controlled by native tray double click. Determine EXE deployment files.
Do not add MetaPlan access or claim the full reporting/session roadmap is complete.

## Acceptance criteria

- [x] Position, size, and maximization use eframe persistence outside the executable directory.
- [x] Executable resources and window icon share the logo; tray has two distinct states.
- [x] Double-click handler controls a durable timer; only committed state updates the icon.
- [x] Exit stops tracking; restart does not count downtime or resume silently.
- [x] Tests cover persistence, interrupted recovery, and no overlapping active sessions.
- [x] Build, formatting, lint checks, and executable dependency inspection pass.
- [x] Manual verification limitations are recorded; interactive acceptance remains in T-0002.

## Checks

2026-10-27: cargo fmt --check, eight tests, Clippy with -D warnings, and Windows x64
release build passed. The lint run emitted only incremental-cache hard-link fallback
warnings from the environment, not source lints.

dumpbin /DEPENDENTS shows only Windows libraries: kernel32.dll,
api-ms-win-core-synch-l1-2-0.dll, user32.dll, shell32.dll, comctl32.dll, gdi32.dll,
combase.dll, dwmapi.dll, ole32.dll, uxtheme.dll, bcryptprimitives.dll, OPENGL32.dll,
imm32.dll, and ntdll.dll. No SQLite or VC runtime DLL is imported.
The PE includes an .rsrc section; product/file description is Attention Focus.
Generated ICO artwork was rendered and visually inspected (app, running, stopped).

Windows Computer Use window discovery timed out twice and once more after resetting
the helper. No interactive geometry restoration, double-click routing, or Windows
shell appearance is claimed. The implementation is complete for this task; T-0002
retains the outstanding manual acceptance gate. Clean-machine graphics support and
short sleep/lock handling remain limitations, documented in docs/DEVELOPMENT.md.

## Handoff

The distributable folder dist/AttentionFocus contains only attention-focus.exe.
Runtime data lives in LocalAppData/AttentionFocus. Complete interactive acceptance
when the helper is available, then continue source settings and read-only parsing.
