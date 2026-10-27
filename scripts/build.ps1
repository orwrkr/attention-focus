$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ReleaseWorkflow.ps1')
$paths = Get-ReleasePaths (Split-Path -Parent $PSScriptRoot)
try {
    # Fail before compilation when either known distributed EXE is locked.
    Assert-ReleaseOutput $paths
    $python = Get-DocsPython
    $version = Get-ReleaseVersion $python
    Assert-PackageOutputs $paths $version
    Assert-OutputUnlocked $paths (Join-Path $paths.Root 'target/debug/attention-focus.exe')
    & $python -c 'import reportlab, PIL, pypdf'
    if ($LASTEXITCODE -ne 0) { throw 'PDF dependencies are missing. Run scripts/setup-docs.ps1 or set ATTENTION_FOCUS_PYTHON.' }
    & $python (Join-Path $PSScriptRoot 'release_tools.py') check-version
    if ($LASTEXITCODE -ne 0) { throw 'Production inputs changed. Bump Cargo.toml version before publishing.' }
    $inputHash = & $python (Join-Path $PSScriptRoot 'release_tools.py') fingerprint
    if ($LASTEXITCODE -ne 0) { throw 'Cannot fingerprint production inputs.' }
    Push-Location $paths.Root
    try {
        & cargo build --locked --release --target-dir (Join-Path $paths.Root 'target')
        if ($LASTEXITCODE -ne 0) { throw "Cargo build failed with exit code $LASTEXITCODE." }
        & cargo build --locked --target-dir (Join-Path $paths.Root 'target')
        if ($LASTEXITCODE -ne 0) { throw "Development build failed with exit code $LASTEXITCODE." }
    } finally { Pop-Location }
    $staging = Join-Path $paths.Root 'target/package-staging'
    $guide = Join-Path $staging "User-Guide-$version.pdf"
    $notices = Join-Path $staging 'THIRD-PARTY-NOTICES.txt'
    Assert-ReleasePath $paths $guide
    & $python (Join-Path $PSScriptRoot 'export_user_guide.py') --output $guide
    if ($LASTEXITCODE -ne 0) { throw 'User guide PDF export failed; production has not been replaced.' }
    & $python (Join-Path $PSScriptRoot 'release_tools.py') notices --output $notices
    if ($LASTEXITCODE -ne 0) { throw 'Notice generation failed; production has not been replaced.' }
    $currentHash = & $python (Join-Path $PSScriptRoot 'release_tools.py') fingerprint
    if ($LASTEXITCODE -ne 0 -or $currentHash -ne $inputHash) { throw 'Production inputs changed during the build. Rerun the build after saving all files.' }
    Publish-ReleaseOutput $paths
    Publish-PackageFile $paths $guide (Join-Path $paths.Directory "User-Guide-$version.pdf")
    Publish-PackageFile $paths $notices (Join-Path $paths.Directory 'THIRD-PARTY-NOTICES.txt')
    & $python (Join-Path $PSScriptRoot 'release_tools.py') stamp
    if ($LASTEXITCODE -ne 0) { throw 'Production build stamp could not be written.' }
    & (Join-Path $PSScriptRoot 'package-production.ps1') -UsePreparedNotices
    if (-not $?) { throw 'Production archive refresh failed.' }
    Write-Output (Join-Path $paths.Root 'target/debug/attention-focus.exe')
} catch {
    Write-Error $_ -ErrorAction Continue
    exit 1
}
