param()

$ErrorActionPreference = "Stop"

$scriptPath = [System.IO.Path]::GetFullPath($MyInvocation.MyCommand.Path)
$scriptDir = [System.IO.Path]::GetDirectoryName($scriptPath)
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $scriptDir ".."))
$releaseBinaryPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target\\release\\rmu-mcp-server.exe"))
$debugBinaryPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target\\debug\\rmu-mcp-server.exe"))

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

function Stop-StaleServerProcesses {
    param(
        [string]$BinaryPath
    )

    $staleServers = Get-CimInstance Win32_Process -Filter "Name = 'rmu-mcp-server.exe'" |
        Where-Object {
            $_.ExecutablePath -and
            [System.StringComparer]::OrdinalIgnoreCase.Equals(
                [System.IO.Path]::GetFullPath($_.ExecutablePath),
                $BinaryPath
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
                [System.StringComparer]::OrdinalIgnoreCase.Equals(
                    [System.IO.Path]::GetFullPath($_.ExecutablePath),
                    $BinaryPath
                )
            }
        if (-not $remaining) {
            return
        }
        Start-Sleep -Milliseconds 150
    } while ((Get-Date) -lt $deadline)

    $pids = ($remaining | Select-Object -ExpandProperty ProcessId) -join ","
    throw "stale rmu-mcp-server.exe processes are still running for $BinaryPath (pids: $pids). Use a fresh launcher retry after they exit."
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

$runBinaryPath = $null

Stop-StaleServerProcesses -BinaryPath $releaseBinaryPath
if (
    (Invoke-BuildIfNeeded -BinaryPath $releaseBinaryPath -RepositoryRoot $repoRoot -CargoArgs @("build", "--release", "-p", "rmu-mcp-server")) -and
    (Test-Path -LiteralPath $releaseBinaryPath)
) {
    $runBinaryPath = $releaseBinaryPath
}
else {
    Stop-StaleServerProcesses -BinaryPath $debugBinaryPath
    if (
        (Invoke-BuildIfNeeded -BinaryPath $debugBinaryPath -RepositoryRoot $repoRoot -CargoArgs @("build", "-p", "rmu-mcp-server")) -and
        (Test-Path -LiteralPath $debugBinaryPath)
    ) {
        $runBinaryPath = $debugBinaryPath
    }
}

if (-not $runBinaryPath) {
    throw "failed to prepare fresh rmu-mcp-server from both release and debug profiles"
}

& $runBinaryPath @args
exit $LASTEXITCODE
