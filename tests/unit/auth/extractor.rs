use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::get,
};
use sqlx::postgres::PgPoolOptions;
use todo_backend::{auth::AuthUser, auth::jwt, state::AppState};
use tower::ServiceExt;
use uuid::Uuid;

use crate::common::test_config;

/// A router with a single protected route that echoes the caller's id.
/// The pool is lazy and never connects; the extractor does not touch the database.
fn app() -> Router {
    let state = AppState {
        pool: PgPoolOptions::new()
            .connect_lazy("postgres://unused")
            .unwrap(),
        config: Arc::new(test_config()),
    };
    Router::new()
        .route(
            "/me",
            get(|AuthUser(id): AuthUser| async move { id.to_string() }),
        )
        .with_state(state)
}

async fn call(authorization: Option<&str>) -> (StatusCode, String) {
    let mut request = Request::builder().uri("/me");
    if let Some(value) = authorization {
        request = request.header(header::AUTHORIZATION, value);
    }
    let response = app()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn valid_bearer_token_yields_the_user_id() {
    let user_id = Uuid::new_v4();
    let token = jwt::issue(user_id, &test_config().jwt_secret, 60).unwrap();

    let (status, body) = call(Some(&format!("Bearer {token}"))).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, user_id.to_string());
}

#[tokio::test]
async fn missing_header_is_unauthorized() {
    assert_eq!(call(None).await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn non_bearer_scheme_is_unauthorized() {
    let token = jwt::issue(Uuid::new_v4(), &test_config().jwt_secret, 60).unwrap();

    let (status, _) = call(Some(&format!("Basic {token}"))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_token_is_unauthorized() {
    let (status, body) = call(Some("Bearer nonsense")).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, r#"{"error":"unauthorized"}"#);
}
