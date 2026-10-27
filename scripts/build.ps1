$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ReleaseOutput.ps1')
$paths = Get-ReleasePaths (Split-Path -Parent $PSScriptRoot)
try {
    # Fail before compilation when either known distributed EXE is locked.
    Assert-ReleaseOutput $paths
    Push-Location $paths.Root
    try {
        & cargo build --locked --release --target-dir (Join-Path $paths.Root 'target')
        if ($LASTEXITCODE -ne 0) { throw "Cargo build failed with exit code $LASTEXITCODE." }
    } finally { Pop-Location }
    Publish-ReleaseOutput $paths
} catch {
    Write-Error $_ -ErrorAction Continue
    exit 1
}
