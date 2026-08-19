use finos_server::{router, AuthConfig};
use storage_postgres::PostgresPlatform;

#[tokio::main]
async fn main() {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "DATABASE_URL is required (postgres://…). Do not substitute SQLite. \
             Example: postgres://finos:finos@127.0.0.1:5432/finos"
        )
    });
    if !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
        panic!("DATABASE_URL must be PostgreSQL, not SQLite: {url}");
    }
    let auth = AuthConfig::from_env_or_test();
    let platform = PostgresPlatform::connect(&url)
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL connect failed (do not use SQLite): {e}"));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8787")
        .await
        .expect("bind 127.0.0.1:8787");
    axum::serve(listener, router(platform, auth))
        .await
        .expect("axum serve");
}
