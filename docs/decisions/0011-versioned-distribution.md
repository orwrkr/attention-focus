# ADR-0011: Neutral Icons and Versioned Distribution

Date: 2026-10-27.
Status: accepted.
Basis: explicit user request.

## Decision

Use a neutral blue diamond for permanent application identity and a purple variant
for development. Only tray icons encode tracking: green play for running, gray pause
for stopped. Development tray states also have a purple D badge. All file/window icons
and tray variants use the same renderer; Windows resource and runtime selections are
explicit. An executable icon must never look like an active green tracking indicator.

Cargo.toml is the version authority. Windows numeric FileVersion is major.minor.patch.0;
ProductVersion is the release version, with -dev on debug builds. The window header,
PDF, manifest, ZIP name, and archive root identify the release version. Keep the loose
production EXE filename stable to avoid breaking shortcuts or introducing fallback copies.

scripts/build.ps1 builds both production and development, renders the English guide,
generates notices, publishes the canonical payload, and rebuilds/verifies its ZIP.
scripts/export-user-guide.ps1 also refreshes the ZIP automatically. Packaging is integrated
into these update paths rather than depending on a background daemon or a later manual step.
Development-only builds do not update production or its archive.

Ship exactly the EXE, User-Guide-VERSION.pdf, THIRD-PARTY-NOTICES.txt, and release.json.
Only the EXE is needed to run; the rest provide documentation, attribution, and version/hash
verification. Do not include settings, databases, logs, PDBs, source files, Python, or DLLs.
Resolved runtime license notices include bundled fonts and Rust's standard library.
Vendored license inputs record provenance where published crate archives omit their texts.

Use a source fingerprint and an EXE build stamp to reject stale or mislabeled packages.
Changed production source/guide/build inputs require a new Cargo version once that
version has been archived. Rebuilding identical inputs is allowed. Hash every payload
file and verify an explicit ZIP allowlist. Publish ZIPs by atomic replacement after
validation, retaining the previous archive on failure. Versioned older ZIPs are release
history, not alternate loose application copies. Locked outputs are errors, never
permission to create fallback filenames or terminate a running app.

PDF export uses pinned ReportLab/Pillow tooling and deterministic metadata. Generated
PDFs and release outputs remain ignored; Markdown, exporter, and notice sources are tracked.
