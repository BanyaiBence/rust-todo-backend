use todo_backend::{build_app, config::Config};

#[tokio::main]
async fn main() {
    let config = Config::from_env().unwrap_or_else(|err| {
        eprintln!("configuration error: {err}");
        std::process::exit(1);
    });

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .expect("failed to bind address");
    println!("listening on {}", config.bind_addr);

    axum::serve(listener, build_app())
        .await
        .expect("server error");
}
