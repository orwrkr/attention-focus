# T-0008: System Clipboard

Status: done.
Dependencies: T-0003.

## Scope

Restore Ctrl+V in text fields using eframe's native clipboard feature. Default
features were disabled for the native build, unintentionally leaving only the
in-process clipboard fallback. The user confirmed direct typing works.

## Checks

The feature graph confirms eframe/clipboard -> egui-winit/clipboard -> arboard with
the native Windows clipboard backend. Dependencies are recorded in Cargo.lock.
Formatting, Clippy with -D warnings, and the release build passed. Existing tests
are recorded in the current context. No custom paste handler or clipboard polling added.

Interactive paste remains unverified. The existing EXE is locked by a running app;
a launch attempt returned the duplicate-instance message, which was dismissed.
The updated executable is in dist/AttentionFocus-Update/attention-focus.exe. Exit
the older app through its tray menu before launching this build; history is shared.
