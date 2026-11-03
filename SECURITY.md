# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately through this repository's
**Security > Report a vulnerability** form on GitHub, not through a public issue. Include
the application version, your Windows version and the steps that reproduce the problem.
Remove private project names, notes and file paths from anything you attach.

Only the latest release receives fixes. This is a personal project maintained in spare
time, so response times are not guaranteed.

## What the application does

Attention Focus is a local Windows application. It has no network access, accounts,
telemetry or automatic updates, and it does not read window contents or keyboard input.
Its data stays in `%LOCALAPPDATA%/AttentionFocus`, in plain SQLite and text files that any
program running under your Windows account can read. It does not encrypt this data.

## Useful areas to report

- Report exports escaping the protections around formulas, Markdown or file destinations.
- Application data being written outside its profile directory, including through links.
- Crashes or data loss caused by a crafted history database or preferences file.

Other programs already running under your account with your permissions are outside this
threat model, as is replacing the application's files or directory links while it runs.

## Release integrity

Each release archive includes `release.json` with SHA-256 hashes of its files. Release
executables are not code-signed at this time, so Windows SmartScreen may show a warning.
