# T-0043: Project Site

Status: done.
Dependencies: T-0034 for going live.

## Goal

A simple project site on GitHub Pages, prepared now and served once the repository is public (ADR-0040).

## Acceptance criteria

- [x] One static page: what the application is, screenshots, privacy, features, install, license, links.
- [x] No scripts, external requests, cookies or analytics; a Content Security Policy.
- [x] Light and dark themes; no horizontal scrolling at phone width.
- [x] A Pages workflow with pinned actions that runs only for a public repository.

## Checks

The page was assembled into `target/site-preview` as the workflow does and captured with headless Edge at
1280 pixels in light and dark, and inside a 390-pixel frame, because headless Edge lays out no narrower than
504 pixels. All three looked as intended.

Live since 2026-11-06. After 0.26.3 was published, the owner set the Pages source to GitHub Actions and
the Pages workflow ran from `workflow_dispatch`; build and deploy passed. The page, stylesheet, icon and both
screenshots return 200 with the expected types, the Content Security Policy is in place, GitHub serves it
over HTTPS with HSTS, and every link returns 200, with Download reaching the `v0.26.3` release. Headless Edge
captured the live page at 1280 pixels in the light theme, styled as in the preview.

## Handoff

After the repository becomes public, the next release adds a link to the site in the application (T-0044).
Once that release is published, set Settings, Pages, Source to GitHub Actions, then run the Pages workflow or
push a change under `site/`. Update the page's signing note when T-0037 takes effect.
