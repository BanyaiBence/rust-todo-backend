use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use crate::{auth::jwt, error::AppError, state::AppState};

/// The authenticated caller, taken from an `Authorization: Bearer <jwt>` header.
/// Adding it as a handler argument makes the route require authentication.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser(pub Uuid);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        let claims = jwt::verify(token, &state.config.jwt_secret)?;
        Ok(AuthUser(claims.sub))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode, header},
        routing::get,
    };
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    use super::*;
    use crate::config::{Config, LogFormat};

    const SECRET: &str = "test-secret";

    /// A router with a single protected route that echoes the caller's id.
    /// The pool is lazy and never connects; the extractor does not touch the database.
    fn app() -> Router {
        let state = AppState {
            pool: PgPoolOptions::new()
                .connect_lazy("postgres://unused")
                .unwrap(),
            config: Arc::new(Config {
                database_url: String::new(),
                jwt_secret: SECRET.to_string(),
                jwt_ttl_seconds: 60,
                bind_addr: "127.0.0.1:0".parse().unwrap(),
                log_format: LogFormat::Pretty,
            }),
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
        let token = jwt::issue(user_id, SECRET, 60).unwrap();

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
        let token = jwt::issue(Uuid::new_v4(), SECRET, 60).unwrap();

        let (status, _) = call(Some(&format!("Basic {token}"))).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn invalid_token_is_unauthorized() {
        let (status, body) = call(Some("Bearer nonsense")).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body, r#"{"error":"unauthorized"}"#);
    }
}
