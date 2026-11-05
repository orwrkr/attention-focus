$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ReleaseWorkflow.ps1')
$python = Get-DocsPython
$paths = Get-ReleasePaths (Split-Path -Parent $PSScriptRoot)
$version = Get-ReleaseVersion $python
Assert-ProductionVersion $paths $version
Assert-PackageOutputs $paths $version
& $python (Join-Path $PSScriptRoot 'release_tools.py') check-version
if ($LASTEXITCODE -ne 0) { throw 'Bump the app version before publishing changed production inputs.' }
& $python (Join-Path $PSScriptRoot 'release_tools.py') package
if ($LASTEXITCODE -ne 0) { throw 'Release folder creation failed.' }
& $python (Join-Path $PSScriptRoot 'release_tools.py') verify
if ($LASTEXITCODE -ne 0) { throw 'Release folder verification failed.' }
