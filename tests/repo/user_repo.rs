use sqlx::PgPool;
use todo_backend::{
    error::AppError,
    repo::user_repo::{create, find_by_username},
};

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
