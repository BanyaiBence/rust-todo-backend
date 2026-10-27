use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Public view of a user; never includes the password hash.
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl Credentials {
    /// Rules for new accounts: username 3–32 chars of `[A-Za-z0-9_-]`, password 8–128 chars.
    pub fn validate_for_registration(&self) -> Result<(), AppError> {
        let invalid = |msg: &str| Err(AppError::Validation(msg.to_string()));

        if !(3..=32).contains(&self.username.chars().count()) {
            return invalid("username must be 3-32 characters");
        }
        if !self
            .username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return invalid("username may only contain letters, digits, '_' and '-'");
        }
        if !(8..=128).contains(&self.password.chars().count()) {
            return invalid("password must be 8-128 characters");
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
}
