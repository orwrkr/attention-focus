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
- [x] The `glib` alert dismissed on GitHub as not used, with the owner's permission and the reason above.
- [x] The first Dependabot version updates (#4 to #6) applied locally, with the notices regenerated.

## Checks

- `scripts/requirements-docs.txt` now pins `pypdf` 6.21.4 and `Pillow` 13.0.0, installed in
  `target/docs-venv`. Version 0.26.1, since the file is part of the release fingerprint.
- `scripts/build.ps1`, `scripts/verify-release.ps1`, `scripts/test-release-output.ps1`,
  `release_tools.py check-notices`, `scripts/test_release_tools.py`, Clippy and 245 tests passed.
- The 0.26.1 guide PDF was rendered: ten pages, the icon legend that goes through Pillow is intact, and the
  Project site sentence is on page nine.
- `cargo tree --target x86_64-pc-windows-msvc -i glib` prints nothing.
- CodeQL had reported no code scanning alerts when this was written.
- The first version updates were applied in one local change instead of merging the pull requests, since
  the Cargo one fails CI until the notices are regenerated: reportlab 5.1.0 (#4), jiff 0.2.39, rusqlite
  0.41.1 and serde_json 1.0.154 (#5), and miniz_oxide 0.9 (#6). Version 0.26.2.
- ReportLab's 5.0 notes change image fetching from remote hosts, and deprecate the renderPM C backends and
  pyRXP; the guide exporter uses none of them. The 0.26.2 guide, rendered page by page, differs from the
  0.26.1 one only in the version digits of the header and footer.
- Clippy, 245 tests, `scripts/build.ps1`, `scripts/verify-release.ps1`, `scripts/test-release-output.ps1`,
  `check-notices` and `scripts/test_release_tools.py` passed. `git diff --check` flags one generated notice
  line, `Authors: ` with a trailing space, because serde_json 1.0.154 lists no authors; thirty older entries
  already have that form. The generator could omit the empty field in a later version.

## Handoff

Dependabot should close pull requests #2 to #6 once `main` has these versions; if it does not, close
them. Later pull requests follow ADR-0041.
