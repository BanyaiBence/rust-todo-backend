use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, models::user::User};

#[tracing::instrument(skip(pool, password_hash))]
pub async fn create(pool: &PgPool, username: &str, password_hash: &str) -> Result<User, AppError> {
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (id, username, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id, username, password_hash, created_at
        "#,
        Uuid::new_v4(),
        username,
        password_hash,
    )
    .fetch_one(pool)
    .await
    .map_err(|err| match AppError::from(err) {
        AppError::Conflict(_) => AppError::Conflict("username already taken".to_string()),
        other => other,
    })
}

#[tracing::instrument(skip(pool))]
pub async fn find_by_username(pool: &PgPool, username: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as!(
        User,
        "SELECT id, username, password_hash, created_at FROM users WHERE username = $1",
        username,
    )
    .fetch_optional(pool)
    .await?;
    Ok(user)
}
