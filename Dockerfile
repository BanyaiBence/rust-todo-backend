# syntax=docker/dockerfile:1

# Builder and runtime share the same Debian release so glibc versions match.
ARG RUST_VERSION=1.95
ARG DEBIAN_RELEASE=trixie

FROM lukemathwalker/cargo-chef:latest-rust-${RUST_VERSION}-slim-${DEBIAN_RELEASE} AS chef
WORKDIR /app

# Compute a dependency-only recipe so dependencies are cached until Cargo.toml/Cargo.lock change.
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
# Check queries against the committed .sqlx cache instead of a live database.
ENV SQLX_OFFLINE=true
RUN cargo build --release --locked --bin todo-backend

FROM debian:${DEBIAN_RELEASE}-slim AS runtime
RUN useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin app
COPY --from=builder /app/target/release/todo-backend /usr/local/bin/todo-backend
USER app
ENV BIND_ADDR=0.0.0.0:3000 \
    LOG_FORMAT=json \
    RUST_LOG=todo_backend=info,tower_http=info
EXPOSE 3000
# The slim image has no curl; probe /health with bash's /dev/tcp.
HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=5 \
    CMD ["bash", "-c", "exec 3<>/dev/tcp/127.0.0.1/3000 && printf 'GET /health HTTP/1.0\\r\\n\\r\\n' >&3 && grep -q '200 OK' <&3"]
ENTRYPOINT ["todo-backend"]
