# Post-restart health: process/launcher/Home gates plus current-console + UI error greps.
# Exit 0 only when app-up.ps1 passes and the current session console has no hard failures.
# Prior console (dev-console.prev.log) is shown for diagnosis but does not fail the gate.
$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$appUp = Join-Path $here "app-up.ps1"
$dir = Join-Path $env:LOCALAPPDATA "com.finos.desktop"
$console = Join-Path $dir "dev-console.log"
$prev = Join-Path $dir "dev-console.prev.log"
$uiErr = Join-Path $dir "ui-errors.log"

Write-Output "=== finos dev-health ==="
Write-Output ("logs: {0}" -f $dir)

& $appUp
$appUpCode = $LASTEXITCODE
if ($appUpCode -ne 0) {
  Write-Output "app-up: FAIL (exit $appUpCode)"
} else {
  Write-Output "app-up: OK"
}

$failPattern = "panicked|Failed to setup|npm error|error\[E|beforeDevCommand failed|Lifecycle script ``dev`` failed"
$current = @()
if (Test-Path $console) {
  $current = @(Get-Content -Path $console -ErrorAction SilentlyContinue)
}
$bad = @($current | Where-Object { $_ -match $failPattern })

if ($bad.Count -gt 0) {
  Write-Output "current console failure matches:"
  $bad | Select-Object -Last 12 | ForEach-Object { Write-Output $_ }
}

if (Test-Path $prev) {
  $prevBad = @(Get-Content -Path $prev -ErrorAction SilentlyContinue | Where-Object { $_ -match $failPattern })
  if ($prevBad.Count -gt 0) {
    Write-Output "prior console (informational, not a gate):"
    $prevBad | Select-Object -Last 6 | ForEach-Object { Write-Output $_ }
  }
}

if (Test-Path $uiErr) {
  $tail = @(Get-Content -Path $uiErr -ErrorAction SilentlyContinue | Select-Object -Last 5)
  if ($tail.Count -gt 0) {
    Write-Output "ui-errors.log (last lines):"
    $tail | ForEach-Object { Write-Output $_ }
  }
} else {
  Write-Output "ui-errors.log: (none yet)"
}

$ok = ($appUpCode -eq 0) -and ($bad.Count -eq 0)
if ($ok) {
  Write-Output "dev-health: OK"
  exit 0
}
Write-Output "dev-health: FAIL"
exit 1
