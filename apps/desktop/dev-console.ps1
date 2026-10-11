$repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $repo
$dir = Join-Path $env:LOCALAPPDATA "com.finos.desktop"
if (-not (Test-Path $dir)) {
  New-Item -ItemType Directory -Path $dir -Force | Out-Null
}
$log = Join-Path $dir "dev-console.log"
$prev = Join-Path $dir "dev-console.prev.log"
# Keep one prior session so post-restart greps can see the failure that killed the old console.
if (Test-Path $log) {
  Copy-Item -Path $log -Destination $prev -Force
  Remove-Item -Path $log -Force
}
$stamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffffffZ")
@(
  "=== launch $stamp ==="
  "repo=$repo"
) | Set-Content -Path $log -Encoding utf8
npm run desktop *>&1 | Tee-Object -FilePath $log -Append
exit $LASTEXITCODE
