use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// The authenticated user's id.
    pub sub: Uuid,
    pub iat: i64,
    pub exp: i64,
}

/// Issues an HS256 token for `user_id` that expires after `ttl_seconds`.
pub fn issue(user_id: Uuid, secret: &str, ttl_seconds: u64) -> Result<String, AppError> {
    let iat = chrono::Utc::now().timestamp();
    let claims = Claims {
        sub: user_id,
        iat,
        exp: iat + ttl_seconds as i64,
    };
    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| AppError::Internal(format!("failed to issue token: {err}")))
}

/// Validates signature and expiry, returning the token's claims.
/// Any invalid token is reported as `Unauthorized`.
pub fn verify(token: &str, secret: &str) -> Result<Claims, AppError> {
    jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .map_err(|err| {
        tracing::debug!(error = %err, "rejected bearer token");
        AppError::Unauthorized
    })
}
