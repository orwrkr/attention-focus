# T-0010: Pinned Window Actions and Runtime Profiles

Status: done.
Dependencies: T-0009.

## Scope

Keep Hide to tray and Exit outside scrolling content at the top. Use fixed Production
and Development subdirectories under LocalAppData/AttentionFocus, separate single-instance
locks, window identities, settings, history, and geometry. Debug defaults to development
and cannot select production. Release defaults to production; an explicit development
override is supported. Keep the production distribution canonical and development builds
internal to target/debug so production may run while development builds or tests execute.

Leave legacy root files untouched. Import only the previous MetaPlan source path into
new profile settings; never copy old history into a new production database. Existing
new production history must survive later restarts. Validate source overlap against the
shared application root, and reject linked profile directories.

## Checks

All 32 tests passed, along with formatting, Clippy with -D warnings, packaging checks,
the canonical production release build, and scripts/dev.ps1 -BuildOnly. Profile tests
verify debug/release defaults, rejection of debug Production, clean initial history,
unchanged legacy history/geometry/settings bytes, independent writes and source settings,
retention of new production history on restart, common-root containment rejection, and
linked-development alias rejection. Native Windows mutex tests hold Production and
Development simultaneously, reject same-profile duplicates, and verify identities remain
stable after directory creation. All test data is synthetic and temporary.

Pinned actions are structurally outside the scrolling body. Visual verification of
the new header and two real app windows remains unperformed: Windows helper list_windows
timed out twice and once more after kernel reset. No app was launched for this check.
Only environment-related incremental-cache hard-link fallback warnings occurred.

Old real user files were not moved, cleared, or written by development tools. The new
Production database initializes empty on its first app launch. Profile setup seeds only
the old MetaPlan path, if valid. The new Development binary is an internal build artifact;
dist still contains exactly one production EXE.

## Handoff

Use dist/AttentionFocus/attention-focus.exe for normal use. Run scripts/dev.ps1 for
development alongside it. Verify pinned controls while scrolling expanded sections and
both profile labels in real Windows UI when the helper is operational. Continue remaining
T-0005/T-0002 manual tracking gates using Development, never fake Production sessions.
