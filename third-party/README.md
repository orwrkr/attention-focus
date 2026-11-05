# Third-Party License Inputs

These upstream texts fill gaps in published crate archives, whose workspace-level
license files were not included. sources.json records the upstream text locations and
installed crate commit identities. For the egui workspace, common MIT/Apache texts came
from the available eframe 0.37.0 commit; other published crate commit URLs returned 404.
Their identities are pinned separately; byte equality against unavailable commits is
not claimed. The generator checks installed crate identities. These are external license texts,
not an application source-code license or translations. Do not edit their legal wording.

The notices generator also collects license/notice files from resolved Windows runtime
dependencies, bundled font attribution files, and Rust standard-library notices from
the installed toolchain. Build-only Python tools are not shipped as application runtimes.
If a dependency has no available license input, packaging fails rather than omitting it.

Its output is `THIRD-PARTY-NOTICES.txt` in this directory. It is generated, so do not edit it;
run `python scripts/release_tools.py update-notices`. The application embeds this file and
shows it under About, which is how each copy of the EXE carries these texts (ADR-0036).
