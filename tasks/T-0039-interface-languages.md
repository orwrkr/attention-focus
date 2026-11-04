# T-0039: Interface Languages

Status: blocked.
Dependencies: T-0038.

## Goal

Let the user choose the interface language. By default it follows the Windows language, and an
unsupported language falls back to English.

**Blocked on:** the owner choosing the set of languages and the scope below.
**Resumes when:** the owner has decided.

## Findings on 2026-11-04

- About 400 distinct English strings exist in application code, by a rough count that includes
  errors and export headings.
- The font bundled with egui covers Latin, Cyrillic and Greek, including German, French, Spanish,
  Portuguese, Italian, Polish, Turkish, Ukrainian, Czech, Dutch, Swedish and Romanian. It lacks
  full Vietnamese coverage.
- It has no Chinese, Japanese, Korean, Arabic, Hebrew, Hindi or Thai. Project names in those scripts
  cannot display today either.
- Windows includes fonts for Chinese, Japanese and Korean (`msyh.ttc`, `YuGothR.ttc`, `malgun.ttf`
  were present on the one machine checked), so they could load on demand with no growth in the EXE.
- egui 0.37 shapes text but has no bidirectional layout. Its source marks RTL support as future
  work, so Arabic and Hebrew interfaces are not feasible now.

## Decisions needed

1. Which languages to support.
2. Whether the user guide PDF and export headings are translated as well. Recommended: not at
   first. Exports keep stable English column names for other tools.

## Implementation notes

- Requires a new ADR, partly superseding ADR-0002: project-authored content stays in English, and
  translations live only in dedicated locale files. AGENTS.md changes accordingly.
- Research the library choice in that ADR, such as Fluent or embedded tables. Russian and Polish
  plurals need real plural rules.
- The Windows language comes from `GetUserPreferredUILanguages`, already reachable through
  `windows-sys`. Weekday and month names in charts must come from the locale files too.
- Keep tests independent of the selected language.
