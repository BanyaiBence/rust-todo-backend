mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test]
async fn health_returns_ok(pool: PgPool) {
    let (status, body) = common::send(&common::app(pool), Method::GET, "/health", None, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "status": "ok" }));
}
