# T-0044: A Link to the Project Site in the Application

Status: blocked.
Dependencies: T-0043; the repository being public.

**Blocked on:** the repository becoming public, by the owner's decision.
**Resumes when:** the repository is public. Then this goes into the next release, and only after that
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

1. Check that eframe opens a URL on Windows (`ctx.open_url`); otherwise use `ShellExecuteW` from
   `windows-sys`, which is already a dependency.
2. Add the link, a test that it is shown and requests that exact address, and the guide's About paragraph.
3. Raise the version, build, verify, tag and publish the release (T-0034).
4. Then enable Pages and run the Pages workflow (T-0043).

## Acceptance criteria

- [ ] The link appears in About in every language and opens the site in the browser.
- [ ] Nothing in the application contacts a server by itself.
- [ ] The release containing it is published before the site is enabled.
