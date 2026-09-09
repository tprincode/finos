# Desktop application (Tauri 2 + React)

Milestone 1 slice 1: Windows shell plus `HealthGet` through `LocalTauriFinanceClient`.
No SQLite, snapshots, or SQL in the UI.

## Run

Two launch modes share the same live SQLite. Do not run both at once.

**Coding days (this month’s default).** Console is expected. Desktop shortcut `finos.bat` calls [`start-finos-dev.bat`](start-finos-dev.bat), which is `npm run desktop` (`tauri dev`). Fully quit, then run the bat again after a Rust host change.

```
npm run desktop
```

Same command from `apps/desktop`: `npm start`.

**Use-the-app days (no console).** Desktop shortcut `finos-installed.bat` calls [`start-finos-installed.bat`](start-finos-installed.bat) and starts `%LOCALAPPDATA%\finos\finos-desktop.exe` if that frozen cut is installed. It does not compile. Rebuild the icon only when you want it to catch up — not after every host change.

**Exit**

- **File → Exit** in the window menu
- **Exit** button on the Platform section
- Close the window (title-bar X)
- In the terminal that started the dev app: `Ctrl+C`

The window should show `HealthGet` `ok`, status `ok`, contract `1.0.0-draft`, and DividendGet / IncomePlanGet / DashboardGet / TrendsGet actual totals (same number after a posted dividend).

**Build signed Windows installer** (long compile; uses `CurrentUser\My` thumbprint `CEAAACA78BB136405A1496C2DD39EB0E8975D6E6`). Run this only for a frozen household cut, not the daily coding loop:

```
npm run desktop:build
```

Then run `src-tauri/target/release/bundle/nsis/finos_0.1.0_x64-setup.exe` and pin Start Menu / Desktop to `%LOCALAPPDATA%\finos\finos-desktop.exe`. Self-signed `CN=finos`. On this PC the cert is in CurrentUser Root + TrustedPublisher; `Get-AuthenticodeSignature` is `Valid`. Public CA and auto-update stay parked.

Updater uses a separate minisign keypair. Only the public key is in `src-tauri/tauri.conf.json`. The private key is in gitignored `src-tauri/updater-keys/` and must never be committed. Check for updates reports status only; a missing GitHub release fails closed and does not post ledger facts.

## Layout

- `src/financeClient.ts` — `LocalTauriFinanceClient` (Tauri IPC)
- `src-tauri` — host crate depending on `crates/application-core` (not a root workspace member)
- Shared types: `@finos/app-contracts`
