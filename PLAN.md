# Todo Backend — Implementation Plan

Rust backend for a simple todo app: users register/log in and manage their own todo entries.

**Stack:** axum · tokio · serde · sqlx (PostgreSQL) · dotenvy · tracing · thiserror · argon2 · jsonwebtoken
**Runtime:** Docker — app container + separate PostgreSQL container (docker compose)
**Process:** test-driven development (red → green → refactor)

## Decisions

| Topic | Decision |
|---|---|
| Auth mechanism | Stateless JWT bearer tokens (`jsonwebtoken`), passwords hashed with `argon2` |
| Login identifier | Username |
| Token lifecycle | Short-lived access token only; no refresh/logout flow |
| Queries | Compile-time-checked `sqlx::query!` / `query_as!`; `.sqlx/` offline cache committed for Docker builds |
| Foreign todos | Accessing another user's todo returns **404** (no existence leak) |
| Todo validation | Title trimmed, 1–200 chars; description ≤ 2000 chars; unknown JSON fields → 422 |
| PATCH semantics | Omitted field = unchanged; `"description": null` clears it; empty body → 422 |
| Malformed ids | A non-UUID `{id}` path segment → 404 |

## 1. Architecture

```
HTTP (axum handlers, extractors, DTOs)
   ↓
Service / domain logic (validation, ownership, hashing, tokens)
   ↓
Repository (sqlx queries against PgPool)
```

```
todo-backend/
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml          # app + postgres (+ test db profile)
├── .env.example
├── migrations/
├── src/
│   ├── main.rs                 # thin: config, tracing, build app, serve
│   ├── lib.rs                  # build_app() exposed for tests
│   ├── config.rs               # dotenvy + env parsing
│   ├── error.rs                # AppError (thiserror) + IntoResponse
│   ├── state.rs                # AppState { pool, config }
│   ├── auth/                   # password hashing, JWT, AuthUser extractor
│   ├── routes/                 # health, auth, todos
│   ├── models/                 # entities + request/response DTOs
│   └── repo/                   # user_repo, todo_repo
└── tests/                      # all tests; src/ contains application code only
    ├── common/mod.rs           # shared helpers: test config, app builder, request helper, register+login
    ├── unit/                   # binary `unit`: pure logic, no database
    │   ├── main.rs
    │   ├── config.rs
    │   ├── error.rs
    │   ├── auth/{password,jwt,extractor}.rs
    │   └── models/user.rs
    ├── repo/                   # binary `repo`: repositories against PostgreSQL
    │   ├── main.rs
    │   ├── user_repo.rs
    │   └── todo_repo.rs
    ├── api/                    # binary `api`: HTTP tests through the full router
    │   ├── main.rs
    │   ├── health.rs
    │   ├── auth.rs
    │   └── todos.rs
    └── e2e/main.rs             # binary `e2e`: #[ignore]d smoke tests against the running stack
```

Test files mirror the `src/` path of the code they test. Each directory under `tests/` is one
test binary (`cargo test --test unit|repo|api`), which keeps link times down and lets the
database-free `unit` suite run anywhere.

`main.rs` stays tiny; everything lives in the library so integration tests drive the real
`Router` in-process via `tower::ServiceExt::oneshot` (no port binding).

## 2. Data model

```sql
CREATE TABLE users (
    id            UUID PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE todos (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title       TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    description TEXT,
    completed   BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_todos_user_id ON todos(user_id);
```

Migrations run at startup via `sqlx::migrate!()`.

## 3. API

| Method | Path | Auth | Success | Notable errors |
|---|---|---|---|---|
| GET | `/health` | – | 200 | – |
| POST | `/auth/register` | – | 201 + user | 409 taken, 422 invalid |
| POST | `/auth/login` | – | 200 `{token}` | 401 |
| GET | `/todos` | Bearer | 200 (own only) | 401 |
| POST | `/todos` | Bearer | 201 | 422 |
| GET | `/todos/{id}` | Bearer | 200 | 404 |
| PATCH | `/todos/{id}` | Bearer | 200 (partial update) | 404, 422 |
| DELETE | `/todos/{id}` | Bearer | 204 | 404 |

All todo queries are scoped with `WHERE id = $1 AND user_id = $2`.
`AuthUser(Uuid)` implements `FromRequestParts`, validating `Authorization: Bearer <jwt>`.

## 4. Errors

`AppError` (thiserror): `NotFound`, `Unauthorized`, `Conflict(String)`, `Validation(String)`,
`Database(#[from] sqlx::Error)`, `Internal(..)`. `IntoResponse` maps to status + `{"error": "..."}`.
Database/internal errors are logged with `tracing::error!` and returned as a generic 500.
Postgres unique violations (`23505`) map to `Conflict`.

## 5. Configuration

`Config::from_env()` loads `.env` via dotenvy and fails fast on missing values. Parsing is done
through an injectable lookup function so it is unit-testable without mutating process env.

```
DATABASE_URL=postgres://todo:todo@localhost:5432/todo
JWT_SECRET=change-me
JWT_TTL_SECONDS=3600
BIND_ADDR=0.0.0.0:3000
RUST_LOG=todo_backend=debug,tower_http=info
LOG_FORMAT=pretty   # or json
```

## 6. Logging

`tracing_subscriber` with `EnvFilter`; pretty locally, JSON when `LOG_FORMAT=json`.
`tower_http::trace::TraceLayer` on the router. `#[tracing::instrument]` on service/repo functions,
never recording passwords or tokens.

## 7. Docker

- **Dockerfile:** multi-stage; cargo-chef caches the dependency build, `SQLX_OFFLINE=true` builds
  against the committed `.sqlx/` cache; `debian:trixie-slim` runtime (~130 MB), non-root user,
  JSON logs, `/health` healthcheck. Migrations are embedded in the binary.
- **docker-compose.yml:**
  - `db`: `postgres:17-alpine`, named volume `pgdata`, `pg_isready` healthcheck, not published to the host
  - `app`: waits for `db` to be healthy, port `${APP_PORT:-3000}`, `init: true` for signal forwarding.
    `DATABASE_URL` is set explicitly (the local `.env` targets `db-test`); only `JWT_SECRET`
    (required), `JWT_TTL_SECONDS` and `POSTGRES_PASSWORD` are taken from the environment / `.env`
  - `db-test` (profile `test`): tmpfs-backed Postgres on port 5433 for `cargo test`

## 8. TDD workflow

- DB-backed tests use `#[sqlx::test(migrations = "./migrations")]` — a fresh migrated database per
  test, parallel-safe. Requires `DATABASE_URL` pointing at a reachable Postgres
  (`docker compose --profile test up -d db-test`).
- Pure logic (hashing, JWT, error mapping, config parsing, validation) is tested in `tests/unit/`.
- No `#[cfg(test)]` modules in `src/`; library/binary test harnesses are disabled in `Cargo.toml`.
- Each step: 🔴 failing test → 🟢 minimal implementation → ♻️ refactor → commit.

## 9. Local development

```sh
cargo install sqlx-cli --no-default-features --features rustls,postgres
cp .env.example .env                               # points DATABASE_URL at db-test (port 5433)
docker compose --profile test up -d --wait db-test
sqlx migrate run                                   # schema for compile-time query checks
cargo test
cargo sqlx prepare -- --all-targets                # after changing any query; commit .sqlx/

docker compose up -d --build --wait                # full stack on http://localhost:3000
cargo test --test e2e -- --ignored                 # end-to-end smoke tests against it
```

## 10. Milestones

### M0 — Scaffolding
- [x] Dependencies, module layout, `lib.rs` with `build_app`
- [x] 🔴/🟢 `GET /health` → 200 `{"status":"ok"}`
- [x] 🔴/🟢 `Config` parsing: valid env, missing vars, invalid values

### M1 — Errors & infrastructure
- [x] 🔴/🟢 `AppError` → status code + JSON body for each variant; `sqlx::Error` conversion
- [x] Tracing init (pretty/JSON) + `TraceLayer` (responses logged at INFO)

### M2 — Database & repositories
- [x] Migrations, `AppState { pool, config }`, `db-test` compose service, migrations on startup
- [x] 🔴/🟢 `user_repo`: create, find by username, duplicate → `Conflict`
- [x] 🔴/🟢 `todo_repo`: create, list by user, get/update/delete scoped to user, cascade on user delete
- [x] `.sqlx/` offline query cache (pulled forward from M5 so builds work without a DB)

### M3 — Authentication
- [x] 🔴/🟢 Unit: hash/verify, JWT round-trip, expired/tampered rejected
- [x] 🔴/🟢 Unit: registration rules (username 3–32 of `[A-Za-z0-9_-]`, password 8–128 chars)
- [x] 🔴/🟢 HTTP: register 201 / 409 / 422; login 200 / 401 (same message for wrong password and unknown user)
- [x] 🔴/🟢 `AuthUser` extractor: missing/malformed/invalid token → 401
- [x] Unknown-user logins verify against a dummy hash (no timing-based username probing)
- [x] `AppJson` extractor: malformed request bodies → 422 with the standard JSON error shape

### M4 — Todo CRUD
- [x] 🔴/🟢 Unit: `CreateTodo` / `UpdateTodo` validation and null-vs-omitted description
- [x] 🔴/🟢 All `/todos` routes → 401 without token
- [x] 🔴/🟢 Create 201; empty/blank/too-long title, too-long description, unknown field → 422
- [x] 🔴/🟢 List returns only caller's todos, oldest first
- [x] 🔴/🟢 Get/PATCH/DELETE own → OK; other user's, unknown or malformed id → 404
- [x] 🔴/🟢 PATCH is partial, can clear description, and bumps `updated_at`
- [x] 🔴/🟢 DELETE → 204, then GET/DELETE → 404

### M5 — Docker & E2E
- [x] Dockerfile, `db` + `app` services in docker-compose.yml, `.dockerignore`
- [x] `tests/e2e`: `#[ignore]` smoke tests against `docker compose up` (health; register → login → CRUD)
- [x] Verified: non-root container, data persists across `down`/`up`, fast shutdown

### M6 — Hardening (optional)
- [ ] CORS, body size limit, graceful shutdown, pagination
- [ ] CI: fmt, clippy `-D warnings`, test (Postgres service), docker build
