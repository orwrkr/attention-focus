# T-0009: One Release Output

Status: done.
Dependencies: T-0008.

## Scope

Use dist/AttentionFocus/attention-focus.exe as the only distributed executable.
Build and publish with scripts/build.ps1. A locked current or legacy EXE is an error,
never a reason to create another copy. Do not terminate an app automatically. If an
agent launched the blocking instance, offer to close it through Exit and wait for approval.
Remove the exact obsolete AttentionFocus-Update copy once it is unlocked.

## Checks

scripts/test-release-output.ps1 passed: locked current/legacy files are rejected
without changing the previous build; successful publication replaces the EXE and removes
the exact obsolete copy and empty directory. The temporary synthetic test tree is cleaned.
scripts/build.ps1 passed and published the canonical EXE. Only dist/AttentionFocus and
its attention-focus.exe remain under dist. No running app process was found before cleanup.
