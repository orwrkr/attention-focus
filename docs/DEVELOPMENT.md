# Development and Preview

## Requirements

- Windows x64 with the Rust MSVC toolchain (verified toolchain: Rust/Cargo 1.98.0).
- MSVC C++ build tools and Windows SDK for native linking and bundled SQLite.
- Access to crates.io for the first dependency download.

The application currently uses eframe 0.37.0, tray-icon 0.26.1, rusqlite 0.41.0,
and tempfile 3.27.0. Cargo.lock records the complete resolved dependency graph.
tempfile owns the disposable SQLite probe directory and cleans it up after closing
the database. No settings or production data are stored by this preview.

## Commands

```powershell
cargo fetch --locked
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
cargo run --locked
```

The release executable is `target/release/attention-focus.exe`.
Build output is ignored by Git. No installer, automatic startup, or updater is added.
Runtime requirements on a clean Windows machine have not yet been verified; do not
describe this build as universally portable.

## Preview behavior

- Startup shows an English window and creates a tray icon.
- A worker verifies SQLite by committing a row, closing the connection, reopening
  read-only, checking the stored row, and removing its temporary directory.
- Closing the window or using Hide to tray keeps the process running.
- A left click on the tray icon or Show Focus restores the window.
- Exit in the window or tray menu terminates the preview.
- Tray setup failure leaves the window usable, disables hiding, and lets window
  close exit. The UI displays the failure instead of hiding silently.
- Tray callbacks wake the event loop; there is no periodic repaint loop.
- MetaPlan is not read or modified. Project selection, timing, and reports are not
  implemented in this integration spike.

## Manual verification checklist

1. Launch the executable; confirm the storage success message and readable layout.
2. Close the window; confirm the process remains alive and the window is hidden.
3. Use the tray icon to restore it, then repeat with Hide to tray and Show Focus.
4. Exit from the window; confirm the process stops and the icon disappears.
5. Relaunch and exit from the tray while hidden; confirm the process stops.
6. Record untested scenarios, including tray failure and clean-machine runtime support.

The preview is an integration gate, not a completed focus tracker or UI benchmark.

## Verification record (2026-10-26)

Formatting, the SQLite reopen test, Clippy, and a Windows x64 release build passed.
The final test/Clippy run reported only an environment-related incremental-cache
hard-link fallback to copying. No source lint failures occurred.
Interactive verification is pending: the Windows Computer Use helper timed out on
window discovery on three attempts, including one after resetting the helper session.
The executable was not launched through that helper. No hide/show/exit behavior,
idle CPU measurement, tray failure injection, or clean-machine compatibility is claimed.
