use axum::{
    extract::{
        FromRequest, FromRequestParts,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::StatusCode,
};

use crate::error::AppError;

/// `axum::Json`, but malformed or mismatched bodies become an `AppError` so every
/// error response shares the `{"error": "..."}` shape.
#[derive(Debug, FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct AppJson<T>(pub T);

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
            AppError::PayloadTooLarge
        } else {
            AppError::Validation(rejection.body_text())
        }
    }
}

/// `axum::extract::Path`; a path segment that does not parse (e.g. a malformed id)
/// cannot name an existing resource, so it is reported as `NotFound`.
#[derive(Debug, FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(AppError))]
pub struct AppPath<T>(pub T);

impl From<PathRejection> for AppError {
    fn from(rejection: PathRejection) -> Self {
        tracing::debug!(%rejection, "unparsable path parameter");
        AppError::NotFound
    }
}

/// `axum::extract::Query`, with unparsable query strings reported as validation errors.
#[derive(Debug, FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(AppError))]
pub struct AppQuery<T>(pub T);

impl From<QueryRejection> for AppError {
    fn from(rejection: QueryRejection) -> Self {
        AppError::Validation(rejection.body_text())
    }
}
