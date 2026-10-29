# T-0024: Windows Sign-In Startup and Tray Launch

Status: done.
Depends on: T-0023.

## Acceptance

- [x] User-controlled Windows sign-in startup, off by default, isolated by profile.
- [x] Optional hidden tray launch only for the registered autostart invocation.
- [x] Manual launch remains visible; tray failure and errors remain accessible.
- [x] No automatic tracking, elevation, source writes or registration on ordinary launch.
- [x] Explain the existing subtle animation and Windows motion preference in settings.
- [x] Verify command quoting, registry isolation, visibility policy, both EXEs and ZIP.

## Verification

74 Rust tests, formatting and Clippy passed. Registry tests use only an isolated
HKCU Software/AttentionFocus/Tests subkey, remove their own values/key and preserve
unrelated values and the other profile's entry. They never write the real Run key.
Commands quote absolute EXE paths, retain Unicode, strip extended path prefixes,
enforce the documented 260-character limit, and include explicit profile/launch flags.
Parsing rejects duplicate or incoherent flags and keeps debug out of Production.

Native synthetic-window tests verify one-time SWP_SHOWWINDOW suppression, visibility
acknowledgement consumption and later explicit Show. Real eframe full tray-start/login
behavior has not been manually replayed; actual logoff/sign-in is not authorized here.

Native Development verified the two settings, existing motion enabled by Windows,
stopped tracking and no startup registration by default. A tray preference survived
Exit/relaunch while a normal manual launch still showed the window. A duplicate
--autostart --start-in-tray invocation exited quietly with code zero and left the
existing Development window running. The test preference was restored to off and
Development exited normally. No session was created, no real startup value was enabled,
and Production was not touched. Early clicks in a restored low window did not expand
settings; native maximize gave stable coordinates and the UI checks then passed.

Both canonical 0.14.0 EXEs, notices and the automatic production ZIP were built through
scripts/build.ps1. verify-release passed source fingerprint, ZIP allowlist/member hashes,
version metadata and eight neutral icon sizes per profile. All nine guide pages were
rendered and visually reviewed; the packaged PDF matched that preview byte for byte.

Next: T-0019 history correction/manual intervals. Real logon and full shell tray-start
acceptance remain separate T-0002 gates. Do not enable Production startup during tests.
