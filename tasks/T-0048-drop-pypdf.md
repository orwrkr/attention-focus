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
- No new version and no build, at the owner's request: the release waits for a more substantial change.

## Acceptance criteria

- [x] No project file pins, imports or checks for `pypdf`.
- [x] The guide PDF exports in a fresh environment built from the requirements file alone.
- [x] The release tool checks that CI runs still pass.

## Checks

- A fresh virtual environment from the new `requirements-docs.txt` holds ReportLab 5.1.0, Pillow 13.0.0
  and `charset-normalizer`, no `pypdf`. `scripts/export_user_guide.py` run in it produced a guide
  byte-identical to the one exported with the old environment; two exports with the old environment were
  identical as well, so the comparison is meaningful.
- With that environment, `scripts/test_release_tools.py` (10 tests) and `release_tools.py check-notices`
  passed, and so did `scripts/test-release-output.ps1`.
- `pypdf` was uninstalled from `target/docs-venv`, so the local build environment matches the file.
- `release_tools.py check-version` now fails as designed: the requirements file and `build.ps1` are part
  of the release fingerprint, which no longer matches the 0.26.5 release folder. For the same reason
  `scripts/verify-release.ps1` reports the folder as stale until the next release.
- Not run: `scripts/build.ps1`, `scripts/verify-release.ps1`, Clippy and the Rust tests. No Rust source or
  shipped file changed, and a build would need the version bump the owner postponed.

## Handoff

The next release must bump `Cargo.toml` to 0.26.6 or later before `scripts/build.ps1`, as for any change to
a fingerprinted input. Dependabot pull request #8 becomes obsolete once `main` with this change is pushed;
if Dependabot does not close it, close it with a note that `pypdf` is no longer a dependency. Should a PDF
check need `pypdf` again, install it in a separate environment rather than in the release requirements.
