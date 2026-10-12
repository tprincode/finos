# Mac dev bootstrap (Apple Silicon)

Locked household paths for the MacBook Air cutover:

| Role | Path |
|------|------|
| Repo / Cursor workspace | `~/Developer/finos` |
| App data + `local.sqlite` | `~/Library/Application Support/com.finos.desktop` |
| Downloads / Financial | `~/Documents/Financial` |
| Transfer kit (Windows) | `E:\Finos\` |

Never put `local.sqlite` in iCloud Drive, Dropbox, or OneDrive.

## Once vs every day

| When | What you run | How hard |
|------|----------------|----------|
| **First time** (or after wiping tools) | USB: double-click `GO.command` (or `./install-mac.sh`) | Slow: Xcode CLT may force a **second** run; Homebrew may ask for password; rustup + `npm install` + `cargo check` |
| **Every coding day** | Double-click `finos.command` (or `./finos.sh`) then `./scripts/app-up.sh` | One launch + health check — **do not** re-run the installer |
| **Tools look broken** | `./install-mac.sh --check` | Fast Ready/doctor report; only re-run full install if it says NOT READY |

First-boot gaps (rustup keg PATH, missing `cargo` in a new Terminal) are handled by `scripts/mac-env.sh` (sourced by install/launch/goldens) and by writing brew/cargo blocks once into `~/.zprofile`. You should not need manual `export PATH=…` after a successful install.

## USB Start here (preferred — almost no git)

1. Plug in the FinOS USB stick.
2. In Finder, open the volume → **Finos** folder.
3. **Double-click `GO.command`** (opens Terminal; Mac counterpart to Windows `finos.bat` / typed bash).

That copies the kit into `~/Developer/finos` and runs the installer. If the Terminal asks for permission the first time: right-click → Open.

Optional Terminal fallback (replace the drive name if Finder shows another):

```bash
bash /Volumes/<VolumeName>/Finos/load-from-usb.sh
# optional: --seed  or  --launch
```

Rebuild the stick from Windows after commit: `.\scripts\pack-usb.ps1` (writes `E:\Finos`).

## Installer (preferred)

```bash
cd ~/Developer/finos
chmod +x install-mac.sh scripts/mac-install.sh
./install-mac.sh
# later: ./install-mac.sh --check   # Ready report (dirs + toolchain)
```

What it does (idempotent; safe to re-run):

1. Confirms macOS / warns if not arm64
2. Ensures Xcode Command Line Tools (opens Apple’s installer if missing, then exits for a re-run — expected once)
3. Installs Homebrew when absent, then Node, `gh`, and Rust via `rustup` when missing (fails loud if brew packages fail)
4. Creates `~/Documents/Financial` and Application Support app-data dir
5. Symlinks `~/Developer/finos` → this clone when the locked path is free
6. `chmod +x` on `finos.sh` / `app-up.sh` / `golden.sh` / `mac-env.sh`
7. Persists brew + cargo PATH into `~/.zprofile` once (`# finos-mac-brew` / `# finos-mac-cargo`)
8. `npm install` + `cargo check`

Flags:

| Flag | Effect |
|------|--------|
| `--seed` | `npm run data-seed` into Application Support |
| `--launch` | start `./finos.sh` when finished |
| `--skip-brew` | toolchain check only (no Homebrew installs) |
| `--check` | Ready/doctor report only (dirs + CLT + toolchain; no npm/cargo work) |
| `--repo PATH` | use that clone instead of the script’s parent |

Then (daily): in Finder open `~/Developer/finos` and **double-click `finos.command`**, or:

```bash
./finos.sh
# other terminal after Home paints:
./scripts/app-up.sh
```

## Manual first run (if you skip the installer)

```bash
mkdir -p ~/Developer
# from E:\Finos kit or: git clone <origin> ~/Developer/finos
cd ~/Developer/finos
chmod +x finos.sh scripts/app-up.sh scripts/golden.sh
mkdir -p ~/Documents/Financial
npm install
cargo check
./finos.sh
```

Prerequisites if installing by hand:

1. Xcode Command Line Tools: `xcode-select --install`
2. [Tauri 2 macOS prerequisites](https://v2.tauri.app/start/prerequisites/) (desktop needs CLT; full Xcode only for iOS)
3. Rust via rustup (`stable` — see repo `rust-toolchain.toml`)
4. Node.js LTS + npm
5. GitHub CLI for HTTPS push: `brew install gh` then `gh auth login` (GitHub.com, HTTPS, browser)

Seed (same app-data root the host opens — host sets `FINOS_APP_DATA` on launch; for seed-before-first-launch):

```bash
export FINOS_APP_DATA="$HOME/Library/Application Support/com.finos.desktop"
npm run data-seed
```

Or: `./install-mac.sh --seed`

## Cursor on Mac

1. Install Cursor (Apple Silicon).
2. **File → Open Folder** → `~/Developer/finos`.
3. Confirm `.cursor/rules` load (including Mac restart companion).
4. `gh auth login` (HTTPS) so `git push` works on `origin`.
5. Agent restart: write restart token under Application Support, stop `finos-desktop`, run `./finos.sh`, then `./scripts/app-up.sh`. Do **not** use `finos.bat` / `app-up.ps1` on Mac.

Do not copy Windows AgentStores wholesale.

## Goldens (coding)

```bash
./scripts/golden.sh ui_modules
./scripts/golden.sh cart
```

Or Tools → Component Registry → **Run golden** (coding checkout with cargo only; household installs stay disabled with a reason).

## Soft dual-run

~1 week: Mac primary for daily FinOS + Cursor; Windows standby with a dated SQLite freeze. After that, Windows is emergency / NSIS builds only until a signed Mac bundle exists.

## Related

- [m1-macos-gate.md](m1-macos-gate.md) — full handoff matrix (still parked until recorded on Mac)
- Installer: `install-mac.sh` → `scripts/mac-install.sh` (PATH helper: `scripts/mac-env.sh`)
- Kit files under `E:\Finos\kit\` when the transfer pack is built
