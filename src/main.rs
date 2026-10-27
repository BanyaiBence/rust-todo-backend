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
        pool: pool.clone(),
        config: Arc::new(config),
    };
    axum::serve(listener, build_app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");

    pool.close().await;
    tracing::info!("shutdown complete");
}

/// Resolves on Ctrl-C or SIGTERM (what `docker stop` sends). In-flight requests are then
/// allowed to finish before the server exits.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received, draining connections");
}
