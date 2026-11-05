# ADR-0040: A Static Project Site on GitHub Pages

Date: 2026-11-05.
Status: accepted.
Basis: explicit user request for a simple project site, and the owner's choice of GitHub Pages over
Firebase, to go live when the repository becomes public. T-0043.

## Context

The project needs a page that says what the application is and where to download it. The repository is
private for now. The application promises no network access or telemetry, so the site should not undercut
that with trackers.

## Decision

**GitHub Pages, from this repository.** `site/` holds a hand-written `index.html`, `style.css` and the
application icon. `.github/workflows/pages.yml` assembles them with the two README screenshots from
`docs/images/` and deploys through the Pages actions, pinned by commit like the other workflows. No
generator, framework or build step.

**It waits for a public repository.** The free plan serves Pages only for public repositories, so the
build job runs only when the repository is public. After that, the owner sets Settings, Pages, Source to
GitHub Actions once, because the workflow token cannot enable Pages.

**Downloads stay on GitHub Releases.** The page links to the latest release instead of copying the EXE,
so there is one copy with its hashes and, later, its signature. The guide link points to
`docs/user-guide.md` in the repository.

**Nothing from other servers.** No scripts, web fonts, analytics or cookies. A Content Security Policy
allows only the site's own images and stylesheet. The page follows the reader's light or dark preference
and works at phone width.

**English only**, like other project content. Translated pages need a new instruction.

## Alternatives

- Firebase Hosting: works with a private repository, but needs a Google project, a deployment token in
  repository secrets and its own CLI. Its free tier is described as 360 MB of transfer per day, too little
  to host the EXE. Its database and functions are not needed.
- A generator such as MkDocs or Hugo: more structure than one page needs, and another toolchain.
- Cloudflare Pages: comparable, but another account, for no gain here.

## Consequences

The page repeats parts of the README, so a change to features, requirements or signing should update both.
`site/icon.png` is a copy of the generated application icon and must be replaced if the icon changes. The
download button fails until a release is published, since drafts are not "latest".
