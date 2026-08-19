# Central application server (M9 OIDC)

Rust Axum adapter over `application-core` and `storage-postgres`. Clients submit versioned command/query envelopes, not SQL (ADR-0011). `POST /v1/commands` and `POST /v1/queries` require a Bearer JWT.

Loopback only (`127.0.0.1:8787`). TLS reverse proxy can wait for a named deploy. Desktop Profile A stays on `LocalTauriFinanceClient` / SQLite.

Goldens use the in-process test issuer (`http://finos.test/oidc`). Production:

```
OIDC_ISSUER=http://finos.test/oidc
OIDC_AUDIENCE=finos-api
OIDC_TEST_HS256_SECRET=...
```

All three must be set together, or all omitted to use the test issuer. JWKS from a live IdP is not required for this slice.

```
docker compose up -d --wait
$env:DATABASE_URL="postgres://finos:finos@localhost:5432/finos"
cargo run -p finos-server
```
