param(
    [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"

$scriptDir = [System.IO.Path]::GetDirectoryName([System.IO.Path]::GetFullPath($MyInvocation.MyCommand.Path))
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $scriptDir ".."))
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot "baseline\investigation\stage9\runtime_report.json"
}

$cliPath = Join-Path $repoRoot "target\debug\rmu-cli.exe"
$serverPath = Join-Path $repoRoot "target\debug\rmu-mcp-server.exe"

if (-not (Test-Path -LiteralPath $cliPath) -or -not (Test-Path -LiteralPath $serverPath)) {
    Push-Location $repoRoot
    try {
        cargo build --locked -p rmu-cli -p rmu-mcp-server | Out-Null
    } finally {
        Pop-Location
    }
}

function Invoke-CliJson {
    param(
        [string[]]$Arguments
    )

    $raw = & $cliPath @Arguments 2>$null
    if ($LASTEXITCODE -ne 0) {
        throw "rmu-cli failed: $($Arguments -join ' ')"
    }
    return $raw | ConvertFrom-Json
}

function New-TempProject {
    param(
        [string]$Name
    )

    $path = Join-Path ([System.IO.Path]::GetTempPath()) ("rmu-runtime-kpi-" + $Name + "-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $path | Out-Null
    return $path
}

function Start-StaleServerProcess {
    param(
        [string]$ProjectPath
    )

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $serverPath
    $psi.Arguments = "--project-path `"$ProjectPath`""
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    return [System.Diagnostics.Process]::Start($psi)
}

$caseResults = @()
$compatPassed = 0
$compatTotal = 0
$tempRoots = @()
$staleProcess = $null

try {
    $emptyProject = New-TempProject "empty"
    $tempRoots += $emptyProject
    $emptyPayload = Invoke-CliJson @("--project-path", $emptyProject, "--json", "preflight")
    $emptyPass = $emptyPayload.status -eq "ok" -and @($emptyPayload.errors).Count -eq 0
    $compatTotal += 1
    if ($emptyPass) { $compatPassed += 1 }
    $caseResults += [ordered]@{
        id = "startup-empty-project"
        category = "compat"
        pass = $emptyPass
        status = $emptyPayload.status
        error_count = @($emptyPayload.errors).Count
    }

    $initializedProject = New-TempProject "initialized"
    $tempRoots += $initializedProject
    New-Item -ItemType Directory -Path (Join-Path $initializedProject "src") | Out-Null
    Set-Content -Path (Join-Path $initializedProject "src\lib.rs") -Value "pub fn runtime_kpi_fixture() {}`n" -NoNewline
    & $cliPath "--project-path" $initializedProject "status" | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "failed to initialize DB for runtime KPI report"
    }
    $initializedPayload = Invoke-CliJson @("--project-path", $initializedProject, "--json", "preflight")
    $initializedPass = $initializedPayload.status -eq "ok" -and @($initializedPayload.errors).Count -eq 0
    $compatTotal += 1
    if ($initializedPass) { $compatPassed += 1 }
    $caseResults += [ordered]@{
        id = "startup-initialized-project"
        category = "compat"
        pass = $initializedPass
        status = $initializedPayload.status
        error_count = @($initializedPayload.errors).Count
    }

    $staleProject = New-TempProject "stale"
    $tempRoots += $staleProject
    $staleProcess = Start-StaleServerProcess -ProjectPath $staleProject
    Start-Sleep -Milliseconds 750
    $stalePayload = Invoke-CliJson @("--project-path", $staleProject, "--json", "preflight")
    $sameBinaryPids = @($stalePayload.same_binary_other_pids)
    $probePath = $stalePayload.stale_process_probe_binary_path
    $stalePass = $stalePayload.status -eq "warning" `
        -and $stalePayload.stale_process_suspected `
        -and $sameBinaryPids.Count -gt 0 `
        -and -not [string]::IsNullOrWhiteSpace($probePath) `
        -and [System.StringComparer]::OrdinalIgnoreCase.Equals(
            [System.IO.Path]::GetFileName($probePath),
            "rmu-mcp-server.exe"
        )
    $caseResults += [ordered]@{
        id = "windows-stale-server-detection"
        category = "stale_detection"
        pass = $stalePass
        status = $stalePayload.status
        stale_process_suspected = [bool]$stalePayload.stale_process_suspected
        same_binary_other_pid_count = $sameBinaryPids.Count
        stale_process_probe_binary_path = $probePath
    }

    $staleRuntimePayload = Invoke-CliJson @("--project-path", $staleProject, "--json", "preflight")
    $originalProcessStartedAt = $env:RMU_TEST_PROCESS_STARTED_AT_MS
    $originalBinaryModifiedAt = $env:RMU_TEST_BINARY_MODIFIED_AT_MS
    try {
        $env:RMU_TEST_PROCESS_STARTED_AT_MS = "1000"
        $env:RMU_TEST_BINARY_MODIFIED_AT_MS = "4001"
        $staleRuntimePayload = Invoke-CliJson @("--project-path", $staleProject, "--json", "preflight")
    } finally {
        if ($null -ne $originalProcessStartedAt) {
            $env:RMU_TEST_PROCESS_STARTED_AT_MS = $originalProcessStartedAt
        } else {
            Remove-Item Env:RMU_TEST_PROCESS_STARTED_AT_MS -ErrorAction SilentlyContinue
        }
        if ($null -ne $originalBinaryModifiedAt) {
            $env:RMU_TEST_BINARY_MODIFIED_AT_MS = $originalBinaryModifiedAt
        } else {
            Remove-Item Env:RMU_TEST_BINARY_MODIFIED_AT_MS -ErrorAction SilentlyContinue
        }
    }
    $staleRuntimePass = $staleRuntimePayload.status -eq "incompatible" `
        -and $staleRuntimePayload.running_binary_stale `
        -and @($staleRuntimePayload.errors).Count -gt 0
    $caseResults += [ordered]@{
        id = "windows-stale-running-binary"
        category = "runtime_guard"
        pass = $staleRuntimePass
        status = $staleRuntimePayload.status
        running_binary_stale = [bool]$staleRuntimePayload.running_binary_stale
        running_binary_version = $staleRuntimePayload.running_binary_version
    }

    $report = [ordered]@{
        generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
        platform = "windows"
        cli_binary_path = $cliPath
        server_binary_path = $serverPath
        startup_compat_success_rate = if ($compatTotal -eq 0) { 0.0 } else { [math]::Round($compatPassed / $compatTotal, 4) }
        stale_server_detection = if ($stalePass) { "deterministic" } else { "not_deterministic" }
        stale_runtime_guard = if ($staleRuntimePass) { "deterministic" } else { "not_deterministic" }
        supported_case_count = $compatTotal
        supported_passed_case_count = $compatPassed
        cases = $caseResults
    }

    $outputDir = [System.IO.Path]::GetDirectoryName([System.IO.Path]::GetFullPath($OutputPath))
    if (-not (Test-Path -LiteralPath $outputDir)) {
        New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    }
    $report | ConvertTo-Json -Depth 6 | Set-Content -Path $OutputPath
} finally {
    if ($null -ne $staleProcess) {
        try {
            if (-not $staleProcess.HasExited) {
                $staleProcess.Kill()
                $staleProcess.WaitForExit()
            }
        } catch {
        }
        $staleProcess.Dispose()
    }

    foreach ($path in $tempRoots) {
        if (Test-Path -LiteralPath $path) {
            Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
