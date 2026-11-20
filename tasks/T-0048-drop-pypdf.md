# T-0048: Drop pypdf From the PDF Tools

Status: done.
Dependencies: T-0045.

## Goal

Stop depending on `pypdf`. No script imports it: the guide exporter uses only ReportLab and Pillow, and
`scripts/build.ps1` merely checked that it could be imported. Dependabot pull request #8 proposed
`pypdf` 7.1.0, whose 7.0.0 and 7.0.1 notes list security fixes for PDF parsing. No published advisory
affected the pinned 6.21.4, and no parsing code of it ever ran here, but an unused pin still draws
alerts and pull requests.

## Scope

- Remove `pypdf` from `scripts/requirements-docs.txt` and from the import check in `scripts/build.ps1`.
- Version 0.26.6 in `Cargo.toml` and `Cargo.lock`, since the change is not in 0.26.5. No build and no
  release, at the owner's request: the release waits for a more substantial change, which joins 0.26.6.

## Acceptance criteria

- [x] No project file pins, imports or checks for `pypdf`.
- [x] The guide PDF exports in a fresh environment built from the requirements file alone.
- [x] The release tool checks that CI runs still pass.
- [x] The version is 0.26.6, not built or packaged.

## Checks

- A fresh virtual environment from the new `requirements-docs.txt` holds ReportLab 5.1.0, Pillow 13.0.0
  and `charset-normalizer`, no `pypdf`. `scripts/export_user_guide.py` run in it produced a guide
  byte-identical to the one exported with the old environment; two exports with the old environment were
  identical as well, so the comparison is meaningful.
- With that environment, `scripts/test_release_tools.py` (10 tests) and `release_tools.py check-notices`
  passed, and so did `scripts/test-release-output.ps1`.
- `pypdf` was uninstalled from `target/docs-venv`, so the local build environment matches the file.
- `release_tools.py check-version` failed at 0.26.5 as designed: the requirements file and `build.ps1`
  are part of the release fingerprint, which no longer matched the 0.26.5 release folder. After the bump
  to 0.26.6 it passes, since no 0.26.6 folder exists, and `cargo metadata --locked --offline` accepts
  the lock file. `scripts/verify-release.ps1` still fails until 0.26.6 is built, because
  `dist/AttentionFocus` holds the 0.26.5 EXE.
- Not run locally: `scripts/build.ps1`, `scripts/verify-release.ps1`, Clippy and the Rust tests. No Rust
  source changed, and the owner postponed the build; CI runs Clippy and the tests on the push.

## Handoff

`main` is 0.26.6 and unbuilt. Further changes join 0.26.6 without another bump, as long as
`scripts/build.ps1` has not packaged it; build only when releasing it, since a packaged 0.26.6 would move
the next change to 0.26.7. Dependabot pull request #8 becomes obsolete once `main` with this change is
pushed; if Dependabot does not close it, close it with a note that `pypdf` is no longer a dependency.
Should a PDF check need `pypdf` again, install it in a separate environment rather than in the release
requirements.
