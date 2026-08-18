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

Live `AiAnalyze` needs an xAI Grok key. Copy [`.env.example`](../../.env.example) to `.env` (gitignored) or `%LOCALAPPDATA%\finos\.env` and set `XAI_API_KEY`. Do not commit the key. Without it, `AiAnalyze` returns `missing_api_key`.

## Layout

- `src/financeClient.ts` — `LocalTauriFinanceClient` (Tauri IPC)
- `src-tauri` — host crate depending on `crates/application-core` (not a root workspace member)
- Shared types: `@finos/app-contracts`
