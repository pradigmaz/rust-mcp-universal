param()

$ErrorActionPreference = "Stop"

$scriptPath = [System.IO.Path]::GetFullPath($MyInvocation.MyCommand.Path)
$scriptDir = [System.IO.Path]::GetDirectoryName($scriptPath)
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $scriptDir ".."))
$triggerPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target\runtime\supervisor\reload.trigger"))

Push-Location $repoRoot
try {
    & cargo build --release -p rmu-mcp-server --bin rmu-mcp-server
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
finally {
    Pop-Location
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $triggerPath) | Out-Null
Set-Content -LiteralPath $triggerPath -Value ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()) -Encoding ASCII
Write-Host "reloaded=true"
Write-Host "trigger=$triggerPath"
