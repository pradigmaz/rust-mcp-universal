param()

$ErrorActionPreference = "Stop"

$scriptPath = [System.IO.Path]::GetFullPath($MyInvocation.MyCommand.Path)
$scriptDir = [System.IO.Path]::GetDirectoryName($scriptPath)
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $scriptDir ".."))
$binaryPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target\\release\\rmu-mcp-server.exe"))

if (-not (Test-Path -LiteralPath $binaryPath)) {
    Write-Error "rmu-mcp-server.exe not found at $binaryPath. Build it first with `cargo build --release -p rmu-mcp-server`."
    exit 1
}

$staleServers = Get-CimInstance Win32_Process -Filter "Name = 'rmu-mcp-server.exe'" |
    Where-Object {
        $_.ExecutablePath -and
        [System.StringComparer]::OrdinalIgnoreCase.Equals(
            [System.IO.Path]::GetFullPath($_.ExecutablePath),
            $binaryPath
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
                $binaryPath
            )
        }
    if (-not $remaining) {
        break
    }
    Start-Sleep -Milliseconds 150
} while ((Get-Date) -lt $deadline)

if ($remaining) {
    $pids = ($remaining | Select-Object -ExpandProperty ProcessId) -join ","
    Write-Error "stale rmu-mcp-server.exe processes are still running for $binaryPath (pids: $pids). Use a fresh launcher retry after they exit."
    exit 1
}

& $binaryPath @args
exit $LASTEXITCODE
