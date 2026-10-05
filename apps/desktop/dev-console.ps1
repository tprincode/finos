$repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $repo
$log = Join-Path $env:LOCALAPPDATA "com.finos.desktop\dev-console.log"
npm run desktop *>&1 | Tee-Object -FilePath $log
exit $LASTEXITCODE
