# T-0039: Interface Languages

Status: done.
Dependencies: T-0038.

## Goal

Let the user choose the interface language. By default it follows the Windows language, and an
unsupported language falls back to English. See ADR-0035.

The owner chose English, Russian, German, French, Spanish, Portuguese (Brazil), Italian, Polish,
Turkish, Chinese (Simplified), Japanese, Korean and Arabic. Ukrainian is left out for now. The user
guide and export contents stay in English, as recommended and approved.

The same request renamed report labels: Running now, and Interrupted, not counted yet, which
appears only when there is such time. A `CLAUDE.md` now imports `AGENTS.md`.

## Acceptance criteria

- [x] Settings offers Same as Windows and each language by its own name; the choice survives a restart.
- [x] Same as Windows follows the Windows preference order, then English.
- [x] The list sets Same as Windows apart, then shows languages in character table order (test; the
  owner's request after 0.22.0, viewed in the real window during T-0040).
- [x] Every user-visible string in the running interface comes from `locales/`.
- [x] Plural forms follow each language's rules.
- [x] Chinese, Japanese, Korean and Arabic use fonts included with Windows, also for user data.
- [x] Arabic reads from right to left, including dates, numbers and wrapped sentences.
- [x] Export contents do not depend on the interface language (test).
- [x] The layout holds at the narrowest window in every language (tests for reports and counters).

## Checks

`cargo fmt --check`, Clippy and `cargo test --locked` passed. Tests check every language for all keys,
the same placeholders, the plural forms its rules need, glyphs in the bundled or Windows font, the
tray tooltip limit, overflow, and that exports are byte-identical across languages. Temporary failures
confirmed that the layout tests detect overflow and overlap.

Checked in the real Development window on 2026-11-05, at 150% scale in the light theme: Russian from
the Windows language with no choice made, then German, Chinese, Japanese, Korean and Arabic in
Settings and reports, and the language list. That check found what the tests could not: counter cards
overlapping in Arabic, Yu Gothic's plain faces sitting above the baseline, invented abbreviations for
Arabic dates, a version number reordered around its letter, and chart columns too narrow. All are fixed.

An independent review of the language code by another model family found one defect, now fixed with
a test: the Sessions section and its rows took their identity from their text, so they closed when
the language or a count changed. It found no problem with text order, fonts, exports or mirroring,
though it could not read the `unicode-bidi` source, which tests cover instead.

Translations are machine-made. The owner can review Russian, and the rest have had only automated
checks and a look at their rendering, so errors of meaning are possible.

Unverified: French, Spanish, Portuguese, Italian, Polish and Turkish in the real window, which tests
cover for glyphs and overflow; the tray menu and tooltip in other languages, which Windows draws;
typed Arabic and Chinese input; user data in those scripts; other scales. The language list took about
3 seconds to open for the first time in a debug build, because it reads about 30 MB of fonts; a release
build was not timed.

## Handoff

Corrections to a translation only need a change to its file in `locales/`. To add a language, see the
last paragraph of ADR-0035.
