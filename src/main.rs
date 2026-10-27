use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;
use todo_backend::{build_app, config::Config, state::AppState, telemetry};

#[tokio::main]
async fn main() {
    let config = Config::from_env().unwrap_or_else(|err| {
        eprintln!("configuration error: {err}");
        std::process::exit(1);
    });
    telemetry::init(&config.log_format);

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .expect("failed to connect to database");
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("failed to run database migrations");

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .expect("failed to bind address");
    tracing::info!(addr = %config.bind_addr, "listening");

    let state = AppState {
        pool,
        config: Arc::new(config),
    };
    axum::serve(listener, build_app(state))
        .await
        .expect("server error");
}
