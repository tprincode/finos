# Rebuild E:\Finos Mac USB kit from the current committed tip.
# Requires a clean git worktree (tracked files) unless -Force.
#
# Usage:
#   .\scripts\pack-usb.ps1
#   .\scripts\pack-usb.ps1 -KitRoot E:\Finos
#   .\scripts\pack-usb.ps1 -Force   # pack even if working tree is dirty
param(
  [string]$KitRoot = "E:\Finos",
  [switch]$Force
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

if (-not (Test-Path $KitRoot)) {
  throw "Kit root missing: $KitRoot (plug in the USB stick)"
}

$dirty = @(git status --porcelain --untracked-files=no)
if ($dirty.Count -gt 0 -and -not $Force) {
  Write-Host "Working tree has tracked changes. Commit first, or pass -Force." -ForegroundColor Red
  $dirty | Select-Object -First 20 | ForEach-Object { Write-Host "  $_" }
  exit 1
}

$head = (git rev-parse HEAD).Trim()
$short = (git rev-parse --short HEAD).Trim()
$branch = (git rev-parse --abbrev-ref HEAD).Trim()
$built = Get-Date -Format "yyyy-MM-ddTHH:mm:ssK"

Write-Host "==> Packing USB kit"
Write-Host "    HEAD   $head"
Write-Host "    Branch $branch"
Write-Host "    Kit    $KitRoot"

$bundle = Join-Path $KitRoot "finos.bundle"
if (Test-Path $bundle) { Remove-Item $bundle -Force }
git bundle create $bundle $branch
if (-not (Test-Path $bundle)) { throw "git bundle create failed" }

$tmp = Join-Path $env:TEMP "finos-usb-$short"
if (Test-Path $tmp) { Remove-Item $tmp -Recurse -Force }
git clone --branch $branch --single-branch $bundle $tmp

$repoDir = Join-Path $KitRoot "repo"
if (Test-Path $repoDir) { Remove-Item $repoDir -Recurse -Force }
Move-Item $tmp $repoDir

Copy-Item (Join-Path $Root "scripts\load-from-usb.sh") (Join-Path $KitRoot "load-from-usb.sh") -Force
Copy-Item (Join-Path $Root "GO.command") (Join-Path $KitRoot "GO.command") -Force

if (-not (Test-Path (Join-Path $repoDir "finos.command"))) {
  throw "finos.command missing from bundled repo — commit it before packing"
}
if (-not (Test-Path (Join-Path $repoDir "scripts\mac-env.sh"))) {
  throw "scripts/mac-env.sh missing from bundled repo — commit Mac harden before packing"
}

$kitDocs = Join-Path $KitRoot "kit"
New-Item -ItemType Directory -Force -Path $kitDocs | Out-Null

$startHere = @"
FinOS Mac USB kit
=================
HEAD: $head
Branch: $branch
Built: $built

START HERE (Mac)
----------------
1. Plug in this USB stick.
2. In Finder open the volume → Finos folder.
3. Double-click GO.command

That one click copies FinOS into ~/Developer/finos and runs the installer.
You do not need to type git or bash commands.

If macOS blocks the first open: right-click GO.command → Open.

Optional Terminal fallback (change VolumeName if needed):
  bash /Volumes/VolumeName/Finos/load-from-usb.sh
  bash /Volumes/VolumeName/Finos/load-from-usb.sh --seed
  bash /Volumes/VolumeName/Finos/load-from-usb.sh --launch

After install (daily)
---------------------
  Finder → ~/Developer/finos → double-click finos.command
  # other Terminal after Home paints:
  ./scripts/app-up.sh

More detail: kit/HOW-TO-LOAD.md and kit/REQUIREMENTS.md
This stick does NOT include your live household database.
"@
Set-Content -Path (Join-Path $KitRoot "START-HERE.txt") -Value $startHere -Encoding utf8

@'
# How to load FinOS from this USB (Mac)

## One click (preferred)

1. Plug in the stick.
2. Finder → Finos folder.
3. Double-click **GO.command**.

That copies FinOS to ~/Developer/finos and runs the installer. Almost no git.

## Terminal fallback

```bash
bash /Volumes/VolumeName/Finos/load-from-usb.sh
```

Optional flags: `--seed` or `--launch`.

## After it finishes (every day)

Finder → `~/Developer/finos` → double-click **finos.command**

Or:

```bash
cd ~/Developer/finos
./finos.sh
# other Terminal after Home paints:
./scripts/app-up.sh
```

## Check without reinstalling

```bash
cd ~/Developer/finos
./install-mac.sh --check
```

## Rebuild this stick (Windows)

```powershell
.\scripts\pack-usb.ps1
```

## What this stick is not

- Not your live household SQLite
- Not a full Windows AgentStores copy
'@ | Set-Content -Path (Join-Path $kitDocs "HOW-TO-LOAD.md") -Encoding utf8

$exclude = @"
Excluded from this Mac coding kit:
- Live household local.sqlite / AgentStores
- node_modules / target build artifacts
- Windows NSIS installer outputs
Rebuild with: .\scripts\pack-usb.ps1
"@
Set-Content -Path (Join-Path $KitRoot "exclude-list.txt") -Value $exclude -Encoding utf8

$badSqlite = @(Get-ChildItem -Path $KitRoot -Recurse -Filter "local.sqlite" -ErrorAction SilentlyContinue |
  Where-Object { $_.FullName -notmatch '[\\/]repo[\\/]' })
if ($badSqlite.Count -gt 0) {
  throw "Refusing to leave local.sqlite on kit: $($badSqlite.FullName -join ', ')"
}

$macEnvOk = Test-Path (Join-Path $repoDir "scripts\mac-env.sh")
$goOk = Test-Path (Join-Path $KitRoot "GO.command")
$finosCmdOk = Test-Path (Join-Path $repoDir "finos.command")
$loadOk = Test-Path (Join-Path $KitRoot "load-from-usb.sh")

$verify = @"
VERIFY_OK
HEAD=$head
BRANCH=$branch
BUILT=$built
bundle=ok
repo=ok
GO.command=$goOk
finos.command=$finosCmdOk
mac-env.sh=$macEnvOk
load-from-usb.sh=$loadOk
local.sqlite=absent
"@
Set-Content -Path (Join-Path $KitRoot "VERIFY_OK.txt") -Value $verify -Encoding utf8

Write-Host ""
Write-Host "VERIFY_OK"
Write-Host "  HEAD          $short ($branch)"
Write-Host "  GO.command    $goOk"
Write-Host "  finos.command $finosCmdOk"
Write-Host "  mac-env.sh    $macEnvOk"
Write-Host "  load-from-usb $loadOk"
Write-Host "Mac: Finder → Finos → double-click GO.command"
exit 0
