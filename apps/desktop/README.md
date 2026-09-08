# Desktop application (Tauri 2 + React)

Milestone 1 slice 1: Windows shell plus `HealthGet` through `LocalTauriFinanceClient`.
No SQLite, snapshots, or SQL in the UI.

## Run

**Start** (from the repository root, after `npm install`):

```
npm run desktop
```

Same command from `apps/desktop`: `npm start` (runs `tauri dev`).

**Exit**

- **File → Exit** in the window menu
- **Exit** button on the Platform section
- Close the window (title-bar X)
- In the terminal that started it: `Ctrl+C`

The window should show `HealthGet` `ok`, status `ok`, contract `1.0.0-draft`, and DividendGet / IncomePlanGet / DashboardGet / TrendsGet actual totals (same number after a posted dividend).

**Build signed Windows installer** (long compile; uses `CurrentUser\My` thumbprint `CEAAACA78BB136405A1496C2DD39EB0E8975D6E6`):

```
npm run desktop:build
```

Output (gitignored `target/`): `src-tauri/target/release/bundle/nsis/finos_0.1.0_x64-setup.exe`. Self-signed `CN=finos`. On this PC the cert is in CurrentUser Root + TrustedPublisher; `Get-AuthenticodeSignature` is `Valid`. Installed to `%LOCALAPPDATA%\finos`.

Updater uses a separate minisign keypair. Only the public key is in `src-tauri/tauri.conf.json`. The private key is in gitignored `src-tauri/updater-keys/` and must never be committed. Check for updates reports status only; a missing GitHub release fails closed and does not post ledger facts.

## Layout

- `src/financeClient.ts` — `LocalTauriFinanceClient` (Tauri IPC)
- `src-tauri` — host crate depending on `crates/application-core` (not a root workspace member)
- Shared types: `@finos/app-contracts`
