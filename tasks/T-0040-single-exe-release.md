# T-0040: One EXE as the Release

Status: done.
Dependencies: T-0039.

## Goal

Ship a release as a single EXE, with licenses embedded as far as is sufficient. The owner chose this
over an installer for now. See ADR-0036.

## Acceptance criteria

- [x] The EXE carries the license of the application and of each third-party component (test).
- [x] Settings shows that text in every language, always from left to right.
- [x] The text fits the window at any width, with no horizontal scrolling, and only visible rows are
  laid out (tests, and the real window).
- [x] A release is exactly `attention-focus.exe`, `User-Guide-VERSION.pdf` and `release.json`, with no
  ZIP (tests).
- [x] A failed packaging keeps the previous release folder, and unknown files in it are not removed
  (tests).
- [x] Changing dependencies without refreshing the notices fails CI and the release workflow.
- [x] Documents describe the new release, and ADR-0011 and ADR-0033 point to ADR-0036.

## Checks

`cargo fmt --check`, Clippy, `cargo test --locked` (202 tests), `scripts/test_release_tools.py` (8
tests), `scripts/test-release-output.ps1`, `actionlint` on both workflows, `check-notices`,
`scripts/build.ps1`, `scripts/verify-release.ps1` and `git diff --check` passed. A temporary failure
confirmed that the viewer test detects text wider than the window.

The guide stays at nine pages, and pages 1 and 8 were reviewed after the change. An early version of
the text left a nearly empty page, so the note on licenses went to "What the application needs".

Checked in the real Development window on 2026-11-05, at 150% scale in the light theme: opening and
hiding the licenses, scrolling, a narrow window, Arabic and the return to Same as Windows, and the
sorted language list from T-0039. Opening took under 0.1 seconds in a debug build. That check found what
the tests had not: lines wider than the window needed sideways scrolling, and the horizontal bar lay
over the text when the window cut the area off. Rows now wrap to the width, so that bar is gone.

An independent review by another model family read the scripts, workflows, viewer and documents, and
found no defects. It ran nothing, so it could not confirm the workflow behavior.

Unverified: that the notices generated on a hosted runner match this machine byte for byte, which the
next CI run shows; the release workflow itself, which needs a version tag; the dark theme and other
scales in this view.

The reading of the licenses in ADR-0036 is the project's, not legal advice.

## Handoff

After changing dependencies or the toolchain, run `python scripts/release_tools.py update-notices` and
commit the result. An installer remains possible later and is not required by the licenses.
