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
└── tests/
    ├── common/mod.rs           # app builder, request helper, register+login helper
    ├── health.rs
    ├── auth.rs
    └── todos.rs
```

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

- **Dockerfile:** multi-stage; builder with cargo-chef for dependency caching and
  `SQLX_OFFLINE=true`; slim runtime image, non-root user.
- **docker-compose.yml:**
  - `db`: `postgres:17-alpine`, named volume, `pg_isready` healthcheck
  - `app`: depends on `db` being healthy, `env_file: .env`, port 3000
  - `db-test` (profile `test`): tmpfs-backed Postgres on port 5433 for `cargo test`

## 8. TDD workflow

- DB-backed tests use `#[sqlx::test(migrations = "./migrations")]` — a fresh migrated database per
  test, parallel-safe. Requires `DATABASE_URL` pointing at a reachable Postgres
  (`docker compose --profile test up -d db-test`).
- Pure logic gets `#[cfg(test)]` unit tests (hashing, JWT, error mapping, config parsing).
- Each step: 🔴 failing test → 🟢 minimal implementation → ♻️ refactor → commit.

## 9. Milestones

### M0 — Scaffolding
- [x] Dependencies, module layout, `lib.rs` with `build_app`
- [x] 🔴/🟢 `GET /health` → 200 `{"status":"ok"}`
- [x] 🔴/🟢 `Config` parsing: valid env, missing vars, invalid values

### M1 — Errors & infrastructure
- [ ] 🔴/🟢 `AppError` → status code + JSON body for each variant
- [ ] Tracing init + `TraceLayer`

### M2 — Database & repositories
- [ ] Migrations, `AppState { pool, config }`, `db-test` compose service
- [ ] 🔴/🟢 `user_repo`: create, find by username, duplicate → `Conflict`
- [ ] 🔴/🟢 `todo_repo`: create, list by user, get/update/delete scoped to user

### M3 — Authentication
- [ ] 🔴/🟢 Unit: hash/verify, JWT round-trip, expired/tampered rejected
- [ ] 🔴/🟢 HTTP: register 201 / 409 / 422; login 200 / 401 (same message for wrong password and unknown user)
- [ ] 🔴/🟢 `AuthUser` extractor: missing/malformed/invalid token → 401

### M4 — Todo CRUD
- [ ] 🔴/🟢 All `/todos` routes → 401 without token
- [ ] 🔴/🟢 Create 201; empty/too-long title → 422
- [ ] 🔴/🟢 List returns only caller's todos
- [ ] 🔴/🟢 Get/PATCH/DELETE own → OK; other user's → 404
- [ ] 🔴/🟢 PATCH is partial and bumps `updated_at`
- [ ] 🔴/🟢 DELETE then GET → 404

### M5 — Docker & E2E
- [ ] Dockerfile, docker-compose.yml, `.env.example`, `cargo sqlx prepare`
- [ ] `#[ignore]` smoke test against `docker compose up`: register → login → CRUD

### M6 — Hardening (optional)
- [ ] CORS, body size limit, graceful shutdown, pagination
- [ ] CI: fmt, clippy `-D warnings`, test (Postgres service), docker build
