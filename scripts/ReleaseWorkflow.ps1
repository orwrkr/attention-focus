. (Join-Path $PSScriptRoot 'ReleaseOutput.ps1')
. (Join-Path $PSScriptRoot 'Find-Python.ps1')

function Get-ReleaseVersion([string] $Python) {
    $value = & $Python (Join-Path $PSScriptRoot 'release_tools.py') version
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read the release version.' }
    return $value.Trim()
}

function Assert-OutputUnlocked([hashtable] $Paths, [string] $Path) {
    Assert-ReleasePath $Paths $Path
    if (Test-Path -LiteralPath $Path) {
        try {
            $handle = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
            $handle.Dispose()
        } catch { throw "Output is busy or unwritable: '$Path'. Close the owning app and retry. No alternate output will be created. $($_.Exception.Message)" }
    }
}

function Assert-PackageOutputs([hashtable] $Paths, [string] $Version) {
    foreach ($path in @((Join-Path $Paths.Directory "User-Guide-$Version.pdf"),(Join-Path $Paths.Directory 'THIRD-PARTY-NOTICES.txt'),(Join-Path $Paths.Directory 'release.json'),(Join-Path $Paths.Root "dist/AttentionFocus-$Version-windows-x64.zip"))) {
        Assert-OutputUnlocked $Paths $path
    }
}

function Assert-ProductionVersion([hashtable] $Paths, [string] $Version) {
    if (-not (Test-Path -LiteralPath $Paths.Output -PathType Leaf)) { throw 'Production EXE is missing. Run scripts/build.ps1 first.' }
    $info = (Get-Item -LiteralPath $Paths.Output).VersionInfo
    if ($info.ProductVersion -ne $Version -or $info.FileVersion -ne "$Version.0" -or $info.FileDescription -ne 'Attention Focus') {
        throw "Production EXE version does not match $Version. Run scripts/build.ps1 before packaging."
    }
}

function Publish-PackageFile([hashtable] $Paths, [string] $Source, [string] $Destination) {
    Assert-ReleasePath $Paths $Source
    Assert-OutputUnlocked $Paths $Destination
    if (Test-Path -LiteralPath $Destination) {
        [IO.File]::Replace($Source, $Destination, [System.Management.Automation.Language.NullString]::Value)
    } else { [IO.File]::Move($Source, $Destination) }
}
