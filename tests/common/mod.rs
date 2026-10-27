//! Helpers shared by the test binaries; each binary uses a different subset.
#![allow(dead_code)]

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

/// The only origin `test_config` allows for CORS.
pub const ALLOWED_ORIGIN: &str = "https://app.example.com";

pub fn test_config() -> Config {
    Config {
        database_url: String::new(), // unused: tests receive a pool from #[sqlx::test]
        jwt_secret: "test-secret".to_string(),
        jwt_ttl_seconds: 3600,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        log_format: LogFormat::Pretty,
        cors_allowed_origins: vec![ALLOWED_ORIGIN.to_string()],
    }
}

pub fn app(pool: PgPool) -> Router {
    todo_backend::build_app(AppState {
        pool,
        config: Arc::new(test_config()),
    })
}

/// Sends a request through the router in-process and returns the status and JSON body
/// (`Value::Null` when the body is empty). `token` is sent as a bearer token.
pub async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let body = match body {
        Some(json) => {
            builder = builder.header("content-type", "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };

    let response = app
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, json)
}

/// Registers `username` with a fixed valid password and returns a login token.
pub async fn register_and_login(app: &Router, username: &str) -> String {
    let credentials = serde_json::json!({ "username": username, "password": "password123" });

    let (status, _) = send(
        app,
        Method::POST,
        "/auth/register",
        Some(credentials.clone()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "register {username}");

    let (status, body) = send(app, Method::POST, "/auth/login", Some(credentials), None).await;
    assert_eq!(status, StatusCode::OK, "login {username}");
    body["token"].as_str().unwrap().to_string()
}
