use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::todo::{NewTodo, Page, Todo, TodoChanges},
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

/// One page of the todos owned by `user_id`, oldest first.
#[tracing::instrument(skip(pool))]
pub async fn list(pool: &PgPool, user_id: Uuid, page: Page) -> Result<Vec<Todo>, AppError> {
    let todos = sqlx::query_as!(
        Todo,
        r#"
        SELECT id, user_id, title, description, completed, created_at, updated_at
        FROM todos
        WHERE user_id = $1
        ORDER BY created_at, id
        LIMIT $2 OFFSET $3
        "#,
        user_id,
        page.limit,
        page.offset,
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
