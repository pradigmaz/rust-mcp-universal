param()

$ErrorActionPreference = "Stop"

$scriptPath = [System.IO.Path]::GetFullPath($MyInvocation.MyCommand.Path)
$scriptDir = [System.IO.Path]::GetDirectoryName($scriptPath)
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $scriptDir ".."))
$supervisorPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target\release\rmu-mcp-supervisor.exe"))

Push-Location $repoRoot
try {
    & cargo build --release -p rmu-mcp-server --bins 1>$null
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $supervisorPath)) {
    throw "rmu-mcp-supervisor.exe was not built at $supervisorPath"
}

$env:RMU_REPO_ROOT = $repoRoot
& $supervisorPath @args
exit $LASTEXITCODE
