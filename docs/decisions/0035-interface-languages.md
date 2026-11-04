# ADR-0035: Interface Languages

Date: 2026-11-05.
Status: accepted. Supersedes ADR-0002 in part: the application interface may be translated.
Basis: explicit user request and the owner's choice of languages.

## Context

The owner asked for selectable languages, following the Windows language by default and falling
back to English. The chosen set is English, Russian, German, French, Spanish, Portuguese (Brazil),
Italian, Polish, Turkish, Chinese (Simplified), Japanese, Korean and Arabic. Ukrainian is left out
for now by the owner's decision. The user guide PDF and export headings stay in English.

All project-authored content remains in English. Only the files in `locales/` hold other languages.

## Decision

**Own small tables, not a framework.** Each language is a flat JSON file in `locales/`, embedded in
the EXE and parsed on first use, using the existing `serde_json`. English is the source and the
fallback for anything missing. Keys are string literals, and tests check them against the source.

Fluent would add about a dozen crates to the shipped program and to its notices, for selectors and
number formatting that a timer does not need. `rust-i18n` adds macros, and gettext keys by English
text, which makes any wording change touch every language. Plural rules are the one part worth
taking from a library, but a handful of messages use them, and a table-tested function covers the
languages in the set. Revisit this if many more languages or richer messages arrive.

**Language choice.** Settings offers Same as Windows and each language by its own name. Same as
Windows takes the first supported entry in the Windows preference order, then English. The choice is
an interface preference in `window.ron`. Portuguese maps to Brazilian. Traditional Chinese maps to
nothing rather than to Simplified, which would be wrong for its readers.

**Fonts from Windows.** The bundled font covers Latin and Cyrillic. Chinese, Japanese, Korean and
Arabic come from fonts Windows includes, read only when needed: for the interface language, when
user data or typed text is in that script, and while the language list is open, since it shows each
name in its own script. Shipping these fonts would add tens of megabytes and license terms. This
also fixes project names in these scripts, which could not be displayed before. They stay in memory
until exit, so opening the list once costs about 30 MB until then.

**Arabic without bidirectional support in egui.** egui 0.37 shapes a run of one font correctly,
including joining and letter order, but places runs from left to right, and its source marks
bidirectional text as future work. Each Arabic word already forms its own run, because the bundled
font has spaces, digits and punctuation but no Arabic. So `bidi` gives egui the runs in the order
they appear on screen, using the `unicode-bidi` crate for the Unicode algorithm. A test checks real
glyph positions, so an egui upgrade that changes this fails visibly.

Three things follow from that:

- Text exists in two forms. Logical text is stored, composed and given to Windows, which orders it
  itself. Painted text goes to egui, at the last moment. Painting twice garbles it.
- Lines must be chosen before reordering, or the end of a sentence lands on the first row, so text
  that can wrap goes through `i18n::paragraph`.
- Inserted values are isolated, or Unicode rules would reverse the groups of a date next to Arabic.

A root layout that prefers the right side makes egui mirror rows and align text to the right.

## Alternatives

- Waiting for egui to support bidirectional text: no date is known, and the owner chose Arabic now.
- Rendering Arabic with a separate text engine: far more code than the interface justifies.
- Translating the user guide and exports: the guide would need layout review per language on every
  release, and tools reading exports need stable column names.

## Consequences

Arabic is not a fully mirrored interface. Checkboxes keep their box on the left, collapsing headers
their arrow, scroll bars stay on the right, and charts keep a left-to-right time axis. Text fields
show typed Arabic words in typing order, because egui edits logical text directly. These limits are
stated in the user guide.

Translations are machine-made and reviewed only by tests, except Russian, which the owner can read.
Tests check that every language has every key, the same placeholders, every plural form its rules
need, and glyphs in the bundled font for languages that rely on it. Text that appears before the
interface exists, such as a failure to open the profile, stays in English. The language of worker
messages follows the choice at the moment they are produced.

Adding a language means a new `locales/*.json` file, an enum entry, its plural rule, and its font if
needed. Contributors can correct a translation without touching code.
