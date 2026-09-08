# Web client (M9)

React app on `RemoteHttpFinanceClient` (same command/query envelopes as desktop). Screens must not embed SQL.

Thin first screen: `HealthGet`, `DividendGet`, `MagiProjectionGet` against Axum + OIDC Bearer JWT.

```
docker compose up -d --wait
$env:DATABASE_URL="postgres://finos:finos@localhost:5432/finos"
cargo run -p finos-server
npm install
npm run dev --workspace=@finos/web
```

Paste a test-issuer Bearer token into the UI. Desktop Profile A stays on `LocalTauriFinanceClient` / SQLite until an owner-named cutover.
