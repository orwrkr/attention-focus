# ADR-0038: Microsoft Runtime Terms, License Wording and File Properties

Date: 2026-11-05.
Status: accepted.
Basis: explicit user request to check Microsoft's terms and the EXE's file properties, to keep the
MIT License, and to stop calling the program free in the sense of the GPL.

## Context

The EXE is linked with `+crt-static`, so it holds object code from `libucrt.lib` (Windows SDK), and
from `libcmt.lib` and `libvcruntime.lib` (Visual Studio). The notices did not mention it. The file
properties had no company or copyright, although Microsoft's `VERSIONINFO` documentation calls
`CompanyName` required. The About panel called the program "free software", which readers can take
to mean copyleft, or free of charge, while MIT guarantees nothing about later versions.

What Microsoft's documents say, as read on 2026-11-05:

- The Windows SDK redistribution list allows the `.lib` files under `ucrt\x64` "when built as part of
  your program". The SDK terms limit `.lib` code to the results of a linker.
- The Visual Studio 2026 distributable list names `VC\redist`, not `VC\Tools\MSVC\...\lib`, where the
  release static libraries are. Its terms do not mention static linking. Microsoft's CRT documentation
  describes static linking and marks only the debug libraries "Not redistributable". A Microsoft staff
  member wrote in 2022 that they found no guidance either, and no ruling followed. So static linking of
  those two libraries is customary and documented, but not expressly licensed.
- Both sets of terms ask the distributor to add significant functionality, to require distributors
  and end users to agree to terms that protect the code at least as much, and to indemnify Microsoft.
  They forbid using Microsoft's trademarks in a way that suggests endorsement, and the SDK terms ask
  for "your valid copyright notice on your programs".
- Visual Studio Community allows an individual to develop their own applications for any purpose.
  The Windows terms treat fonts as part of Windows, and the application only reads the fonts of the
  computer it runs on, without copying or shipping them.

This is the project's reading of those documents, not legal advice.

## Decision

**A copyright notice shows on the program.** `build.rs` reads the line from `LICENSE`, the one source.
It appears in About and in the file's `LegalCopyright`, and the holder is `CompanyName`.
`scripts/verify-release.ps1` fails when either is missing.

**The notices name the Microsoft code.** `third-party/microsoft-runtime-notice.txt` joins the generated
notices. It says that the MIT License does not cover those parts, and passes on the conditions that
protect them. It relies on notice rather than a click-through agreement, which would add a step for a
small tray program, and that may not be what Microsoft has in mind.

**The wording states only the license.** About now says that the program is distributed under the MIT
License and includes third-party components under their own licenses. It avoids "free" and "open
source" in every language. The second part stays true for code that is not open source, and the first
stays true while the repository is private.

**MIT stays.** The owner considered GPL-3.0, which would keep later versions open, and kept MIT.

**Marks line up with their text.** A mark took the height of an interactive row while labels are
shorter, which put it 2.5 points too low. It now takes the height of what it stands beside, or of the
first line when that wraps.

## Alternatives

- A dynamic C runtime: the Windows part is a system component, but `vcruntime` would need a separate
  redistributable, which ends the single EXE. Part of the startup code stays static in any case.
- A hybrid: static `vcruntime` with the system's `ucrt`, which Windows Update services. It reduces the
  Microsoft code in the EXE and needs Windows 10 or later. This is worth considering if SignPath or a
  security fix calls for it.
- The GNU target for Windows: no Microsoft libraries, but a different toolchain, and unverified here.

## Consequences

Because the runtime is linked statically, a Microsoft fix reaches users only through a new release,
linked with an updated toolset or SDK. Windows Update does not change an existing EXE. The libraries
come from Visual Studio and the Windows SDK on the build machine: GitHub refreshes its images weekly, and
a local build needs them updated separately. System DLLs that the EXE calls still update with Windows.
The owner takes on the indemnity in Microsoft's terms by distributing the EXE, whatever the notices
say. SignPath's conditions forbid proprietary components but allow system libraries, so the
application should mention the static runtime and ask.

Release builds on `windows-latest` link the copies of `libcmt.lib` and `libvcruntime.lib` from the
Visual Studio Enterprise 2026 that GitHub preinstalls, while the local copy is Community. Both sets of
terms, read on 2026-11-05, have the same Build Devices clause and, apart from punctuation, the same
distributable code section. That clause covers machines owned by the licensee, hosted on Azure "for
you", or managed by an authorized outsourcer and "dedicated solely to your use". A runner is a virtual
machine that GitHub provides in Azure, with software that GitHub installed, so the clause does not
plainly describe it, and it does not say whose license applies. Microsoft's Visual Studio licensing
guidance and GitHub's terms for Actions do not mention the case. GitHub, a Microsoft company, offers
these images for building software, so the practical risk looks low, but no written permission was
found. A local build under Community on the owner's machine avoids the question, but a release signed
through SignPath is expected to come from the workflow (T-0037).
