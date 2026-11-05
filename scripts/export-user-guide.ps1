$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ReleaseWorkflow.ps1')
$python = Get-DocsPython
$paths = Get-ReleasePaths (Split-Path -Parent $PSScriptRoot)
$version = Get-ReleaseVersion $python
Assert-ProductionVersion $paths $version
Assert-PackageOutputs $paths $version
& $python (Join-Path $PSScriptRoot 'release_tools.py') check-version
if ($LASTEXITCODE -ne 0) { throw 'Production inputs changed. Bump the version and run scripts/build.ps1.' }
$staging = Join-Path $paths.Root 'target/package-staging'
$pdf = Join-Path $staging "User-Guide-$version.pdf"
Assert-ReleasePath $paths $pdf
& $python (Join-Path $PSScriptRoot 'export_user_guide.py') --output $pdf
if ($LASTEXITCODE -ne 0) { throw 'PDF export failed. Run scripts/setup-docs.ps1 or set ATTENTION_FOCUS_PYTHON.' }
Publish-PackageFile $paths $pdf (Join-Path $paths.Directory "User-Guide-$version.pdf")
# Every managed PDF update refreshes the release folder without another user step.
& (Join-Path $PSScriptRoot 'package-production.ps1')
if (-not $?) { throw 'PDF exported but release folder refresh failed.' }
