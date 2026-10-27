mod common;

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn register(app: &axum::Router, body: Value) -> (StatusCode, Value) {
    common::send(app, Method::POST, "/auth/register", Some(body), None).await
}

async fn login(app: &axum::Router, body: Value) -> (StatusCode, Value) {
    common::send(app, Method::POST, "/auth/login", Some(body), None).await
}

#[sqlx::test]
async fn register_creates_user_without_exposing_password(pool: PgPool) {
    let app = common::app(pool);

    let (status, body) = register(
        &app,
        json!({ "username": "alice", "password": "password123" }),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["username"], "alice");
    assert!(body["id"].is_string());
    assert!(body["created_at"].is_string());
    assert!(body.get("password").is_none());
    assert!(body.get("password_hash").is_none());
}

#[sqlx::test]
async fn register_stores_a_hash_not_the_password(pool: PgPool) {
    let app = common::app(pool.clone());
    register(
        &app,
        json!({ "username": "alice", "password": "password123" }),
    )
    .await;

    let stored: String =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE username = 'alice'")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert!(stored.starts_with("$argon2id$"));
}

#[sqlx::test]
async fn register_duplicate_username_is_conflict(pool: PgPool) {
    let app = common::app(pool);
    register(
        &app,
        json!({ "username": "alice", "password": "password123" }),
    )
    .await;

    let (status, body) = register(
        &app,
        json!({ "username": "alice", "password": "different123" }),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body, json!({ "error": "username already taken" }));
}

#[sqlx::test]
async fn register_invalid_input_is_unprocessable(pool: PgPool) {
    let app = common::app(pool);

    let (status, body) =
        register(&app, json!({ "username": "al", "password": "password123" })).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"].is_string());
}

#[sqlx::test]
async fn malformed_body_gets_a_json_error(pool: PgPool) {
    let app = common::app(pool);

    let (status, body) = register(&app, json!({ "username": "alice" })).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"].as_str().unwrap().contains("password"));
}

#[sqlx::test]
async fn login_returns_a_bearer_token(pool: PgPool) {
    let app = common::app(pool.clone());
    register(
        &app,
        json!({ "username": "alice", "password": "password123" }),
    )
    .await;

    let (status, body) = login(
        &app,
        json!({ "username": "alice", "password": "password123" }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["token_type"], "Bearer");
    assert_eq!(body["expires_in"], 3600);
    let token = body["token"].as_str().unwrap();
    let claims = todo_backend::auth::jwt::verify(token, &common::test_config().jwt_secret).unwrap();
    let alice_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM users WHERE username = 'alice'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(claims.sub, alice_id);
}

#[sqlx::test]
async fn wrong_password_and_unknown_user_are_indistinguishable(pool: PgPool) {
    let app = common::app(pool);
    register(
        &app,
        json!({ "username": "alice", "password": "password123" }),
    )
    .await;

    let wrong_password = login(
        &app,
        json!({ "username": "alice", "password": "nope-nope" }),
    )
    .await;
    let unknown_user = login(
        &app,
        json!({ "username": "mallory", "password": "password123" }),
    )
    .await;

    assert_eq!(wrong_password.0, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong_password, unknown_user);
}
