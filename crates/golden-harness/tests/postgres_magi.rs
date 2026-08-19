//! MAGI pack on PostgreSQL: same owner-approved oracles as SQLite (AC-ARCH-17).
//! Missing DATABASE_URL must fail — never fall back to SQLite. Never writes oracles.

use golden_harness::{compare_magi_pack_postgres, repo_root};

#[tokio::test]
async fn postgres_magi_pack_matches_approved_oracles() {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "DATABASE_URL is required for postgres_magi; do not substitute SQLite. \
             Start docker compose (postgres:16) and set \
             DATABASE_URL=postgres://finos:finos@localhost:5432/finos"
        )
    });
    assert!(
        url.starts_with("postgres://") || url.starts_with("postgresql://"),
        "DATABASE_URL must be PostgreSQL, not SQLite: {url}"
    );
    compare_magi_pack_postgres(&repo_root(), &url)
        .await
        .expect("MAGI pack on PostgreSQL must match owner-approved oracles");
}
