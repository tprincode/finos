# Postgres / multi-device gate (locked with mobile priority)

Local SQLite WAL remains the operational SOT ([ADR-0003](../adr/0003-sqlite-postgres-authority.md)). Cloud folders transport **immutable** publish heads and outbox intents ([ADR-0007](../adr/0007-snapshot-device-handoff.md)) — never the live `.sqlite` file.

## Stay parked until an owner names one of

1. **Concurrent multi-writer** — PC and Mac both write in the same week without handoff discipline.
2. **Always-on phone writes** — Week Ahead confirm (or more) must apply when no desktop is running.
3. **Measured SQLite hotspot** remains after open-path / App.tsx extract / fleet defer work **and** dual-adapter contract tests cover the needed surface.

## Do not unlock Postgres for

- Desktop feel / speed alone (see [desktop-latency-2026-09-30.md](desktop-latency-2026-09-30.md)).
- “Having a cloud database.”
- Apple/Google Drive as a live DB sync target.

## Mobile path without cutover

- `MobilePublish` → `head.json` under `FINOS_MOBILE_PUBLISH_DIR` (or app-data `mobile-publish/`).
- Phone reads via `MobileHeadGet` or head.json URL.
- `MobileOutboxPut` / `MobileOutboxDrain` for Week Ahead confirm intents applied only on the primary desktop.
