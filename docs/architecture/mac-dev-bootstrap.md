# Mac dev bootstrap (Apple Silicon)

Locked household paths for the MacBook Air cutover:

| Role | Path |
|------|------|
| Repo / Cursor workspace | `~/Developer/finos` |
| App data + `local.sqlite` | `~/Library/Application Support/com.finos.desktop` |
| Downloads / Financial | `~/Documents/Financial` |
| Transfer kit (Windows) | `E:\Finos\` |

Never put `local.sqlite` in iCloud Drive, Dropbox, or OneDrive.

## Prerequisites

1. Xcode Command Line Tools: `xcode-select --install`
2. [Tauri 2 macOS prerequisites](https://v2.tauri.app/start/prerequisites/)
3. Rust via rustup (`stable` — see repo `rust-toolchain.toml`)
4. Node.js LTS + npm
5. GitHub CLI for HTTPS push: `brew install gh` then `gh auth login` (GitHub.com, HTTPS, browser)

## First run

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

In another terminal after Home paints:

```bash
./scripts/app-up.sh
```

Seed (same app-data root the host opens — host sets `FINOS_APP_DATA` on launch; for seed-before-first-launch):

```bash
export FINOS_APP_DATA="$HOME/Library/Application Support/com.finos.desktop"
npm run data-seed
```

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
- Kit files under `E:\Finos\kit\` when the transfer pack is built
