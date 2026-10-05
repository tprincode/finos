# Prove the desktop painted Home. A process and an HTTP 200 are not enough.
# Exit 0 only when exactly one finos-desktop.exe is running and `page loaded Home`
# is newer than that process.
$ErrorActionPreference = "Stop"
$dir = Join-Path $env:LOCALAPPDATA "com.finos.desktop"
$procs = @(Get-Process -Name "finos-desktop" -ErrorAction SilentlyContinue)
Write-Output ("finos-desktop count: {0}" -f $procs.Count)
foreach ($p in $procs) {
  Write-Output ("pid {0} started {1:o}" -f $p.Id, $p.StartTime.ToUniversalTime())
}
$launchers = @(Get-CimInstance Win32_Process -Filter "Name = 'cmd.exe'" |
  Where-Object { $_.CommandLine -match "start-finos-dev\.bat" })
Write-Output ("start-finos-dev.bat count: {0}" -f $launchers.Count)
foreach ($p in $launchers) {
  Write-Output ("launcher pid {0}" -f $p.ProcessId)
}
if ($procs.Count -ne 1 -or $launchers.Count -ne 1) { exit 1 }

$started = $procs[0].StartTime.ToUniversalTime().AddSeconds(-15)
$lines = @()
foreach ($name in @("dev-console.log", "page-loaded.log")) {
  $path = Join-Path $dir $name
  if (Test-Path $path) {
    $lines += @(Get-Content -Path $path -ErrorAction SilentlyContinue)
  }
}
$bad = @($lines | Where-Object { $_ -match "panicked|Failed to setup|npm error" })
if ($bad.Count -gt 0) {
  Write-Output "log failure:"
  $bad | Select-Object -Last 5 | ForEach-Object { Write-Output $_ }
  exit 1
}
$failedPaint = @($lines | Where-Object { $_ -match "^page loaded Home failed " })
if ($failedPaint.Count -gt 0) {
  Write-Output "Home did not paint:"
  Write-Output $failedPaint[-1]
  exit 1
}
$hits = @($lines | Where-Object { $_ -match "^page loaded Home 20" })
if ($hits.Count -eq 0) {
  Write-Output "no page loaded Home line"
  exit 1
}
$newest = $null
foreach ($h in $hits) {
  if ($h -match "page loaded Home (\S+)") {
    $stamp = [datetime]::Parse($Matches[1]).ToUniversalTime()
    if (-not $newest -or $stamp -gt $newest) { $newest = $stamp }
  }
}
if (-not $newest -or $newest -lt $started) {
  Write-Output "page loaded Home is older than this process"
  exit 1
}
try {
  $r = Invoke-WebRequest -Uri "http://localhost:1420" -UseBasicParsing -TimeoutSec 5
  Write-Output ("http {0}" -f [int]$r.StatusCode)
} catch {
  Write-Output "localhost:1420 did not answer"
  exit 1
}
Write-Output ("page loaded Home {0:o}" -f $newest)
exit 0
