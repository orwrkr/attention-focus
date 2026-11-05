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
504 pixels. All three looked as intended. The workflow has not run, since the repository is private, so the
deployment itself is unverified, as are the links, which need a public repository and a published release.

## Handoff

After the repository becomes public, set Settings, Pages, Source to GitHub Actions, then run the Pages workflow
or push a change under `site/`. Update the page's signing note when T-0037 takes effect.
