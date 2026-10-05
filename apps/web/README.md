# Web / phone client

React app on `RemoteHttpFinanceClient` (same command/query envelopes as desktop). No SQL in the UI.

## Mobile v0 (read head)

Primary desktop **File → Force publish mobile head** writes `head.json` under:

- `FINOS_MOBILE_PUBLISH_DIR` if set (point this at a Google Drive / iCloud **sync folder** used only as transport), else
- `%LOCALAPPDATA%\com.finos.desktop\mobile-publish\`

Phone UI loads that head via:

1. **Load from server** → `MobileHeadGet` (same folder on the machine running Axum / desktop publish), or
2. **Load from URL** → paste a URL to `head.json` if the sync folder is web-reachable.

Shows Week Ahead, open tasks, and MAGI cliff summary. **Confirm** queues `MobileOutboxPut` (`week_ahead_confirm`). Primary desktop **File → Apply mobile outbox** runs `MobileOutboxDrain`, then re-publishes.

Live SQLite never sits on cloud drive lock. Postgres cutover stays parked until concurrent multi-writer is named.

## M9 proof (unchanged path)

```
docker compose up -d --wait
$env:DATABASE_URL="postgres://finos:finos@localhost:5432/finos"
cargo run -p finos-server
npm install
npm run dev --workspace=@finos/web
```

Desktop Profile A stays on `LocalTauriFinanceClient` / SQLite until an owner-named cutover.
