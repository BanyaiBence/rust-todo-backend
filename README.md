# todo-backend

A small JSON API for a todo app: users register, log in, and manage their own todo entries.

Built with **axum** and **tokio**, **sqlx** on **PostgreSQL**, **argon2** password hashing and
**JWT** bearer tokens. It runs as two containers, the app and its database, via Docker Compose.
It was developed test-first; see [PLAN.md](PLAN.md) for the design and milestones.

## Quick start

Requires Docker with Compose.

```sh
cp .env.example .env          # set JWT_SECRET to a long random value
docker compose up -d --build --wait
curl localhost:3000/health    # {"status":"ok"}
```

The first build compiles all dependencies and takes several minutes. Later builds reuse the
cached dependency layer. Stop the stack with `docker compose down`; add `-v` to also delete
the database volume.

## Configuration

The app reads environment variables (and `.env` when run directly with `cargo run`).

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | *required* | Postgres connection string. Set automatically by Compose for the `app` service. |
| `JWT_SECRET` | *required* | HMAC secret for signing tokens. Must not be empty. |
| `JWT_TTL_SECONDS` | `3600` | Token lifetime. |
| `BIND_ADDR` | `0.0.0.0:3000` | Listen address. |
| `LOG_FORMAT` | `pretty` | `pretty` or `json` (the Docker image uses `json`). |
| `RUST_LOG` | `todo_backend=info,tower_http=info` | Log filter ([`EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) syntax). |
| `CORS_ALLOWED_ORIGINS` | *(none)* | Comma-separated `http(s)://` origins allowed to call the API from a browser, or `*` for any. |

Compose-only settings: `APP_PORT` (host port, default `3000`) and `POSTGRES_PASSWORD`
(default `todo`).

## API

All request and response bodies are JSON. Protected endpoints require an
`Authorization: Bearer <token>` header with a token from `/auth/login`.

| Method | Path | Auth | Success |
|---|---|---|---|
| `GET` | `/health` | – | `200` |
| `POST` | `/auth/register` | – | `201` user |
| `POST` | `/auth/login` | – | `200` token |
| `GET` | `/todos?limit=&offset=` | Bearer | `200` array of todos |
| `POST` | `/todos` | Bearer | `201` todo |
| `GET` | `/todos/{id}` | Bearer | `200` todo |
| `PATCH` | `/todos/{id}` | Bearer | `200` todo |
| `DELETE` | `/todos/{id}` | Bearer | `204` |

The examples below use `curl` and [`jq`](https://jqlang.org/), with `API=http://localhost:3000`.

### Register

Usernames are 3–32 characters of `A-Z a-z 0-9 _ -`. Passwords are 8–128 characters.

```sh
curl -s $API/auth/register \
  -H 'content-type: application/json' \
  -d '{"username": "alice", "password": "correct-horse"}'
```

```json
{
  "id": "1d9d03e9-eded-4c3e-ad49-7e899df64bf3",
  "username": "alice",
  "created_at": "2026-10-27T22:18:24.288499Z"
}
```

Registering a taken username returns `409 {"error": "username already taken"}`.

### Log in

```sh
TOKEN=$(curl -s $API/auth/login \
  -H 'content-type: application/json' \
  -d '{"username": "alice", "password": "correct-horse"}' | jq -r .token)
```

Full response:

```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

A wrong password and an unknown username both return `401 {"error": "unauthorized"}`. Both
take the same time to answer, so responses don't reveal which usernames exist.

### Create a todo

`title` is required and trimmed, and must be 1–200 characters. `description` is optional,
up to 2000 characters.

```sh
curl -s $API/todos \
  -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"title": "Buy milk", "description": "2 litres, semi-skimmed"}'
```

```json
{
  "id": "74b182dc-c748-4f50-a1f6-d394f8705bf3",
  "title": "Buy milk",
  "description": "2 litres, semi-skimmed",
  "completed": false,
  "created_at": "2026-10-27T22:18:24.491487Z",
  "updated_at": "2026-10-27T22:18:24.491487Z"
}
```

### List todos

Returns the caller's todos, oldest first. Use `limit` (1–100, default 50) and `offset`
(default 0) to page through them. The response is a plain array; a page shorter than
`limit` is the last one.

```sh
curl -s "$API/todos?limit=1&offset=1" -H "authorization: Bearer $TOKEN"
```

```json
[
  {
    "id": "48dfc931-fd5d-4800-b30a-6ad96e28b43a",
    "title": "Write README",
    "description": null,
    "completed": false,
    "created_at": "2026-10-27T22:18:24.513863Z",
    "updated_at": "2026-10-27T22:18:24.513863Z"
  }
]
```

### Get a todo

```sh
curl -s $API/todos/74b182dc-c748-4f50-a1f6-d394f8705bf3 -H "authorization: Bearer $TOKEN"
```

Returns the todo, or `404` if it doesn't exist or belongs to another user.

### Update a todo

`PATCH` changes only the fields you send: `title`, `description`, `completed`. Omitting a field
leaves it unchanged; `"description": null` clears the description. A body with no fields is
rejected.

```sh
curl -s -X PATCH $API/todos/74b182dc-c748-4f50-a1f6-d394f8705bf3 \
  -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"completed": true, "description": null}'
```

```json
{
  "id": "74b182dc-c748-4f50-a1f6-d394f8705bf3",
  "title": "Buy milk",
  "description": null,
  "completed": true,
  "created_at": "2026-10-27T22:18:24.491487Z",
  "updated_at": "2026-10-27T22:18:24.590827Z"
}
```

### Delete a todo

```sh
curl -s -X DELETE $API/todos/74b182dc-c748-4f50-a1f6-d394f8705bf3 \
  -H "authorization: Bearer $TOKEN" -w '%{http_code}\n'
# 204
```

Deleting it again, or fetching it afterwards, returns `404`.

### Errors

Every error has the same shape:

```json
{ "error": "title must be 1-200 characters" }
```

| Status | When |
|---|---|
| `401` | Missing, malformed, invalid or expired token; wrong login credentials |
| `404` | Todo doesn't exist, belongs to another user, or the id isn't a UUID |
| `409` | Username already taken |
| `413` | Request body over 64 KiB |
| `422` | Validation failure, malformed JSON, unknown fields, or invalid `limit`/`offset` |
| `500` | Server error. Details are logged, never returned. |

Unknown fields are rejected so typos don't fail silently:

```sh
curl -s $API/todos -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' -d '{"title": "x", "done": true}'
# {"error":"Failed to deserialize the JSON body into the target type: done: unknown field `done`, expected `title` or `description` at line 1 column 19"}
```

## Development

Requires Rust 1.95+, Docker, and sqlx-cli:

```sh
cargo install sqlx-cli --no-default-features --features rustls,postgres
cp .env.example .env                                # DATABASE_URL points at db-test (port 5433)
docker compose --profile test up -d --wait db-test  # throwaway Postgres for tests
sqlx migrate run                                    # schema for compile-time query checks
cargo run                                           # http://localhost:3000
```

The SQL queries are checked at compile time. After changing a query, regenerate the offline
cache with `cargo sqlx prepare -- --all-targets` and commit `.sqlx/`. Docker and CI builds use
that cache instead of a live database.

### Tests

All tests live under `tests/`, mirroring the `src/` path of the code they cover:

| Command | Suite | Needs |
|---|---|---|
| `cargo test --test unit` | Pure logic: config, errors, validation, hashing, JWT, auth extractor | nothing |
| `cargo test --test repo` | Repository queries; each test gets a fresh migrated database | `db-test` |
| `cargo test --test api` | HTTP behavior through the full router | `db-test` |
| `cargo test --test e2e -- --ignored` | Smoke tests over real HTTP | running Compose stack |

`cargo test` runs the first three. `E2E_BASE_URL` points the e2e tests at another deployment.

### CI

`.github/workflows/ci.yml` runs on pushes to `main` and on pull requests. It has two jobs:

1. **Lint and test:** `cargo fmt --check`, `clippy -D warnings`, and the test suite against a
   Postgres service.
2. **Docker and e2e:** builds the Compose stack and runs the e2e tests against it.

## Project layout

```
src/
├── main.rs        # startup: config, logging, DB pool + migrations, graceful shutdown
├── lib.rs         # router, CORS and body-limit layers
├── config.rs      # environment parsing
├── error.rs       # AppError → HTTP status + JSON body
├── extract.rs     # Json/Path/Query extractors with consistent errors
├── auth/          # password hashing, JWT, AuthUser extractor
├── models/        # entities and request/response types with validation
├── repo/          # SQL queries (users, todos)
└── routes/        # handlers: health, auth, todos
migrations/        # schema, applied automatically on startup
tests/             # unit, repo, api and e2e suites + shared helpers
```
