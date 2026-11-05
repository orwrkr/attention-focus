# Installs the signed copy of the EXE that scripts/build.ps1 published, then the release folder.
param([Parameter(Mandatory)] [string] $SignedExe)
$ErrorActionPreference = 'Stop'
$signature = Get-AuthenticodeSignature -LiteralPath $SignedExe
if ($signature.Status -ne 'Valid') { throw "Refusing '$SignedExe': its signature status is $($signature.Status)." }
. (Join-Path $PSScriptRoot 'ReleaseWorkflow.ps1')
$python = Get-DocsPython
$paths = Get-ReleasePaths (Split-Path -Parent $PSScriptRoot)
$version = Get-ReleaseVersion $python
Assert-ProductionVersion $paths $version
$built = (Get-Item -LiteralPath $paths.Output).VersionInfo
$signed = (Get-Item -LiteralPath $SignedExe).VersionInfo
foreach ($name in 'ProductName', 'ProductVersion', 'FileVersion', 'FileDescription') {
    if ($signed.$name -ne $built.$name) { throw "Refusing '$SignedExe': its $name differs from the built EXE." }
}
Assert-OutputUnlocked $paths $paths.Output
[IO.File]::Copy($SignedExe, $paths.Output, $true)
& $python (Join-Path $PSScriptRoot 'release_tools.py') stamp
if ($LASTEXITCODE -ne 0) { throw 'Production build stamp could not be written.' }
& (Join-Path $PSScriptRoot 'package-production.ps1')
if (-not $?) { throw 'Release folder refresh failed.' }
