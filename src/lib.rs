pub mod config;
pub mod error;
pub mod models;
pub mod repo;
pub mod routes;
pub mod state;
pub mod telemetry;

use axum::{Router, routing::get};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::state::AppState;

pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(routes::health::health))
        .layer(TraceLayer::new_for_http().on_response(DefaultOnResponse::new().level(Level::INFO)))
        .with_state(state)
}
