$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ReleaseOutput.ps1')
$project = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$testRoot = Join-Path $project ('target/release-output-test-' + [guid]::NewGuid().ToString('N'))
$paths = Get-ReleasePaths $testRoot
[IO.Directory]::CreateDirectory((Split-Path -Parent $paths.Source)) | Out-Null
[IO.Directory]::CreateDirectory($paths.Directory) | Out-Null
[IO.Directory]::CreateDirectory($paths.LegacyDirectory) | Out-Null
[IO.File]::WriteAllText($paths.Source, 'New build')
[IO.File]::WriteAllText($paths.Output, 'Previous build')
[IO.File]::WriteAllText($paths.LegacyOutput, 'Obsolete build')
$lock = [IO.File]::Open($paths.Output, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::None)
$rejected = $false
try { Publish-ReleaseOutput $paths | Out-Null } catch { $rejected = $true } finally { $lock.Dispose() }
if (-not $rejected -or [IO.File]::ReadAllText($paths.Output) -ne 'Previous build' -or -not (Test-Path -LiteralPath $paths.Source)) {
    throw 'Locked output was not rejected without changing the old build.'
}
$lock = [IO.File]::Open($paths.LegacyOutput, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::None)
$rejected = $false
try { Publish-ReleaseOutput $paths | Out-Null } catch { $rejected = $true } finally { $lock.Dispose() }
if (-not $rejected -or [IO.File]::ReadAllText($paths.Output) -ne 'Previous build') { throw 'Locked legacy EXE was not rejected.' }
Publish-ReleaseOutput $paths | Out-Null
if ([IO.File]::ReadAllText($paths.Output) -ne 'New build' -or (Test-Path -LiteralPath $paths.Source) -or (Test-Path -LiteralPath $paths.LegacyDirectory)) {
    throw 'Release publication did not leave one distributed EXE.'
}
$unsigned = Join-Path $testRoot 'unsigned.exe'
[IO.File]::WriteAllText($unsigned, 'Not signed')
$refusal = ''
try { & (Join-Path $PSScriptRoot 'use-signed-exe.ps1') -SignedExe $unsigned | Out-Null } catch { $refusal = "$_" }
if ($refusal -notlike 'Refusing*signature status*') { throw "An unsigned EXE was not refused: $refusal" }
# Resolve and verify the exact temporary directory before recursive test cleanup.
$resolved = (Resolve-Path -LiteralPath $testRoot).Path
$allowed = [IO.Path]::GetFullPath((Join-Path $project 'target')) + [IO.Path]::DirectorySeparatorChar
if (-not $resolved.StartsWith($allowed, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe test cleanup target.' }
Remove-Item -LiteralPath $resolved -Recurse -Force
Write-Output 'Release output checks passed: locked current/legacy files, preservation, atomic replacement, obsolete-copy cleanup, and unsigned EXE refusal.'
