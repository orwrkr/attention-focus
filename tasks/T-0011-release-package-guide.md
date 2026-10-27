# T-0011: Clear Status Icons, User Guide, and Versioned Packages

Status: done.
Dependencies: T-0010.

## Scope

Make permanent application icons neutral and distinct from running/stopped tray states,
including development identification. Build both production and development EXEs.
Prepare an English user guide, reproducible PDF exporter, third-party notices, version
metadata, and an automatically refreshed production ZIP. Keep one canonical production
EXE and version archives/PDFs appropriately. Reject locked outputs and never stop apps
automatically. Document and enforce the update/version/package workflow.

## Checks

- All six icon states are distinct at 16/32/64 pixels. Permanent application identities
  are neutral blue/purple diamonds; only tray states indicate running/stopped. Reviewed
  actual artwork in the guide. Audited runtime window/tray and build-resource call sites.
- Both version 0.2.0 EXEs built. Windows FileVersion is 0.2.0.0; ProductVersion is 0.2.0
  for production and 0.2.0-dev for debug. Verified FileDescription and all eight embedded
  icon sizes against generated neutral production/development catalogs.
- English five-page PDF rendered with Poppler; every page visually inspected. Final
  packaged PDF is byte-identical to the reviewed render input. Page text and metadata
  checked with pypdf. No orphan pages, missing icons, or clipped content found.
- Formatting, 32 Rust tests, Clippy, five Python packaging tests, all PowerShell parsing,
  and existing locked-output/atomic-publication checks pass. Rust emitted only an
  environmental incremental-cache hard-link fallback warning.
- Production ZIP exact four-file allowlist, versions, source fingerprint, EXE build
  stamp, SHA-256 hashes, ZIP integrity, and loose manifest verified. Tests cover stale
  source/binary rejection, version-bump enforcement, missing notices, path escape,
  exclusion of synthetic private databases, and preservation of existing archives.
- Main build automatically refreshed the archive. Separately exercised PDF export:
  the exporter automatically rebuilt and verified the ZIP, then verify-release passed
  again for both EXEs. Canonical EXE stays stable; PDF and ZIP names include version.
- PE imports of both EXEs are Windows system libraries only. Only the EXE is required
  to run; the package also supplies guide, third-party notices, and manifest. Notices
  cover 136 resolved Windows runtime packages, embedded fonts, and Rust standard library.
  Override provenance is explicit, including unavailable published egui commit URLs.

No application was launched and no user database or MetaPlan file was changed. Native
shell icon caching/rendering, real tray interaction, sleep/lock, multi-monitor geometry,
and clean-machine execution remain part of the existing manual T-0005/T-0002 gate.
Embedded-resource checks and renderer review do not claim that gate is complete.

## Outputs and next step

- Production: dist/AttentionFocus/attention-focus.exe.
- Development: target/debug/attention-focus.exe.
- Guide: dist/AttentionFocus/User-Guide-0.2.0.pdf.
- Package: dist/AttentionFocus-0.2.0-windows-x64.zip.

Future production input changes require a Cargo version bump, scripts/build.ps1, PDF
page review if affected, and scripts/verify-release.ps1. Use Development for remaining
interactive checks; do not add synthetic production time or stop apps automatically.
