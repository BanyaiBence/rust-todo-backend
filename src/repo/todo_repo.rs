use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::todo::{NewTodo, Todo, TodoChanges},
};

#[tracing::instrument(skip(pool, new))]
pub async fn create(pool: &PgPool, user_id: Uuid, new: NewTodo) -> Result<Todo, AppError> {
    let todo = sqlx::query_as!(
        Todo,
        r#"
        INSERT INTO todos (id, user_id, title, description)
        VALUES ($1, $2, $3, $4)
        RETURNING id, user_id, title, description, completed, created_at, updated_at
        "#,
        Uuid::new_v4(),
        user_id,
        new.title,
        new.description,
    )
    .fetch_one(pool)
    .await?;
    Ok(todo)
}

/// All todos owned by `user_id`, oldest first.
#[tracing::instrument(skip(pool))]
pub async fn list(pool: &PgPool, user_id: Uuid) -> Result<Vec<Todo>, AppError> {
    let todos = sqlx::query_as!(
        Todo,
        r#"
        SELECT id, user_id, title, description, completed, created_at, updated_at
        FROM todos
        WHERE user_id = $1
        ORDER BY created_at, id
        "#,
        user_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(todos)
}

#[tracing::instrument(skip(pool))]
pub async fn get(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<Option<Todo>, AppError> {
    let todo = sqlx::query_as!(
        Todo,
        r#"
        SELECT id, user_id, title, description, completed, created_at, updated_at
        FROM todos
        WHERE id = $1 AND user_id = $2
        "#,
        id,
        user_id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(todo)
}

/// Returns `None` if the todo does not exist or belongs to another user.
#[tracing::instrument(skip(pool, changes))]
pub async fn update(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    changes: TodoChanges,
) -> Result<Option<Todo>, AppError> {
    let (set_description, description) = match changes.description {
        Some(description) => (true, description),
        None => (false, None),
    };

    let todo = sqlx::query_as!(
        Todo,
        r#"
        UPDATE todos SET
            title       = COALESCE($3, title),
            description = CASE WHEN $4 THEN $5 ELSE description END,
            completed   = COALESCE($6, completed),
            updated_at  = now()
        WHERE id = $1 AND user_id = $2
        RETURNING id, user_id, title, description, completed, created_at, updated_at
        "#,
        id,
        user_id,
        changes.title,
        set_description,
        description,
        changes.completed,
    )
    .fetch_optional(pool)
    .await?;
    Ok(todo)
}

/// Returns `false` if the todo does not exist or belongs to another user.
#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<bool, AppError> {
    let result = sqlx::query!(
        "DELETE FROM todos WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::user_repo;

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

        let titles: Vec<_> = list(&pool, alice)
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

        assert!(list(&pool, alice).await.unwrap().is_empty());
    }
}
