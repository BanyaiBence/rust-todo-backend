use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Validation(String),
    #[error(transparent)]
    Database(sqlx::Error),
    #[error("{0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Database(_) | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // Never expose server-side failure details to clients; log them instead.
        let message = if status.is_server_error() {
            tracing::error!(error = %self, "request failed");
            "internal server error".to_string()
        } else {
            self.to_string()
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound,
            err if err
                .as_database_error()
                .is_some_and(|db| db.is_unique_violation()) =>
            {
                AppError::Conflict("resource already exists".to_string())
            }
            err => AppError::Database(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

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
}
