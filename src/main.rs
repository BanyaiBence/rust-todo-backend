use todo_backend::{build_app, config::Config, telemetry};

#[tokio::main]
async fn main() {
    let config = Config::from_env().unwrap_or_else(|err| {
        eprintln!("configuration error: {err}");
        std::process::exit(1);
    });
    telemetry::init(&config.log_format);

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .expect("failed to bind address");
    tracing::info!(addr = %config.bind_addr, "listening");

    axum::serve(listener, build_app())
        .await
        .expect("server error");
}
