pub mod config;
pub mod routes;

use axum::{Router, routing::get};

pub fn build_app() -> Router {
    Router::new().route("/health", get(routes::health::health))
}
