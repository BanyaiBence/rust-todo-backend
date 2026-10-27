use axum::{http::StatusCode, response::IntoResponse};
use serde_json::{Value, json};
use todo_backend::error::AppError;

async fn render(err: AppError) -> (StatusCode, Value) {
    let response = err.into_response();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn not_found_is_404() {
    let (status, body) = render(AppError::NotFound).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({ "error": "not found" }));
}

#[tokio::test]
async fn unauthorized_is_401() {
    let (status, body) = render(AppError::Unauthorized).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, json!({ "error": "unauthorized" }));
}

#[tokio::test]
async fn conflict_is_409_with_message() {
    let (status, body) = render(AppError::Conflict("username already taken".into())).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body, json!({ "error": "username already taken" }));
}

#[tokio::test]
async fn validation_is_422_with_message() {
    let (status, body) = render(AppError::Validation("title must not be empty".into())).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body, json!({ "error": "title must not be empty" }));
}

#[tokio::test]
async fn internal_is_500_without_leaking_details() {
    let (status, body) = render(AppError::Internal("secret detail".into())).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, json!({ "error": "internal server error" }));
}

#[tokio::test]
async fn database_error_is_500_without_leaking_details() {
    let (status, body) = render(AppError::Database(sqlx::Error::PoolTimedOut)).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, json!({ "error": "internal server error" }));
}

#[test]
fn row_not_found_converts_to_not_found() {
    assert!(matches!(
        AppError::from(sqlx::Error::RowNotFound),
        AppError::NotFound
    ));
}

#[test]
fn other_sqlx_errors_convert_to_database() {
    assert!(matches!(
        AppError::from(sqlx::Error::PoolTimedOut),
        AppError::Database(_)
    ));
}

#[tokio::test]
async fn payload_too_large_is_413() {
    let (status, body) = render(AppError::PayloadTooLarge).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body, json!({ "error": "request body too large" }));
}
