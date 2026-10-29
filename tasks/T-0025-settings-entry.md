# T-0025: Visible Settings Entry

Status: done.
Depends on: T-0024.

## Acceptance

- Keep a Settings control in the fixed header, outside the scrolling content.
- Open application settings above the counters and scroll them into view.
- Toggle the panel without changing startup registration or tracking.
- Build both canonical EXEs and refresh the versioned production package.

## Verification

74 Rust tests passed with permissions for isolated registry and directory-link
fixtures; the restricted run failed those four permission-dependent fixtures.
Formatting and Clippy passed. Native Development 0.14.2 verified the fixed Settings
button, opening/closing the panel and automatic reveal after scrolling away.
Neither startup registration nor tracking was changed. Exit was sent; subsequent
process lookup found no attention-focus process. The computer-use JS session was
reset immediately; the supported API exposes no separate mode-disconnect command.
Fresh user permission was obtained before this session.

Both EXEs and the production archive were built through scripts/build.ps1.
The initial 0.14.1 package was superseded by 0.14.2 for the final button appearance.
The nine-page PDF was rendered and visually reviewed. Release verification passed.
No schema change. Real Windows sign-in remains a separate T-0002 acceptance gate.

Next: T-0019 history correction/manual intervals, starting with validation and
compatibility decisions. Each new computer-use session requires fresh permission.
