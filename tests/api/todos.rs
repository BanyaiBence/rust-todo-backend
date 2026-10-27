use axum::{
    Router,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::common::{self, register_and_login, send};

async fn create_todo(app: &Router, token: &str, body: Value) -> Value {
    let (status, todo) = send(app, Method::POST, "/todos", Some(body), Some(token)).await;
    assert_eq!(status, StatusCode::CREATED);
    todo
}

fn todo_uri(todo: &Value) -> String {
    format!("/todos/{}", todo["id"].as_str().unwrap())
}

#[sqlx::test]
async fn all_todo_routes_require_authentication(pool: PgPool) {
    let app = common::app(pool);
    let id = uuid::Uuid::new_v4();
    let item = format!("/todos/{id}");
    let routes = [
        (Method::GET, "/todos"),
        (Method::POST, "/todos"),
        (Method::GET, item.as_str()),
        (Method::PATCH, item.as_str()),
        (Method::DELETE, item.as_str()),
    ];

    for (method, uri) in routes {
        let (status, body) = send(&app, method.clone(), uri, Some(json!({})), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        assert_eq!(body, json!({ "error": "unauthorized" }));
    }
}

#[sqlx::test]
async fn create_returns_the_new_todo(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;

    let todo = create_todo(
        &app,
        &token,
        json!({ "title": "buy milk", "description": "2 litres" }),
    )
    .await;

    assert!(todo["id"].is_string());
    assert_eq!(todo["title"], "buy milk");
    assert_eq!(todo["description"], "2 litres");
    assert_eq!(todo["completed"], false);
    assert!(todo["created_at"].is_string());
    assert_eq!(todo["created_at"], todo["updated_at"]);
    assert!(todo.get("user_id").is_none());
}

#[sqlx::test]
async fn create_without_description_stores_null(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;

    let todo = create_todo(&app, &token, json!({ "title": "buy milk" })).await;

    assert_eq!(todo["description"], Value::Null);
}

#[sqlx::test]
async fn create_with_invalid_body_is_unprocessable(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;

    for body in [
        json!({ "title": "" }),
        json!({ "title": "   " }),
        json!({ "title": "t".repeat(201) }),
        json!({ "title": "x", "description": "d".repeat(2001) }),
        json!({ "title": "x", "done": true }),
        json!({ "description": "no title" }),
    ] {
        let (status, response) = send(
            &app,
            Method::POST,
            "/todos",
            Some(body.clone()),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "body: {body}");
        assert!(response["error"].is_string());
    }
}

#[sqlx::test]
async fn list_returns_only_own_todos_oldest_first(pool: PgPool) {
    let app = common::app(pool);
    let alice = register_and_login(&app, "alice").await;
    let bob = register_and_login(&app, "bob").await;
    create_todo(&app, &alice, json!({ "title": "first" })).await;
    create_todo(&app, &bob, json!({ "title": "bobs" })).await;
    create_todo(&app, &alice, json!({ "title": "second" })).await;

    let (status, body) = send(&app, Method::GET, "/todos", None, Some(&alice)).await;

    assert_eq!(status, StatusCode::OK);
    let titles: Vec<_> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["title"].clone())
        .collect();
    assert_eq!(titles, [json!("first"), json!("second")]);
}

#[sqlx::test]
async fn list_is_empty_for_a_new_user(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;

    let (status, body) = send(&app, Method::GET, "/todos", None, Some(&token)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
}

#[sqlx::test]
async fn get_returns_own_todo(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;
    let todo = create_todo(&app, &token, json!({ "title": "x" })).await;

    let (status, body) = send(&app, Method::GET, &todo_uri(&todo), None, Some(&token)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, todo);
}

#[sqlx::test]
async fn other_users_todo_is_not_found_for_every_method(pool: PgPool) {
    let app = common::app(pool);
    let alice = register_and_login(&app, "alice").await;
    let bob = register_and_login(&app, "bob").await;
    let todo = create_todo(&app, &alice, json!({ "title": "private" })).await;
    let uri = todo_uri(&todo);

    for (method, body) in [
        (Method::GET, None),
        (Method::PATCH, Some(json!({ "title": "hijacked" }))),
        (Method::DELETE, None),
    ] {
        let (status, response) = send(&app, method.clone(), &uri, body, Some(&bob)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method}");
        assert_eq!(response, json!({ "error": "not found" }));
    }

    let (status, unchanged) = send(&app, Method::GET, &uri, None, Some(&alice)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(unchanged, todo);
}

#[sqlx::test]
async fn unknown_or_malformed_id_is_not_found(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;
    let unknown = format!("/todos/{}", uuid::Uuid::new_v4());

    for uri in [unknown.as_str(), "/todos/not-a-uuid"] {
        let (status, body) = send(&app, Method::GET, uri, None, Some(&token)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(body, json!({ "error": "not found" }));
    }
}

#[sqlx::test]
async fn patch_changes_only_given_fields(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;
    let todo = create_todo(
        &app,
        &token,
        json!({ "title": "x", "description": "keep me" }),
    )
    .await;

    let (status, updated) = send(
        &app,
        Method::PATCH,
        &todo_uri(&todo),
        Some(json!({ "completed": true })),
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["completed"], true);
    assert_eq!(updated["title"], "x");
    assert_eq!(updated["description"], "keep me");
    assert_eq!(updated["created_at"], todo["created_at"]);
    assert_ne!(updated["updated_at"], todo["updated_at"]);
}

#[sqlx::test]
async fn patch_with_null_description_clears_it(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;
    let todo = create_todo(
        &app,
        &token,
        json!({ "title": "x", "description": "remove me" }),
    )
    .await;

    let (status, updated) = send(
        &app,
        Method::PATCH,
        &todo_uri(&todo),
        Some(json!({ "title": "renamed", "description": null })),
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["title"], "renamed");
    assert_eq!(updated["description"], Value::Null);
}

#[sqlx::test]
async fn patch_with_invalid_body_is_unprocessable(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;
    let todo = create_todo(&app, &token, json!({ "title": "x" })).await;

    for body in [
        json!({}),
        json!({ "title": "" }),
        json!({ "complete": true }),
    ] {
        let (status, _) = send(
            &app,
            Method::PATCH,
            &todo_uri(&todo),
            Some(body.clone()),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "body: {body}");
    }
}

#[sqlx::test]
async fn delete_removes_the_todo(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;
    let todo = create_todo(&app, &token, json!({ "title": "x" })).await;
    let uri = todo_uri(&todo);

    let (status, body) = send(&app, Method::DELETE, &uri, None, Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(body, Value::Null);

    let (status, _) = send(&app, Method::GET, &uri, None, Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = send(&app, Method::DELETE, &uri, None, Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn list_is_paginated_with_limit_and_offset(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;
    for title in ["t1", "t2", "t3", "t4"] {
        create_todo(&app, &token, json!({ "title": title })).await;
    }

    let (status, body) = send(
        &app,
        Method::GET,
        "/todos?limit=2&offset=1",
        None,
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let titles: Vec<_> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["title"].clone())
        .collect();
    assert_eq!(titles, [json!("t2"), json!("t3")]);
}

#[sqlx::test]
async fn invalid_pagination_is_unprocessable(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;

    for query in ["limit=0", "limit=101", "limit=abc", "offset=-1", "page=2"] {
        let uri = format!("/todos?{query}");
        let (status, body) = send(&app, Method::GET, &uri, None, Some(&token)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{query}");
        assert!(body["error"].is_string(), "{query}");
    }
}

#[sqlx::test]
async fn oversized_body_is_rejected_with_413(pool: PgPool) {
    let app = common::app(pool);
    let token = register_and_login(&app, "alice").await;
    let huge = json!({ "title": "x", "description": "d".repeat(100 * 1024) });

    let (status, body) = send(&app, Method::POST, "/todos", Some(huge), Some(&token)).await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body, json!({ "error": "request body too large" }));
}
