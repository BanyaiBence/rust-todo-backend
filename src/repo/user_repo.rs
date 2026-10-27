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

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn create_returns_the_stored_user(pool: PgPool) {
        let user = create(&pool, "alice", "hash").await.unwrap();

        assert_eq!(user.username, "alice");
        assert_eq!(user.password_hash, "hash");
    }

    #[sqlx::test]
    async fn duplicate_username_is_a_conflict(pool: PgPool) {
        create(&pool, "alice", "hash").await.unwrap();

        let err = create(&pool, "alice", "other").await.unwrap_err();

        assert!(
            matches!(&err, AppError::Conflict(msg) if msg == "username already taken"),
            "got {err:?}"
        );
    }

    #[sqlx::test]
    async fn find_by_username_returns_existing_user(pool: PgPool) {
        let created = create(&pool, "alice", "hash").await.unwrap();

        let found = find_by_username(&pool, "alice").await.unwrap().unwrap();

        assert_eq!(found.id, created.id);
        assert_eq!(found.password_hash, "hash");
    }

    #[sqlx::test]
    async fn find_by_username_returns_none_for_unknown_user(pool: PgPool) {
        assert!(find_by_username(&pool, "nobody").await.unwrap().is_none());
    }
}
