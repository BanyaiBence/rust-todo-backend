mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;

#[tokio::test]
async fn health_returns_ok() {
    let (status, body) = common::send(common::app(), Method::GET, "/health", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "status": "ok" }));
}
