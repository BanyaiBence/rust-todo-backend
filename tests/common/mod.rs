use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use todo_backend::{
    config::{Config, LogFormat},
    state::AppState,
};
use tower::ServiceExt;

pub fn test_config() -> Config {
    Config {
        database_url: String::new(), // unused: tests receive a pool from #[sqlx::test]
        jwt_secret: "test-secret".to_string(),
        jwt_ttl_seconds: 3600,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        log_format: LogFormat::Pretty,
    }
}

pub fn app(pool: PgPool) -> Router {
    todo_backend::build_app(AppState {
        pool,
        config: Arc::new(test_config()),
    })
}

/// Sends a request through the router in-process and returns the status and JSON body
/// (`Value::Null` when the body is empty).
pub async fn send(
    app: Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(json) => {
            builder = builder.header("content-type", "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };

    let response = app.oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, json)
}
