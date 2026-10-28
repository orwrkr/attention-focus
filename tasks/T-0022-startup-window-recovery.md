# T-0022: Recover Collapsed Startup Windows

Status: done.
Depends on: T-0021.

## Scope

Fix restored maximized geometry collapsing into a tiny top-left window. Recover
already saved undersized geometry without resetting preferences, history or source.

## Acceptance

- [x] Never query zero-initialized synthetic MINMAXINFO to size restored maximization.
- [x] Preserve work-area/DPI/max-width handling and valid restored dimensions.
- [x] Repair saved dimensions below 420 by 360 logical pixels once at startup.
- [x] Cover missing native startup limits and collapsed geometry with regressions.
- [x] Check Development restart/maximize/restore and publish both verified EXEs/ZIP.

## Findings and plan

The reported saved normal geometry is 64 by 64 logical pixels at position zero.
The startup maximized path manually sends WM_GETMINMAXINFO with zeroed fields.
Pinned winit 0.30.13 consumes that message, only conditionally filling tracking limits;
before constraints are installed, zero tracking width can become zero maximized width.
Replace that query with direct work-area/frame calculation and tolerate absent native
limits in the subclass. Repair each invalid normal-size axis to the initial default,
preserving valid axes and all other preferences. Verify with synthetic Development only.

## Checks

70 Rust tests, formatting and Clippy passed. Tests cover collapsed/valid/oversized
normal axes at 100/125/150/200 percent scale and a native subclass whose original
handler leaves startup limits unset. The latter now produces a positive explicit
maximized width instead of zero, while normal original-handler limits remain respected.

Native Development 0.12.2 at the current 150-percent display scale recovered a seeded
64-by-64 normal geometry to 620-by-640. Maximizing, normal Exit, and relaunch restored
760-by-981 maximized client geometry; Restore returned to 620-by-640. Saved normal
geometry and maximized flag were inspected separately. Other preferences were retained,
no session was started, and Production settings/history/process were untouched.
Development exited normally. A pre-check settings backup remains beside Development
preferences. Initial test seeding rejected RON's trailing comma as JSON before any write;
the successful test replaced only the window field, preserving the rest of the file.

Both 0.12.2 EXEs, the matching eight-page PDF/notices and the automatic production ZIP
passed verify-release fingerprint, member hashes, allowlist, version metadata and eight
neutral icon resource sizes per profile. All eight rendered PDF pages were inspected.

Next: T-0019. Other monitors/DPI configurations and real power/session transitions
remain separate T-0002 gates.
