param([switch] $UsePreparedNotices)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ReleaseWorkflow.ps1')
$python = Get-DocsPython
$paths = Get-ReleasePaths (Split-Path -Parent $PSScriptRoot)
$version = Get-ReleaseVersion $python
Assert-ProductionVersion $paths $version
Assert-PackageOutputs $paths $version
& $python (Join-Path $PSScriptRoot 'release_tools.py') check-version
if ($LASTEXITCODE -ne 0) { throw 'Bump the app version before publishing changed production inputs.' }
if (-not $UsePreparedNotices) {
    $staging = Join-Path $paths.Root 'target/package-staging'
    Assert-ReleasePath $paths (Join-Path $staging 'THIRD-PARTY-NOTICES.txt')
    [IO.Directory]::CreateDirectory($staging) | Out-Null
    $notice = Join-Path $staging 'THIRD-PARTY-NOTICES.txt'
    & $python (Join-Path $PSScriptRoot 'release_tools.py') notices --output $notice
    if ($LASTEXITCODE -ne 0) { throw 'Third-party notice generation failed.' }
    Publish-PackageFile $paths $notice (Join-Path $paths.Directory 'THIRD-PARTY-NOTICES.txt')
}
& $python (Join-Path $PSScriptRoot 'release_tools.py') package
if ($LASTEXITCODE -ne 0) { throw 'Production archive creation failed.' }
& $python (Join-Path $PSScriptRoot 'release_tools.py') verify
if ($LASTEXITCODE -ne 0) { throw 'Production archive verification failed.' }
