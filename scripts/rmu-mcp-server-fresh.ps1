param()

$ErrorActionPreference = "Stop"

$scriptPath = [System.IO.Path]::GetFullPath($MyInvocation.MyCommand.Path)
$scriptDir = [System.IO.Path]::GetDirectoryName($scriptPath)
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $scriptDir ".."))
$releaseBinaryPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target\\release\\rmu-mcp-server.exe"))
$debugBinaryPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target\\debug\\rmu-mcp-server.exe"))
$runtimeReleaseBinaryPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target\\runtime\\release\\rmu-mcp-server.exe"))
$runtimeDebugBinaryPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target\\runtime\\debug\\rmu-mcp-server.exe"))
$targetRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target"))

function Get-LatestSourceWriteTimeUtc {
    param(
        [string]$RepositoryRoot
    )

    $paths = @(
        (Join-Path $RepositoryRoot "Cargo.toml"),
        (Join-Path $RepositoryRoot "Cargo.lock"),
        (Join-Path $RepositoryRoot "crates\\core"),
        (Join-Path $RepositoryRoot "crates\\mcp-server")
    )

    $latest = [datetime]::MinValue
    foreach ($path in $paths) {
        if (-not (Test-Path -LiteralPath $path)) {
            continue
        }
        $item = Get-Item -LiteralPath $path
        if ($item.PSIsContainer) {
            $candidate = Get-ChildItem -LiteralPath $path -Recurse -File |
                Sort-Object LastWriteTimeUtc -Descending |
                Select-Object -First 1
            if ($candidate -and $candidate.LastWriteTimeUtc -gt $latest) {
                $latest = $candidate.LastWriteTimeUtc
            }
        }
        elseif ($item.LastWriteTimeUtc -gt $latest) {
            $latest = $item.LastWriteTimeUtc
        }
    }

    return $latest
}

function Test-RebuildRequired {
    param(
        [string]$BinaryPath,
        [string]$RepositoryRoot
    )

    if (-not (Test-Path -LiteralPath $BinaryPath)) {
        return $true
    }

    $binaryWriteTimeUtc = (Get-Item -LiteralPath $BinaryPath).LastWriteTimeUtc
    $latestSourceWriteTimeUtc = Get-LatestSourceWriteTimeUtc -RepositoryRoot $RepositoryRoot
    return $latestSourceWriteTimeUtc -gt $binaryWriteTimeUtc
}

function Stop-CheckoutServerProcesses {
    param(
        [string]$TargetRoot
    )

    $staleServers = Get-CimInstance Win32_Process -Filter "Name = 'rmu-mcp-server.exe'" |
        Where-Object {
            $_.ExecutablePath -and
            ([System.IO.Path]::GetFullPath($_.ExecutablePath)).StartsWith(
                "$TargetRoot\",
                [System.StringComparison]::OrdinalIgnoreCase
            )
        }

    foreach ($server in $staleServers) {
        Stop-Process -Id $server.ProcessId -Force -ErrorAction SilentlyContinue
    }

    $deadline = (Get-Date).AddSeconds(5)
    do {
        $remaining = Get-CimInstance Win32_Process -Filter "Name = 'rmu-mcp-server.exe'" |
            Where-Object {
                $_.ExecutablePath -and
                ([System.IO.Path]::GetFullPath($_.ExecutablePath)).StartsWith(
                    "$TargetRoot\",
                    [System.StringComparison]::OrdinalIgnoreCase
                )
            }
        if (-not $remaining) {
            return
        }
        Start-Sleep -Milliseconds 150
    } while ((Get-Date) -lt $deadline)

    $pids = ($remaining | Select-Object -ExpandProperty ProcessId) -join ","
    throw "stale rmu-mcp-server.exe processes are still running under $TargetRoot (pids: $pids). Use a fresh launcher retry after they exit."
}

function Invoke-BuildIfNeeded {
    param(
        [string]$BinaryPath,
        [string]$RepositoryRoot,
        [string[]]$CargoArgs
    )

    if (-not (Test-RebuildRequired -BinaryPath $BinaryPath -RepositoryRoot $RepositoryRoot)) {
        return $true
    }

    Push-Location $RepositoryRoot
    try {
        & cargo @CargoArgs
        if ($LASTEXITCODE -ne 0) {
            return $false
        }
    }
    finally {
        Pop-Location
    }

    if (-not (Test-Path -LiteralPath $BinaryPath)) {
        return $false
    }
    return $true
}

function Publish-RuntimeBinary {
    param(
        [string]$SourceBinaryPath,
        [string]$RuntimeBinaryPath
    )

    $runtimeDirectory = Split-Path -Parent $RuntimeBinaryPath
    New-Item -ItemType Directory -Force -Path $runtimeDirectory | Out-Null
    Copy-Item -LiteralPath $SourceBinaryPath -Destination $RuntimeBinaryPath -Force
}

$runBinaryPath = $null

Stop-CheckoutServerProcesses -TargetRoot $targetRoot
if (
    (Invoke-BuildIfNeeded -BinaryPath $releaseBinaryPath -RepositoryRoot $repoRoot -CargoArgs @("build", "--release", "-p", "rmu-mcp-server")) -and
    (Test-Path -LiteralPath $releaseBinaryPath)
) {
    Publish-RuntimeBinary -SourceBinaryPath $releaseBinaryPath -RuntimeBinaryPath $runtimeReleaseBinaryPath
    $runBinaryPath = $runtimeReleaseBinaryPath
}
else {
    if (
        (Invoke-BuildIfNeeded -BinaryPath $debugBinaryPath -RepositoryRoot $repoRoot -CargoArgs @("build", "-p", "rmu-mcp-server")) -and
        (Test-Path -LiteralPath $debugBinaryPath)
    ) {
        Publish-RuntimeBinary -SourceBinaryPath $debugBinaryPath -RuntimeBinaryPath $runtimeDebugBinaryPath
        $runBinaryPath = $runtimeDebugBinaryPath
    }
}

if (-not $runBinaryPath) {
    throw "failed to prepare fresh rmu-mcp-server from both release and debug profiles"
}

& $runBinaryPath @args
exit $LASTEXITCODE
