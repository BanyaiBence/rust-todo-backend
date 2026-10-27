use todo_backend::{
    auth::password::{hash, verify},
    error::AppError,
};

#[tokio::test]
async fn hash_is_not_the_plain_password() {
    let hashed = hash("correct horse".into()).await.unwrap();

    assert_ne!(hashed, "correct horse");
    assert!(hashed.starts_with("$argon2id$"));
}

#[tokio::test]
async fn same_password_hashes_differently_because_of_salt() {
    let a = hash("correct horse".into()).await.unwrap();
    let b = hash("correct horse".into()).await.unwrap();

    assert_ne!(a, b);
}

#[tokio::test]
async fn verify_accepts_correct_password() {
    let hashed = hash("correct horse".into()).await.unwrap();

    assert!(verify("correct horse".into(), hashed).await.unwrap());
}

#[tokio::test]
async fn verify_rejects_wrong_password() {
    let hashed = hash("correct horse".into()).await.unwrap();

    assert!(!verify("battery staple".into(), hashed).await.unwrap());
}

#[tokio::test]
async fn verify_with_malformed_hash_is_an_internal_error() {
    let err = verify("x".into(), "not-a-hash".into()).await.unwrap_err();

    assert!(matches!(err, AppError::Internal(_)), "got {err:?}");
}
