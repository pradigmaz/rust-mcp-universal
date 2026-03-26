param()

$ErrorActionPreference = "Stop"

$scriptPath = [System.IO.Path]::GetFullPath($MyInvocation.MyCommand.Path)
$scriptDir = [System.IO.Path]::GetDirectoryName($scriptPath)
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $scriptDir ".."))
$codexBinDir = Join-Path $env:USERPROFILE ".codex\\bin"
$installedBinary = Join-Path $codexBinDir "rmu-mcp-server.exe"
$backupBinary = Join-Path $codexBinDir "rmu-mcp-server.previous.exe"

function Get-RunningBinaryProcesses {
    param(
        [string]$BinaryPath
    )

    @(Get-CimInstance Win32_Process -Filter "Name = 'rmu-mcp-server.exe'" |
        Where-Object {
            $_.ExecutablePath -and
            [System.StringComparer]::OrdinalIgnoreCase.Equals(
                [System.IO.Path]::GetFullPath($_.ExecutablePath),
                $BinaryPath
            )
        })
}

function Stop-SourceBinaryProcesses {
    param(
        [string]$BinaryPath
    )

    $running = Get-RunningBinaryProcesses -BinaryPath $BinaryPath

    foreach ($server in $running) {
        Stop-Process -Id $server.ProcessId -Force -ErrorAction SilentlyContinue
    }
}

function Invoke-BuildProfile {
    param(
        [string]$Profile
    )

    $cargoArgs = @("build", "-p", "rmu-mcp-server")
    if ($Profile -eq "release") {
        $cargoArgs = @("build", "--release", "-p", "rmu-mcp-server")
    }

    & cargo @cargoArgs
    return ($LASTEXITCODE -eq 0)
}

$sourceBinary = $null
$installedProfile = $null

foreach ($candidate in @(
    @{ profile = "release"; path = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target\\release\\rmu-mcp-server.exe")) },
    @{ profile = "debug"; path = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target\\debug\\rmu-mcp-server.exe")) }
)) {
    Stop-SourceBinaryProcesses -BinaryPath $candidate.path

    Push-Location $repoRoot
    try {
        if (-not (Invoke-BuildProfile -Profile $candidate.profile)) {
            continue
        }
    }
    finally {
        Pop-Location
    }

    if (Test-Path -LiteralPath $candidate.path) {
        $sourceBinary = $candidate.path
        $installedProfile = $candidate.profile
        break
    }
}

if (-not $sourceBinary) {
    throw "failed to build rmu-mcp-server for both release and debug profiles"
}

if (-not (Test-Path -LiteralPath $sourceBinary)) {
    throw "release binary not found at $sourceBinary"
}

New-Item -ItemType Directory -Force -Path $codexBinDir | Out-Null
$runningInstalled = @(Get-RunningBinaryProcesses -BinaryPath $installedBinary)
if ($runningInstalled.Count -gt 0) {
    $pids = ($runningInstalled | Select-Object -ExpandProperty ProcessId) -join ","
    if (Test-Path -LiteralPath $installedBinary) {
        Copy-Item -LiteralPath $installedBinary -Destination $backupBinary -Force
    }
    Write-Warning "active Codex RMU server is running (pids: $pids); installed binary was not replaced"
    Write-Host "pending_restart=true"
    Write-Host "restart_hint=restart the Codex app, then rerun this installer; opening a new chat is not enough because MCP transport is app-global"
    Write-Host "installed_profile=$installedProfile"
    Write-Host "installed_binary=$installedBinary"
    Write-Host "backup_binary=$backupBinary"
    Write-Host "repo_root=$repoRoot"
    Write-Host "running_installed_pids=$pids"
    exit 0
}

if (Test-Path -LiteralPath $installedBinary) {
    Copy-Item -LiteralPath $installedBinary -Destination $backupBinary -Force
}
Copy-Item -LiteralPath $sourceBinary -Destination $installedBinary -Force
Write-Host "pending_restart=false"
Write-Host "installed_profile=$installedProfile"
Write-Host "installed_binary=$installedBinary"
Write-Host "backup_binary=$backupBinary"
Write-Host "repo_root=$repoRoot"
