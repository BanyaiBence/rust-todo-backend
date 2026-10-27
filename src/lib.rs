pub mod config;
pub mod error;
pub mod routes;
pub mod telemetry;

use axum::{Router, routing::get};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

pub fn build_app() -> Router {
    Router::new()
        .route("/health", get(routes::health::health))
        .layer(TraceLayer::new_for_http().on_response(DefaultOnResponse::new().level(Level::INFO)))
}
