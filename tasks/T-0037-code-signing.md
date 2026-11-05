# T-0037: Free Code Signing Through SignPath Foundation

Status: blocked.
Dependencies: T-0034.

## Goal

Sign release executables for free through SignPath Foundation, so Windows names a verified
signature instead of an unknown file. See ADR-0033.

**Blocked on:** a public repository and a first public release, which the program requires before
applying, and then the owner's application and SignPath's approval.
**Resumes when:** the repository is public and the first release is published.

## Prepared

- [x] MIT license, no proprietary components, and releases that describe their purpose.
- [x] `.github/workflows/release.yml`: builds through `scripts/build.ps1`, drafts a release from a
  matching tag and submits signing only when configured.
- [x] `scripts/use-signed-exe.ps1`: installs only a validly signed EXE, then rebuilds the release output.
- [x] `.github/signpath/artifact-configuration.xml`: the configuration to enter in SignPath.
- [x] "Code signing policy" section in README and in release notes, with roles and privacy.

## Owner steps

1. Make the repository public. It exists as a private repository, and CI passes there.
2. Push tag `vVERSION`, check the draft release and publish it. This is the first, unsigned release.
3. Turn on multi-factor authentication on GitHub, and on SignPath when the account exists.
4. Apply at <https://signpath.org/apply>.
5. After approval, in SignPath: add the predefined GitHub.com trusted build system and link it to
   the project, enter the artifact configuration, and create the signing policies.
6. In the GitHub repository settings, add the secret `SIGNPATH_API_TOKEN` and the variables
   `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG` and `SIGNPATH_SIGNING_POLICY_SLUG`.
7. Run the workflow manually with `test-signing`, then set `release-signing` and push the next tag.
   Approve the request in SignPath, and publish the draft.

## Acceptance criteria

- [ ] A tag produces a draft release containing a signed EXE, with a valid signature on a clean
  computer.
- [ ] The README states that signing is in effect, with the attribution line "Free code signing
  provided by SignPath.io, certificate by SignPath Foundation", and README, `SECURITY.md`,
  and the release notes no longer call releases unsigned.

## Checks

`actionlint`, `python scripts/test_release_tools.py` and `scripts/test-release-output.ps1` passed
on 2026-11-04, including an unsigned file being refused and a replaced EXE requiring a new stamp.
With an empty `CARGO_HOME`, the workflow's `cargo fetch` followed by the offline metadata call
that notices use also passed. An independent read of the workflow and scripts, checked against
the SignPath documentation, found no verified defect.

The tag `v0.24.1` ran the release workflow on a hosted runner on 2026-11-05: the build, the PDF
dependencies, verification and the draft release all passed, with the signing steps skipped.
Unverified, because signing needs a public project: a real signing request, the successful path through
`scripts/use-signed-exe.ps1` with a truly signed file, and the artifact configuration, which
follows the SignPath documentation but has not been accepted by SignPath.

## Handoff

Follow the owner steps. Do not create the repository, push, tag or publish without an explicit
instruction. Prepared application answers can be kept in the ignored `local/` directory.
