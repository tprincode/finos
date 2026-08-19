# M8 gate — PostgreSQL adapter, MAGI pack, Axum, concurrency, snapshot import

V1.1 §12 full M8 exit is core-on-server + remote client + concurrency. Profile A desktop **keeps SQLite** (ADR-0003). No OIDC, no MAGI oracle edits, no authority cutover.

Docker Compose (`postgres:16` on `localhost:5432`) is how tests *run* PostgreSQL. Pass: container `healthy` and `pg_isready -U finos -d finos`.

## Commands

```
docker compose up -d --wait
$env:DATABASE_URL="postgres://finos:finos@localhost:5432/finos"
cargo test -p golden-harness -- postgres_contract
cargo test -p golden-harness -- postgres_magi
cargo test -p golden-harness --test remote_http
cargo test -p golden-harness --test remote_concurrency
cargo test -p golden-harness --test postgres_snapshot_import
cargo test -p golden-harness --features magi-gate
cargo test --workspace
```

HTTP and Postgres tests **fail** if `DATABASE_URL` is missing. They do not fall back to SQLite. MAGI oracles are never written.

## Slice 1 must pass

| Filter | What it proves |
|--------|----------------|
| `postgres_contract_sqlite_and_postgres_match_cents` | Same cents on SQLite and PostgreSQL (50000 / 140000 / 120000) |

## Slice 2 must pass

| Filter | What it proves |
|--------|----------------|
| `postgres_magi_pack_matches_approved_oracles` | G-MAGI-01–10 on PostgreSQL match the same owner-approved oracles |
| `cargo test -p golden-harness --features magi-gate` | MAGI pack still passes on SQLite (desktop path) |

## Slice 3 must pass

| Filter | What it proves |
|--------|----------------|
| `remote_http_contract_cents_and_g01_magi` | Axum `POST /v1/commands` and `/v1/queries` return contract cents and G-MAGI-01 SAFE / 4000000 / 2960000. `App.tsx` stays `LocalTauriFinanceClient`. |

## Slice 4 must pass

| Filter | What it proves |
|--------|----------------|
| `concurrent_account_update_returns_concurrency_conflict` | Two `AccountUpdate`s with the same `expectedVersion`; one succeeds, the other is `concurrency_conflict` |

## Slice 5 must pass

| Filter | What it proves |
|--------|----------------|
| `sqlite_snapshot_imports_into_postgres_with_reconciled_cents` | `SnapshotImport` copies a local SQLite snapshot into PostgreSQL; counts, money, lots, MAGI match (AC-ARCH-08) |

## Out of these slices

OIDC / web (`apps/web`), TLS reverse proxy, AC-ARCH-10 user/device on every command, SQLite→Postgres authority cutover (desktop remains the SQLite writer).
