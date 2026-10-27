use std::sync::LazyLock;

use argon2::{
    Argon2,
    password_hash::{Error as HashError, PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

use crate::error::AppError;

/// A real hash of a random value, verified against when a login names an unknown user so
/// that the response takes as long as a wrong password would.
static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
    hash_blocking(uuid::Uuid::new_v4().to_string()).expect("hashing dummy password")
});

pub fn dummy_hash() -> String {
    DUMMY_HASH.clone()
}

/// Hashes a password with Argon2id and a random salt, returning a PHC string.
/// Runs on the blocking thread pool because hashing is deliberately CPU-expensive.
pub async fn hash(password: String) -> Result<String, AppError> {
    run_blocking(move || hash_blocking(password)).await
}

/// Checks `password` against a PHC hash string. A malformed hash is an internal error.
pub async fn verify(password: String, hash: String) -> Result<bool, AppError> {
    run_blocking(move || {
        match Argon2::default().verify_password(password.as_bytes(), hash.as_str()) {
            Ok(()) => Ok(true),
            Err(HashError::PasswordInvalid) => Ok(false),
            Err(err) => Err(AppError::Internal(format!(
                "password verification failed: {err}"
            ))),
        }
    })
    .await
}

fn hash_blocking(password: String) -> Result<String, AppError> {
    let hash: PasswordHash = Argon2::default()
        .hash_password(password.as_bytes())
        .map_err(|err| AppError::Internal(format!("password hashing failed: {err}")))?;
    Ok(hash.to_string())
}

async fn run_blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|err| AppError::Internal(format!("password task failed: {err}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
