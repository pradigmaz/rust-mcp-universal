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

& $binaryPath @args
exit $LASTEXITCODE
