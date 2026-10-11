# Mac dev bootstrap (Apple Silicon)

Locked household paths for the MacBook Air cutover:

| Role | Path |
|------|------|
| Repo / Cursor workspace | `~/Developer/finos` |
| App data + `local.sqlite` | `~/Library/Application Support/com.finos.desktop` |
| Downloads / Financial | `~/Documents/Financial` |
| Transfer kit (Windows) | `E:\Finos\` |

Never put `local.sqlite` in iCloud Drive, Dropbox, or OneDrive.

## Installer (preferred)

After the repo is on the Mac (clone, USB kit, or copy):

```bash
cd ~/Developer/finos   # or wherever the clone landed
chmod +x install-mac.sh scripts/mac-install.sh
./install-mac.sh
```

What it does (idempotent; safe to re-run):

1. Confirms macOS / warns if not arm64
2. Ensures Xcode Command Line Tools (opens Apple’s installer if missing, then exits for a re-run)
3. Installs Homebrew when absent, then Node, `gh`, and Rust via `rustup` when missing
4. Creates `~/Documents/Financial` and Application Support app-data dir
5. Symlinks `~/Developer/finos` → this clone when the locked path is free
6. `chmod +x` on `finos.sh` / `app-up.sh` / `golden.sh`
7. `npm install` + `cargo check`

Flags:

| Flag | Effect |
|------|--------|
| `--seed` | `npm run data-seed` into Application Support |
| `--launch` | start `./finos.sh` when finished |
| `--skip-brew` | toolchain check only (no Homebrew installs) |
| `--repo PATH` | use that clone instead of the script’s parent |

Then:

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
2. [Tauri 2 macOS prerequisites](https://v2.tauri.app/start/prerequisites/)
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

## Soft dual-run

~1 week: Mac primary for daily FinOS + Cursor; Windows standby with a dated SQLite freeze. After that, Windows is emergency / NSIS builds only until a signed Mac bundle exists.

## Related

- [m1-macos-gate.md](m1-macos-gate.md) — full handoff matrix (still parked until recorded on Mac)
- Installer: `install-mac.sh` → `scripts/mac-install.sh`
- Kit files under `E:\Finos\kit\` when the transfer pack is built
