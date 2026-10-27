pub mod auth;
pub mod config;
pub mod error;
pub mod extract;
pub mod models;
pub mod repo;
pub mod routes;
pub mod state;
pub mod telemetry;

use std::time::Duration;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, header},
    routing::{get, post},
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{config::Config, state::AppState};

/// Largest accepted request body. Todos are small; this leaves ample headroom.
pub const MAX_BODY_BYTES: usize = 64 * 1024;

pub fn build_app(state: AppState) -> Router {
    let cors = cors_layer(&state.config);

    Router::new()
        .route("/health", get(routes::health::health))
        .route("/auth/register", post(routes::auth::register))
        .route("/auth/login", post(routes::auth::login))
        .route(
            "/todos",
            get(routes::todos::list).post(routes::todos::create),
        )
        .route(
            "/todos/{id}",
            get(routes::todos::get)
                .patch(routes::todos::update)
                .delete(routes::todos::delete),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(cors)
        .layer(TraceLayer::new_for_http().on_response(DefaultOnResponse::new().level(Level::INFO)))
        .with_state(state)
}

/// Allows browser clients on the configured origins. With no origins configured the layer
/// matches nothing, so cross-origin browser requests are refused.
fn cors_layer(config: &Config) -> CorsLayer {
    let origins = if config.cors_allowed_origins == ["*"] {
        AllowOrigin::any()
    } else {
        AllowOrigin::list(
            config
                .cors_allowed_origins
                .iter()
                .filter_map(|origin| HeaderValue::from_str(origin).ok()),
        )
    };

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(Duration::from_secs(3600))
}
