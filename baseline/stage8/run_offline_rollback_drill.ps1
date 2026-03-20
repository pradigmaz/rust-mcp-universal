$ErrorActionPreference = "Stop"

$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$cli = Join-Path $root "target\debug\rmu-cli.exe"
if (!(Test-Path $cli)) {
  throw "CLI binary not found: $cli"
}

$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$proj = Join-Path $env:TEMP ("rmu_offline_drill_" + $stamp)
New-Item -ItemType Directory -Path (Join-Path $proj "src") -Force | Out-Null
function Write-Utf8NoBom {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Content
  )
  $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($Path, $Content, $utf8NoBom)
}

function Invoke-RmuJson {
  param(
    [Parameter(Mandatory = $true)][string[]]$Args
  )
  $output = (& $cli @Args 2>&1 | Out-String).Trim()
  $exitCode = $LASTEXITCODE
  $json = $null
  try {
    $json = $output | ConvertFrom-Json
  }
  catch {
    $json = $null
  }
  return [pscustomobject]@{
    exit_code = $exitCode
    raw = $output
    json = $json
  }
}

Write-Utf8NoBom -Path (Join-Path $proj "src\lib.rs") -Content "pub fn offline_probe_symbol() -> i32 { 42 }"

$datasetPath = Join-Path $proj "dataset.json"
$datasetJson = @"
{
  "queries": [
    {
      "query": "offline_probe_symbol",
      "qrels": [
        {"path": "src/lib.rs", "relevance": 1.0}
      ]
    }
  ]
}
"@
Set-Content -Path $datasetPath -Value $datasetJson -Encoding UTF8

$baselinePath = Join-Path $proj "baseline.json"
$baselineJson = @"
{
  "recall_at_k": 1.0,
  "mrr_at_k": 1.0,
  "ndcg_at_k": 1.0,
  "avg_estimated_tokens": 1.0,
  "latency_p50_ms": 0.1,
  "latency_p95_ms": 0.1
}
"@
Set-Content -Path $baselinePath -Value $baselineJson -Encoding UTF8

$thresholdsPath = Join-Path $proj "thresholds_strict.json"
$thresholdsJson = @"
{
  "min": {
    "recall_at_k": 1.1
  },
  "max": {
    "latency_p95_ms": 0.0,
    "avg_estimated_tokens": 0.0
  }
}
"@
Set-Content -Path $thresholdsPath -Value $thresholdsJson -Encoding UTF8
Write-Utf8NoBom -Path $datasetPath -Content $datasetJson
Write-Utf8NoBom -Path $baselinePath -Content $baselineJson
Write-Utf8NoBom -Path $thresholdsPath -Content $thresholdsJson

# 1) Offline-first drill
$indexCall = Invoke-RmuJson -Args @("--project-path", $proj, "--json", "index", "--reindex")
$statusOffCall = Invoke-RmuJson -Args @("--project-path", $proj, "--migration-mode", "off", "--json", "status")
$searchLexCall = Invoke-RmuJson -Args @("--project-path", $proj, "--migration-mode", "off", "--rollout-phase", "shadow", "--json", "search", "--query", "offline_probe_symbol", "--limit", "5")
$searchSemanticCall = Invoke-RmuJson -Args @("--project-path", $proj, "--migration-mode", "off", "--json", "search", "--query", "offline_probe_symbol", "--limit", "5", "--semantic", "--semantic-fail-mode", "fail_open")

# 2) Fast rollback drill via benchmark compare payload
$benchCall = Invoke-RmuJson -Args @("--project-path", $proj, "--json", "query-benchmark", "--dataset", $datasetPath, "--baseline", $baselinePath, "--thresholds", $thresholdsPath, "--runs", "2", "--auto-index")

# 3) Full rollback drill: backup -> delete-index -> restore -> verify
$dbPath = [string]$statusOffCall.json.db_path
if (-not (Test-Path $dbPath)) {
  throw "DB not found for rollback drill: $dbPath"
}
$backupDir = Join-Path $proj "rollback_backup"
New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
Copy-Item $dbPath (Join-Path $backupDir "index.db") -Force
if (Test-Path ($dbPath + "-wal")) {
  Copy-Item ($dbPath + "-wal") (Join-Path $backupDir "index.db-wal") -Force
}
if (Test-Path ($dbPath + "-shm")) {
  Copy-Item ($dbPath + "-shm") (Join-Path $backupDir "index.db-shm") -Force
}

$deleteCall = Invoke-RmuJson -Args @("--project-path", $proj, "--json", "delete-index", "--yes")
$statusAfterDeleteCall = Invoke-RmuJson -Args @("--project-path", $proj, "--migration-mode", "off", "--json", "status")

New-Item -ItemType Directory -Path (Split-Path $dbPath -Parent) -Force | Out-Null
Copy-Item (Join-Path $backupDir "index.db") $dbPath -Force
if (Test-Path (Join-Path $backupDir "index.db-wal")) {
  Copy-Item (Join-Path $backupDir "index.db-wal") ($dbPath + "-wal") -Force
}
if (Test-Path (Join-Path $backupDir "index.db-shm")) {
  Copy-Item (Join-Path $backupDir "index.db-shm") ($dbPath + "-shm") -Force
}
$statusAfterRestoreCall = Invoke-RmuJson -Args @("--project-path", $proj, "--migration-mode", "off", "--json", "status")

$result = [ordered]@{
  timestamp_utc = (Get-Date).ToUniversalTime().ToString("o")
  project_path = $proj
  offline_drill = [ordered]@{
    index_ok = ($indexCall.exit_code -eq 0 -and $indexCall.json.indexed -ge 1)
    status_migration_off_ok = ($statusOffCall.exit_code -eq 0 -and (([string]$statusOffCall.json.db_path).Length -gt 0))
    lexical_search_ok = ($searchLexCall.exit_code -eq 0 -and $null -ne $searchLexCall.json.hits)
    lexical_hits = @($searchLexCall.json.hits).Count
    semantic_search_ok = ($searchSemanticCall.exit_code -eq 0 -and $null -ne $searchSemanticCall.json.hits)
    semantic_search_hits = @($searchSemanticCall.json.hits).Count
  }
  fast_rollback_drill = [ordered]@{
    benchmark_ok = ($benchCall.exit_code -eq 0 -and ([string]$benchCall.json.mode) -eq "baseline_vs_candidate")
    rollback_level = [string]$benchCall.json.rollback.level
    rollback_reasons = @($benchCall.json.rollback.reasons)
    rollback_fast_actions = @($benchCall.json.rollback.fast_actions)
  }
  full_rollback_drill = [ordered]@{
    backup_created = (Test-Path (Join-Path $backupDir "index.db"))
    delete_index_ok = ($deleteCall.exit_code -eq 0 -and $deleteCall.json.removed_count -ge 1)
    migration_off_after_delete_failed_as_expected = ($statusAfterDeleteCall.exit_code -ne 0)
    restore_status_ok = ($statusAfterRestoreCall.exit_code -eq 0 -and (([string]$statusAfterRestoreCall.json.db_path).Length -gt 0))
    restore_files = $statusAfterRestoreCall.json.files
    restore_semantic_vectors = $statusAfterRestoreCall.json.semantic_vectors
  }
}

$artifact = Join-Path $PSScriptRoot ("ROLLBACK_OFFLINE_DRILL_" + $stamp + ".json")
$result | ConvertTo-Json -Depth 8 | Set-Content -Path $artifact -Encoding UTF8

Write-Output $artifact
