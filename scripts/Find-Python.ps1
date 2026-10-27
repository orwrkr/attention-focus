function Get-DocsPython {
    if ($env:ATTENTION_FOCUS_PYTHON) {
        if (-not (Test-Path -LiteralPath $env:ATTENTION_FOCUS_PYTHON -PathType Leaf)) { throw 'ATTENTION_FOCUS_PYTHON does not name a Python executable.' }
        return $env:ATTENTION_FOCUS_PYTHON
    }
    $local = Join-Path (Split-Path -Parent $PSScriptRoot) 'target/docs-venv/Scripts/python.exe'
    if (Test-Path -LiteralPath $local -PathType Leaf) { return $local }
    $python = Get-Command python -ErrorAction SilentlyContinue
    if (-not $python) { throw 'Python 3.11+ is required to export the guide. Set ATTENTION_FOCUS_PYTHON or add Python to PATH.' }
    return $python.Source
}
