$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ReleaseWorkflow.ps1')
$python = Get-DocsPython
$paths = Get-ReleasePaths (Split-Path -Parent $PSScriptRoot)
$version = Get-ReleaseVersion $python
Assert-ProductionVersion $paths $version
$development = (Get-Item -LiteralPath (Join-Path $paths.Root 'target/debug/attention-focus.exe')).VersionInfo
if ($development.FileVersion -ne "$version.0" -or $development.ProductVersion -ne "$version-dev" -or $development.FileDescription -ne 'Attention Focus (Development)') { throw 'Development EXE metadata does not match the current version.' }
& $python (Join-Path $PSScriptRoot 'release_tools.py') verify
if ($LASTEXITCODE -ne 0) { throw 'Production release verification failed.' }
& $python (Join-Path $PSScriptRoot 'release_tools.py') verify-icons
if ($LASTEXITCODE -ne 0) { throw 'Executable icon resource verification failed.' }
Write-Output "Production and Development file metadata verified: $version"
