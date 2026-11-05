# T-0045: Dependency Alerts and Scanning Setup

Status: done.
Dependencies: the repository being public.

## Goal

Act on the first Dependabot alerts and finish the Dependabot and CodeQL files that the owner added from
GitHub's templates (ADR-0041).

## Acceptance criteria

- [x] `pypdf` and `Pillow` updated to versions without open alerts, and the guide PDF still builds.
- [x] The `glib` alert assessed: Linux only, not in the Windows build.
- [x] `.github/dependabot.yml` names real ecosystems, and the CodeQL actions are pinned by commit.
- [ ] The `glib` alert dismissed on GitHub, by the owner or with the owner's permission.

## Checks

- `scripts/requirements-docs.txt` now pins `pypdf` 6.21.4 and `Pillow` 13.0.0, installed in
  `target/docs-venv`. Version 0.26.1, since the file is part of the release fingerprint.
- `scripts/build.ps1`, `scripts/verify-release.ps1`, `scripts/test-release-output.ps1`,
  `release_tools.py check-notices`, `scripts/test_release_tools.py`, Clippy and 245 tests passed.
- The 0.26.1 guide PDF was rendered: ten pages, the icon legend that goes through Pillow is intact, and the
  Project site sentence is on page nine.
- `cargo tree --target x86_64-pc-windows-msvc -i glib` prints nothing.
- CodeQL had reported no code scanning alerts when this was written.

## Handoff

Dependabot should close its two security pull requests once `main` has these versions; if it does not,
close them. Check the first scheduled Dependabot pull requests against ADR-0041.
