# ADR-0041: Dependency and Code Scanning on GitHub

Date: 2026-11-06.
Status: accepted.
Basis: the owner turned on GitHub's security features after making the repository public, and added a
Dependabot file and a CodeQL workflow from GitHub's templates. T-0045.

## Context

Dependabot reported 91 alerts on its first scan. Ninety were for `pypdf` and `Pillow` in
`scripts/requirements-docs.txt`, the tools that export the user guide PDF. They run only on a developer
machine or a release runner, and only on the project's own Markdown and generated icons; nothing of them is
in the EXE. One was for the Rust crate `glib` 0.18, which `gtk` brings in through `muda` and
`libappindicator` for Linux. `cargo tree --target x86_64-pc-windows-msvc -i glib` prints nothing, so it is
not built for this Windows application. No release of `muda` or `tray-icon` moves to a fixed `glib` yet.

The template Dependabot file named no package ecosystem, and the CodeQL template referred to actions by
movable tags.

## Decision

- **Update the PDF tools** to the first fixed versions, `pypdf` 6.21.4 and `Pillow` 13.0.0, in a new
  application version, because the requirements file is part of the release fingerprint.
- **Leave `glib` as it is** and dismiss its alert as code that is not used, with the reason above. Revisit
  when `muda` or `tray-icon` moves to a fixed release.
- **Dependabot version updates** for `cargo` monthly, and for `pip` in `/scripts` and `github-actions`
  weekly, with minor and patch updates grouped per ecosystem. A changed `Cargo.lock` changes the embedded
  license notices, so a Cargo pull request fails CI until `scripts/build.ps1` regenerates them locally.
- **CodeQL** keeps the owner's languages, `actions`, `python` and `rust`, without a build, and its actions
  are pinned by commit like the other workflows.

## Consequences

Dependabot pull requests are proposals: each is checked, and a change to a fingerprinted input still needs
a version bump before release. Security updates arrive as they are published, independent of the schedule.
