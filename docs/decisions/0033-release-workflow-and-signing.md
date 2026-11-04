# ADR-0033: Release Workflow and Free Code Signing

Date: 2026-11-04.
Status: accepted.
Basis: explicit user request to use the free SignPath option.

## Context

Release executables are unsigned, so Windows SmartScreen warns about them. SignPath Foundation
signs open source projects for free. Its terms expect an OSI license with no proprietary
components, a project already released in the form to be signed, binaries built verifiably from
source on GitHub-hosted runners, multi-factor authentication, a person approving each release,
restrictions on the signed file's metadata, and a "Code signing policy" on the home and download
pages. Until now, releases were only built on the owner's machine.

## Decision

**Build in a workflow.** `.github/workflows/release.yml` runs the same `scripts/build.ps1` on a
hosted Windows runner, so the pipeline, its guards and its verification stay in one place. A tag
`vVERSION` must match `Cargo.toml` and leads to a draft release that the owner publishes by hand.
A manual run builds the same files and keeps them as a workflow artifact.

**Signing is optional and off until configured.** The first public release has to exist before
applying, so the workflow must work unsigned. It submits a signing request only when the
repository variable `SIGNPATH_ORGANIZATION_ID` exists. A `test-signing` policy exercises the
path and leaves the package unsigned; only `release-signing` changes the package.

**A signed EXE goes through one script.** `scripts/use-signed-exe.ps1` refuses a file without a
valid signature or with different version metadata. It then replaces the production EXE, renews
the build stamp and rebuilds the archive, so `release.json` hashes the signed file. The
fingerprint still ties it to the same inputs. Signing covers the EXE only; the PDF, notices and
ZIP have no Authenticode signature, and `release.json` hashes them inside the archive.

**Least privilege.** The build job, which runs project and dependency code, can only read. A
separate job that runs no project code holds the permission to create the draft. Actions are
pinned by commit, and the SignPath token is passed only to the signing step.

**Metadata restrictions.** The artifact configuration, `.github/signpath/artifact-configuration.xml`,
requires the product name and the version of this build. SignPath keeps the active copy in its
project; the tracked file documents it for review.

**Say only what is true.** README and `SECURITY.md` say releases are not signed yet, and state the
roles and privacy sentence the policy needs. The attribution line, "Free code signing provided by
SignPath.io, certificate by SignPath Foundation", is added when signing is in effect.

## Alternatives

- Keep releases unsigned: this remains the state until approval, but does not address the warning.
- A purchased certificate or a paid signing service: an ongoing cost for a small free application.
- A self-signed certificate: other computers do not trust it, so the warning stays.
- Signing on the owner's machine: incompatible with verifiable CI builds, and it requires
  storing a private key locally.

## Consequences

Publishing needs the owner: creating the repository, a first release, the application, and
approving each signature. None of this has run yet, so the workflow and its hosted-runner
assumptions are unverified until the first tag. The certificate names SignPath Foundation, not the
owner, as the publisher. SmartScreen reputation builds over time, so a warning may continue for a
while after signing starts. Local builds from `scripts/build.ps1` remain unsigned. Production
users replacing the EXE see no behavior change.
