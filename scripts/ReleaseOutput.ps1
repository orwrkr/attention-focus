Set-StrictMode -Version Latest

function Get-ReleasePaths([string] $Root) {
    $rootPath = [IO.Path]::GetFullPath($Root).TrimEnd([char[]]'\/')
    @{
        Root = $rootPath
        Source = Join-Path $rootPath 'target/release/attention-focus.exe'
        Directory = Join-Path $rootPath 'dist/AttentionFocus'
        Output = Join-Path $rootPath 'dist/AttentionFocus/attention-focus.exe'
        LegacyDirectory = Join-Path $rootPath 'dist/AttentionFocus-Update'
        LegacyOutput = Join-Path $rootPath 'dist/AttentionFocus-Update/attention-focus.exe'
    }
}

function Assert-ReleasePath([hashtable] $Paths, [string] $Path) {
    $absolute = [IO.Path]::GetFullPath($Path)
    if (-not $absolute.StartsWith($Paths.Root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Release path is outside the project: $absolute"
    }
    $current = $absolute
    while ($current -ne $Paths.Root) {
        if (Test-Path -LiteralPath $current) {
            $entry = Get-Item -LiteralPath $current -Force
            if ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "Linked release paths are not supported: $current"
            }
        }
        $current = Split-Path -Parent $current
    }
}

function Assert-ReleaseOutput([hashtable] $Paths) {
    foreach ($path in @($Paths.Source, $Paths.Output, $Paths.LegacyOutput)) {
        Assert-ReleasePath $Paths $path
    }
    if (Test-Path -LiteralPath $Paths.LegacyDirectory) {
        foreach ($entry in Get-ChildItem -LiteralPath $Paths.LegacyDirectory -Force) {
            if ($entry.FullName -ne $Paths.LegacyOutput) {
                throw "Unexpected entry in obsolete release directory: $($entry.FullName)"
            }
        }
    }
    foreach ($path in @($Paths.Output, $Paths.LegacyOutput)) {
        if (Test-Path -LiteralPath $path) {
            try {
                $handle = [IO.File]::Open($path, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
                $handle.Dispose()
            } catch {
                throw "Cannot replace release EXE '$path'. Exit the running app and retry. No alternate EXE will be created. $($_.Exception.Message)"
            }
        }
    }
}

function Publish-ReleaseOutput([hashtable] $Paths) {
    Assert-ReleaseOutput $Paths
    if (-not (Test-Path -LiteralPath $Paths.Source -PathType Leaf)) {
        throw "Compiled EXE is missing: $($Paths.Source)"
    }
    [IO.Directory]::CreateDirectory($Paths.Directory) | Out-Null
    try {
        if (Test-Path -LiteralPath $Paths.Output) {
            [IO.File]::Replace($Paths.Source, $Paths.Output, [System.Management.Automation.Language.NullString]::Value)
        } else {
            [IO.File]::Move($Paths.Source, $Paths.Output)
        }
    } catch {
        throw "Failed to publish '$($Paths.Output)'. The previous EXE is retained; no alternate distribution is created. $($_.Exception.Message)"
    }
    # Exact known legacy file only; never recursively remove release directories.
    if (Test-Path -LiteralPath $Paths.LegacyOutput) { Remove-Item -LiteralPath $Paths.LegacyOutput -ErrorAction Stop }
    if (Test-Path -LiteralPath $Paths.LegacyDirectory) { Remove-Item -LiteralPath $Paths.LegacyDirectory -ErrorAction Stop }
    Write-Output $Paths.Output
}
