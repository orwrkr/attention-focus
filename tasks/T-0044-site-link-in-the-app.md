# T-0044: A Link to the Project Site in the Application

Status: done.
Dependencies: T-0043; the repository being public.

The repository became public on 2026-11-06. This goes into the next release, and only after that
release is published does the site go live (owner's order, 2026-11-05).

## Goal

About shows a link to `https://orwrkr.github.io/attention-focus/`, so the release that people download
leads to the site.

## Scope

- A link in the About panel that opens the default browser on click. The application itself still makes no
  network requests, so the README and site statements stay true.
- The link text through `locales/*.json` in every language, and the address in one constant.
- In the same release, raise the five-second wait in
  `refused_interface_commands_answer_once_and_later_commands_still_work`, which failed once on a busy
  GitHub runner before the first event arrived (v0.25.2, first attempt).

## Plan

1. Done. eframe is built without its `links` feature, so `ctx.open_url` would only log a warning;
   `platform::open_in_browser` calls `ShellExecuteW` instead, with COM initialized for the call.
2. Done. `main.site` is a link in About with the address as its hover text; a failure shows
   `errors.site_not_opened`. The address is `attention_focus::SITE`, the manifest's `homepage`, and a
   test derives it from the repository address. The guide mentions it. Every five-second worker wait in
   `src/tracking/tests.rs` is now `WORKER_WAIT`, sixty seconds.
3. Version 0.26.0, then 0.26.1 with the PDF tool updates from T-0045, 0.26.2 with the first
   Dependabot version updates, and 0.26.3 with tidier license notices; 0.26.0 was never tagged. Build, verify, tag and publish the release
   (T-0034).
4. Done: Pages is enabled and the site is live (T-0043). The README links to it.

## Acceptance criteria

- [x] The link appears in About in every language and opens the site in the browser.
- [x] Nothing in the application contacts a server by itself.
- [x] The release containing it is published before the site is enabled.

## Checks

- `cargo fmt --check`, Clippy with warnings as errors, and 245 tests passed, including
  `the_site_is_the_pages_address_of_the_repository`.
- `scripts/build.ps1` built 0.26.0; `scripts/verify-release.ps1`, `scripts/test-release-output.ps1`,
  `release_tools.py check-notices` and `scripts/test_release_tools.py` passed; `git diff --check` was clean.
- A DeepSeek review of the diff found no defects. Its two cosmetic notes were applied: the guide's line
  wrapping and a clearer error text.
- In the Development window, in Russian and Arabic: About shows the link, right-aligned in Arabic, and
  hovering shows the address. One click produced no error, so the shell accepted the address. Whether the
  browser showed the page was left to the owner, since other windows were not inspected.
- The language was set back to following Windows afterwards.
- The owner then clicked the link and confirmed that the browser opened the address.
- The release workflow for `v0.26.1` passed on its first attempt. Its draft has the local build's
  source fingerprint, file hashes that match its manifest, the tracked notices inside the EXE, and file
  properties 0.26.1.0; it is not signed. The `v0.25.2` draft was deleted; its tag remains.
- 0.26.2 replaced it with the first Dependabot version updates (T-0045). Its release run passed on the
  first attempt, and its draft passed the same checks with file properties 0.26.2.0; it is not signed.
  The `v0.26.1` draft was deleted; its tag remains.
- 0.26.3 replaced it with tidier license notices. Its release run passed on the first attempt, and its
  draft passed the same checks with file properties 0.26.3.0; it is not signed. The `v0.26.2` draft was
  deleted; its tag remains.
- The owner had it published on 2026-11-06 as the first public release, marked latest. The EXE
  downloaded through `releases/latest/download/attention-focus.exe` has the SHA-256 of the checked draft.
