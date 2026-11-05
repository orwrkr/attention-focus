$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ReleaseWorkflow.ps1')
$python = Get-DocsPython
$paths = Get-ReleasePaths (Split-Path -Parent $PSScriptRoot)
$version = Get-ReleaseVersion $python
Assert-ProductionVersion $paths $version
$development = (Get-Item -LiteralPath (Join-Path $paths.Root 'target/debug/attention-focus.exe')).VersionInfo
if ($development.FileVersion -ne "$version.0" -or $development.ProductVersion -ne "$version-dev" -or $development.FileDescription -ne 'Attention Focus (Development)') { throw 'Development EXE metadata does not match the current version.' }
# Microsoft's documentation requires CompanyName, and its runtime terms ask for a copyright notice.
$copyright = (Get-Content -LiteralPath (Join-Path $paths.Root 'LICENSE') | Where-Object { $_ -like 'Copyright (c) *' } | Select-Object -First 1).Trim()
foreach ($file in @((Get-Item -LiteralPath $paths.Output).VersionInfo, $development)) {
    if ($file.LegalCopyright -ne $copyright -or [string]::IsNullOrWhiteSpace($file.CompanyName)) { throw "EXE copyright or company metadata is missing: '$($file.LegalCopyright)', '$($file.CompanyName)'." }
}
& $python (Join-Path $PSScriptRoot 'release_tools.py') verify
if ($LASTEXITCODE -ne 0) { throw 'Production release verification failed.' }
& $python (Join-Path $PSScriptRoot 'release_tools.py') verify-icons
if ($LASTEXITCODE -ne 0) { throw 'Executable icon resource verification failed.' }
Write-Output "Production and Development file metadata verified: $version"
