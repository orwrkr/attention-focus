# Contributing

Bug reports and focused pull requests are welcome. Please open an issue before starting
larger changes, so we can agree on the approach first.

## Build and test

You need Windows x64, the MSVC C++ build tools with the Windows SDK, and `rustup`, which
installs the toolchain pinned in `rust-toolchain.toml`.

```powershell
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
./scripts/dev.ps1
```

`./scripts/dev.ps1` runs the separate **Development** profile, so it never touches your
normal history. Tests create temporary databases, real windows that stay off-screen or
appear briefly, directory junctions, and keys under `HKCU\Software\AttentionFocus\Tests`.

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for packaging and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the design.

## Expectations for changes

- Keep changes small and focused, and add tests for behavior changes. Timing logic takes
  injected clocks, so tests do not need to sleep or change the system clock.
- Never lose or double-count time. History changes must be transactional, and schema
  upgrades must back up the database first.
- Write code, comments and documentation in English. User data such as project names and
  notes may be in any language and must be preserved unchanged.
- Add interface text to `locales/en.json` and show it through `i18n`, never as a literal in
  code. Other files in `locales/` hold translations, and corrections to them are welcome: they
  are machine-made. See [ADR-0035](docs/decisions/0035-interface-languages.md).
- Use synthetic data only. Do not commit personal projects, paths, histories or exports.
- Do not add cloud services, telemetry, window monitoring or productivity scoring.
- Record significant design decisions in `docs/decisions/`, using `templates/ADR.md`.

This repository is developed with AI coding agents. [AGENTS.md](AGENTS.md) is their
working contract, and `tasks/` and `docs/decisions/` keep the history of the work.

By contributing, you agree that your contribution is licensed under the
[MIT License](LICENSE).
