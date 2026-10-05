<#
    Run golden suites and say up front how long each one takes.

    Going quiet through a multi-minute cargo run reads as a hang, and the owner cannot see tool
    calls. This prints the expected duration before the suite starts, from the last measured run
    in docs/architecture/golden-durations.json, then prints the actual elapsed time and records
    it so the next estimate is better.

    Usage:  .\scripts\golden.ps1 cart ui_modules
#>
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Suites
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$recordPath = Join-Path $repo "docs\architecture\golden-durations.json"

if (-not $Suites -or $Suites.Count -eq 0) {
    Write-Host "usage: .\scripts\golden.ps1 <suite> [suite...]"
    Write-Host "suites are the --test names, for example: cart accessibility ui_modules"
    exit 2
}

$record = Get-Content $recordPath -Raw | ConvertFrom-Json

function Format-Duration([int]$seconds) {
    if ($seconds -lt 60) { return "$seconds seconds" }
    $minutes = [math]::Round($seconds / 60.0, 1)
    if ($minutes -eq 1) { return "1 minute" }
    return "$minutes minutes"
}

$failed = @()
$totalStart = Get-Date

foreach ($suite in $Suites) {
    $known = $null
    if ($record.suites.PSObject.Properties.Name -contains $suite) {
        $known = $record.suites.$suite
    }

    if ($null -eq $known) {
        Write-Host "running the $suite golden, duration not measured on this machine yet"
    }
    else {
        Write-Host ("running a {0} golden: {1}" -f (Format-Duration $known), $suite)
    }

    $start = Get-Date
    & cargo test -p golden-harness --test $suite
    $code = $LASTEXITCODE
    $elapsed = [int][math]::Round(((Get-Date) - $start).TotalSeconds)

    if ($null -eq $known) {
        Write-Host ("the $suite golden took {0}" -f (Format-Duration $elapsed))
    }
    else {
        Write-Host ("the $suite golden took {0}, expected {1}" -f (Format-Duration $elapsed), (Format-Duration $known))
    }
    if ($code -ne 0) {
        Write-Host "$suite FAILED (exit $code)"
        $failed += $suite
    }

    # Record even a failing run: the owner still waited that long.
    if ($record.suites.PSObject.Properties.Name -contains $suite) {
        $record.suites.$suite = $elapsed
    }
    else {
        $record.suites | Add-Member -NotePropertyName $suite -NotePropertyValue $elapsed
    }
    # WriteAllText with a BOM-less encoding on purpose: Set-Content -Encoding UTF8 prepends a
    # byte order mark on PowerShell 5.1 and serde_json rejects it as "expected value" at 1:1.
    $json = ($record | ConvertTo-Json -Depth 5) + "`r`n"
    [System.IO.File]::WriteAllText($recordPath, $json, (New-Object System.Text.UTF8Encoding $false))
}

if ($Suites.Count -gt 1) {
    $total = [int][math]::Round(((Get-Date) - $totalStart).TotalSeconds)
    Write-Host ("{0} goldens took {1} in total" -f $Suites.Count, (Format-Duration $total))
}

if ($failed.Count -gt 0) {
    Write-Host ("failed: {0}" -f ($failed -join ", "))
    exit 1
}
exit 0
