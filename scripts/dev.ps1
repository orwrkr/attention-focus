param([switch] $BuildOnly)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
Push-Location $root
try {
    # Debug defaults to Development and never replaces the distributed production EXE.
    & cargo build --locked
    if ($LASTEXITCODE -ne 0) { throw "Development build failed with exit code $LASTEXITCODE." }
    $executable = Join-Path $root 'target/debug/attention-focus.exe'
    if (-not $BuildOnly) {
        & $executable --profile development
        if ($LASTEXITCODE -ne 0) { throw "Development app exited with code $LASTEXITCODE." }
    } else { Write-Output $executable }
} finally { Pop-Location }
