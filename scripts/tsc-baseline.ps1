<#
    Fail when the desktop TypeScript error count rises.

    Vite does not typecheck, so a new error ships until this script is run. The allowed count
    lives in docs/architecture/typescript-baseline.json. Pass -Record only after a slice has
    fixed errors and the new, lower count should become the bar.
#>
param([switch]$Record)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$baselinePath = Join-Path $repo "docs\architecture\typescript-baseline.json"
Set-Location $repo

$lines = npx tsc --noEmit -p apps/desktop/tsconfig.json 2>&1 | Where-Object { $_ -match 'error TS' }
$count = @($lines).Count
$baseline = (Get-Content $baselinePath -Raw | ConvertFrom-Json).count

Write-Host "typescript errors: $count (baseline $baseline)"

if ($Record) {
    if ($count -gt $baseline) {
        Write-Host "refusing to record a higher count"
        $lines | Select-Object -First 20 | ForEach-Object { Write-Host $_ }
        exit 1
    }
    $json = (@{ note = "Count of error TS lines from npx tsc --noEmit -p apps/desktop/tsconfig.json. scripts/tsc-baseline.ps1 fails when the count rises. A slice that fixes errors updates this number in the same change."; count = $count } | ConvertTo-Json) + "`r`n"
    [System.IO.File]::WriteAllText($baselinePath, $json, (New-Object System.Text.UTF8Encoding $false))
    Write-Host "recorded $count"
    exit 0
}

if ($count -gt $baseline) {
    Write-Host "typescript errors rose above the baseline"
    $lines | ForEach-Object { Write-Host $_ }
    exit 1
}
exit 0
