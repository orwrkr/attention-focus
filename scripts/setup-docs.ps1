$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
& python -m venv (Join-Path $root 'target/docs-venv')
if ($LASTEXITCODE -ne 0) { throw 'Python 3.11+ is required to create the documentation environment.' }
& (Join-Path $root 'target/docs-venv/Scripts/python.exe') -m pip install -r (Join-Path $PSScriptRoot 'requirements-docs.txt')
if ($LASTEXITCODE -ne 0) { throw 'Documentation dependency installation failed.' }
