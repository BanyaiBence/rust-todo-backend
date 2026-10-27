use sqlx::PgPool;
use todo_backend::{
    models::todo::{NewTodo, Page, TodoChanges},
    repo::{
        todo_repo::{create, delete, get, list, update},
        user_repo,
    },
};
use uuid::Uuid;

const FIRST_PAGE: Page = Page {
    limit: 50,
    offset: 0,
};

async fn user(pool: &PgPool, name: &str) -> Uuid {
    user_repo::create(pool, name, "hash").await.unwrap().id
}

fn new_todo(title: &str) -> NewTodo {
    NewTodo {
        title: title.to_string(),
        description: Some(format!("{title} details")),
    }
}

#[sqlx::test]
async fn create_returns_an_incomplete_todo(pool: PgPool) {
    let alice = user(&pool, "alice").await;

    let todo = create(&pool, alice, new_todo("buy milk")).await.unwrap();

    assert_eq!(todo.user_id, alice);
    assert_eq!(todo.title, "buy milk");
    assert_eq!(todo.description.as_deref(), Some("buy milk details"));
    assert!(!todo.completed);
    assert_eq!(todo.created_at, todo.updated_at);
}

#[sqlx::test]
async fn list_returns_only_the_owners_todos_in_creation_order(pool: PgPool) {
    let alice = user(&pool, "alice").await;
    let bob = user(&pool, "bob").await;
    create(&pool, alice, new_todo("first")).await.unwrap();
    create(&pool, bob, new_todo("bobs")).await.unwrap();
    create(&pool, alice, new_todo("second")).await.unwrap();

    let titles: Vec<_> = list(&pool, alice, FIRST_PAGE)
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();

    assert_eq!(titles, ["first", "second"]);
}

#[sqlx::test]
async fn get_returns_own_todo(pool: PgPool) {
    let alice = user(&pool, "alice").await;
    let created = create(&pool, alice, new_todo("x")).await.unwrap();

    let found = get(&pool, alice, created.id).await.unwrap().unwrap();

    assert_eq!(found.id, created.id);
}

#[sqlx::test]
async fn get_hides_other_users_todo(pool: PgPool) {
    let alice = user(&pool, "alice").await;
    let bob = user(&pool, "bob").await;
    let created = create(&pool, alice, new_todo("x")).await.unwrap();

    assert!(get(&pool, bob, created.id).await.unwrap().is_none());
}

#[sqlx::test]
async fn get_returns_none_for_unknown_id(pool: PgPool) {
    let alice = user(&pool, "alice").await;

    assert!(get(&pool, alice, Uuid::new_v4()).await.unwrap().is_none());
}

#[sqlx::test]
async fn update_changes_only_given_fields_and_bumps_updated_at(pool: PgPool) {
    let alice = user(&pool, "alice").await;
    let created = create(&pool, alice, new_todo("x")).await.unwrap();

    let updated = update(
        &pool,
        alice,
        created.id,
        TodoChanges {
            completed: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();

    assert!(updated.completed);
    assert_eq!(updated.title, created.title);
    assert_eq!(updated.description, created.description);
    assert_eq!(updated.created_at, created.created_at);
    assert!(updated.updated_at > created.updated_at);
}

#[sqlx::test]
async fn update_can_set_title_and_clear_description(pool: PgPool) {
    let alice = user(&pool, "alice").await;
    let created = create(&pool, alice, new_todo("x")).await.unwrap();

    let updated = update(
        &pool,
        alice,
        created.id,
        TodoChanges {
            title: Some("renamed".into()),
            description: Some(None),
            completed: None,
        },
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(updated.title, "renamed");
    assert_eq!(updated.description, None);
    assert!(!updated.completed);
}

#[sqlx::test]
async fn update_does_not_touch_other_users_todo(pool: PgPool) {
    let alice = user(&pool, "alice").await;
    let bob = user(&pool, "bob").await;
    let created = create(&pool, alice, new_todo("x")).await.unwrap();

    let result = update(
        &pool,
        bob,
        created.id,
        TodoChanges {
            title: Some("hijacked".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert!(result.is_none());
    let unchanged = get(&pool, alice, created.id).await.unwrap().unwrap();
    assert_eq!(unchanged.title, "x");
}

#[sqlx::test]
async fn delete_removes_own_todo(pool: PgPool) {
    let alice = user(&pool, "alice").await;
    let created = create(&pool, alice, new_todo("x")).await.unwrap();

    assert!(delete(&pool, alice, created.id).await.unwrap());
    assert!(get(&pool, alice, created.id).await.unwrap().is_none());
}

#[sqlx::test]
async fn delete_does_not_touch_other_users_todo(pool: PgPool) {
    let alice = user(&pool, "alice").await;
    let bob = user(&pool, "bob").await;
    let created = create(&pool, alice, new_todo("x")).await.unwrap();

    assert!(!delete(&pool, bob, created.id).await.unwrap());
    assert!(get(&pool, alice, created.id).await.unwrap().is_some());
}

#[sqlx::test]
async fn deleting_a_user_cascades_to_their_todos(pool: PgPool) {
    let alice = user(&pool, "alice").await;
    create(&pool, alice, new_todo("x")).await.unwrap();

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(alice)
        .execute(&pool)
        .await
        .unwrap();

    assert!(list(&pool, alice, FIRST_PAGE).await.unwrap().is_empty());
}

#[sqlx::test]
async fn list_applies_limit_and_offset(pool: PgPool) {
    let alice = user(&pool, "alice").await;
    for title in ["t1", "t2", "t3", "t4", "t5"] {
        create(&pool, alice, new_todo(title)).await.unwrap();
    }

    let page = Page {
        limit: 2,
        offset: 1,
    };
    let titles: Vec<_> = list(&pool, alice, page)
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();

    assert_eq!(titles, ["t2", "t3"]);
}
