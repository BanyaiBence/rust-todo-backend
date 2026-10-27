use axum::{extract::FromRequest, extract::rejection::JsonRejection};

use crate::error::AppError;

/// `axum::Json`, but malformed or mismatched bodies become an `AppError` so every
/// error response shares the `{"error": "..."}` shape.
#[derive(Debug, FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct AppJson<T>(pub T);

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        AppError::Validation(rejection.body_text())
    }
}
