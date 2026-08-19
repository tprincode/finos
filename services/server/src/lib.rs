//! Axum loopback adapter. Clients submit command/query envelopes, not SQL (ADR-0011).
//! `/v1/commands` and `/v1/queries` require Bearer JWT (M9). Desktop Profile A stays on LocalTauri / SQLite.

mod auth;

pub use auth::{mint_test_token, AuthConfig, Identity, TEST_AUDIENCE, TEST_ISSUER};

use application_core::contracts::{CommandRequest, CommandResult, QueryRequest, QueryResult};
use application_core::queries::{execute_command_on, execute_query_on};
use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use serde::de::DeserializeOwned;
use storage_postgres::PostgresPlatform;
use tower::ServiceExt;

#[derive(Clone)]
pub struct AppState {
    pub platform: PostgresPlatform,
    pub auth: AuthConfig,
}

pub fn router(platform: PostgresPlatform, auth: AuthConfig) -> Router {
    Router::new()
        .route("/v1/commands", post(commands))
        .route("/v1/queries", post(queries))
        .with_state(AppState { platform, auth })
}

pub fn test_router(platform: PostgresPlatform) -> Router {
    router(platform, AuthConfig::test_issuer())
}

fn identity_from_headers(headers: &HeaderMap, auth: &AuthConfig) -> Result<Identity, StatusCode> {
    let value = headers
        .get(header::AUTHORIZATION)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let raw = value.to_str().map_err(|_| StatusCode::UNAUTHORIZED)?;
    let token = raw
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;
    auth.authenticate(token).map_err(|_| StatusCode::UNAUTHORIZED)
}

async fn commands(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CommandRequest>,
) -> Result<Json<CommandResult>, StatusCode> {
    let identity = identity_from_headers(&headers, &state.auth)?;
    let result = execute_command_on(&state.platform, &state.platform, request.clone()).await;
    state
        .platform
        .record_command_audit(
            &identity.user_sub,
            &identity.device_id,
            request.correlation_id,
            &request.command_name,
            result.ok,
            result.error_code.as_deref(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(result))
}

async fn queries(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResult>, StatusCode> {
    let _identity = identity_from_headers(&headers, &state.auth)?;
    Ok(Json(
        execute_query_on(&state.platform, &state.platform, request).await,
    ))
}

/// Issue one HTTP POST against the in-process router (no bind).
pub async fn http_command(
    app: Router,
    request: CommandRequest,
    bearer: &str,
) -> Result<CommandResult, String> {
    http_json(app, "/v1/commands", &request, Some(bearer)).await
}

pub async fn http_query(
    app: Router,
    request: QueryRequest,
    bearer: &str,
) -> Result<QueryResult, String> {
    http_json(app, "/v1/queries", &request, Some(bearer)).await
}

pub async fn http_post_status<T: serde::Serialize>(
    app: Router,
    uri: &str,
    body: &T,
    bearer: Option<&str>,
) -> Result<StatusCode, String> {
    let response = dispatch(app, uri, body, bearer).await?;
    Ok(response.status())
}

async fn http_json<T: serde::Serialize, R: DeserializeOwned>(
    app: Router,
    uri: &str,
    body: &T,
    bearer: Option<&str>,
) -> Result<R, String> {
    let response = dispatch(app, uri, body, bearer).await?;
    if response.status() != StatusCode::OK {
        return Err(format!("http {}", response.status()));
    }
    let collected = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_slice(&collected).map_err(|e| e.to_string())
}

async fn dispatch<T: serde::Serialize>(
    app: Router,
    uri: &str,
    body: &T,
    bearer: Option<&str>,
) -> Result<axum::http::Response<Body>, String> {
    let bytes = serde_json::to_vec(body).map_err(|e| e.to_string())?;
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = builder.body(Body::from(bytes)).map_err(|e| e.to_string())?;
    app.oneshot(request).await.map_err(|e| e.to_string())
}
