use axum::{
    body::Body,
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::common::{self, ALLOWED_ORIGIN};

async fn preflight(pool: PgPool, origin: &str) -> (StatusCode, HeaderMap) {
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/todos")
        .header(header::ORIGIN, origin)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(
            header::ACCESS_CONTROL_REQUEST_HEADERS,
            "authorization,content-type",
        )
        .body(Body::empty())
        .unwrap();
    let response = common::app(pool).oneshot(request).await.unwrap();
    (response.status(), response.headers().clone())
}

#[sqlx::test]
async fn preflight_from_allowed_origin_is_permitted(pool: PgPool) {
    let (status, headers) = preflight(pool, ALLOWED_ORIGIN).await;

    assert!(status.is_success());
    assert_eq!(headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], ALLOWED_ORIGIN);
    let methods = headers[header::ACCESS_CONTROL_ALLOW_METHODS]
        .to_str()
        .unwrap();
    assert!(
        methods.contains("PATCH") && methods.contains("DELETE"),
        "{methods}"
    );
    let allowed_headers = headers[header::ACCESS_CONTROL_ALLOW_HEADERS]
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    assert!(
        allowed_headers.contains("authorization"),
        "{allowed_headers}"
    );
}

#[sqlx::test]
async fn preflight_from_unknown_origin_gets_no_cors_headers(pool: PgPool) {
    let (_, headers) = preflight(pool, "https://evil.example.com").await;

    assert!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
}

#[sqlx::test]
async fn simple_request_from_allowed_origin_carries_cors_header(pool: PgPool) {
    let request = Request::builder()
        .uri("/health")
        .header(header::ORIGIN, ALLOWED_ORIGIN)
        .body(Body::empty())
        .unwrap();

    let response = common::app(pool).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        ALLOWED_ORIGIN
    );
}
