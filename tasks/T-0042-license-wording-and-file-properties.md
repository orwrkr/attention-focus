# T-0042: License Wording, Microsoft Terms and File Properties

Status: done.
Dependencies: T-0041.

## Goal

Replace the words "free software" in About with a neutral statement, in every language. Keep MIT.
Align info marks with their text. Fill in the EXE's file properties. Check Microsoft's terms and
include what they ask for. See ADR-0038.

## Acceptance criteria

- [x] About states the MIT License and third-party licenses, with no "free" or "open source".
- [x] Each language was checked for those connotations, including by another model family.
- [x] A mark lines up with a label, button, checkbox or picker, and with the first line when it wraps
  (test, real window).
- [x] File properties hold every string Microsoft's documentation requires, and the copyright notice;
  `scripts/verify-release.ps1` checks them.
- [x] About shows the copyright notice from `LICENSE` (test).
- [x] The notices name the statically linked Microsoft code and pass on its conditions (test).

## Checks

`cargo fmt --check`, Clippy, `cargo test --locked` (210 tests), `scripts/test_release_tools.py`,
`scripts/test-release-output.ps1`, `check-notices`, `scripts/build.ps1`, `scripts/verify-release.ps1` and
`git diff --check` passed. A temporary failure confirmed that the alignment test detects a mark 2.5 points
too low, which is what the owner saw.

Another model family back-translated all twelve translations and searched every locale file. It found no
connotation of freedom, copyleft, free of charge, open source or sale, and suggested a closer Polish
wording, now used. Translations remain machine-made; the owner can review the Russian.

Checked in the real Development window on 2026-11-05, in Russian and Arabic: marks beside counters and
report lines, About with the copyright line, and the new text. Microsoft's terms were read from the Visual
Studio 2026 Community license document, the local Windows SDK 10.0.26100 and Windows licenses, and
Microsoft Learn; ADR-0038 lists the findings.

Unverified: the terms that apply to Visual Studio Enterprise on GitHub's runner; whether notice alone meets
Microsoft's requirement that end users agree to terms, and whether static linking of `libcmt.lib` and
`libvcruntime.lib` is covered, on which Microsoft has given no ruling; SignPath's view of the static
runtime. The release workflow has not run for this version, and the 0.24.1 draft still has the old wording.

## Handoff

Mention the static Microsoft runtime in the SignPath application (T-0037), and rebuild a release when
Microsoft fixes a runtime defect that matters here.
