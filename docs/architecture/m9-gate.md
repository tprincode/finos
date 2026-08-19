# M9 gate — OIDC, web client, central backup

V1.1 §12 remaining after M8: authenticated clients against one audited authoritative server. Desktop Profile A **keeps SQLite** (ADR-0003). No MAGI oracle edits. No authority cutover until the owner names it.

Docker Compose (`postgres:16` on `localhost:5432`) is how tests run PostgreSQL. Pass: container `healthy` and `pg_isready -U finos -d finos`.

Goldens use the in-process **test issuer** (`http://finos.test/oidc`, HS256). Live Auth0/Okta is not required. Production env: `OIDC_ISSUER`, `OIDC_AUDIENCE`, `OIDC_TEST_HS256_SECRET` (JWKS later).

## Commands

```
docker compose up -d --wait
$env:DATABASE_URL="postgres://finos:finos@localhost:5432/finos"
cargo test -p golden-harness --test oidc
cargo test -p golden-harness --test remote_http
cargo test -p golden-harness --test remote_concurrency
cargo test -p golden-harness --test web
cargo test -p golden-harness --test postgres_backup
cargo test -p golden-harness --features magi-gate
cargo test --workspace
```

HTTP, OIDC, backup, and Postgres tests **fail** if `DATABASE_URL` is missing. They do not fall back to SQLite. MAGI oracles are never written.

## Slice B1 must pass

| Filter | What it proves |
|--------|----------------|
| `unauthenticated_commands_and_queries_fail_closed` | Missing/invalid Bearer → HTTP 401 |
| `two_authenticated_clients_and_concurrency_conflict` | Two JWT identities; concurrent `AccountUpdate` still `concurrency_conflict`; `command_audit` records user, device, correlation, result |

## Slice B2 must pass

| Filter | What it proves |
|--------|----------------|
| `web_client_compiles_and_contains_no_sql` | `apps/web` types compile; UI has no SQL; uses `RemoteHttpFinanceClient` |

## Slice B3 must pass

| Filter | What it proves |
|--------|----------------|
| `postgres_logical_dump_restore_keeps_magi_and_contract_cents` | Logical dump → throwaway DB; DividendGet 50000 and G-MAGI-01 SAFE / 4000000 / 2960000 still match |

## Out of this gate

M1 macOS restore/handoff (needs a Mac). SQLite→Postgres authority cutover (owner-gated: do not switch `App.tsx` to `RemoteHttpFinanceClient`). Live Auth0/Okta, TLS reverse proxy, PITR/S3.
