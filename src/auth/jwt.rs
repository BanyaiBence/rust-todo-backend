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

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret";

    #[test]
    fn issued_token_verifies_to_the_same_user() {
        let user_id = Uuid::new_v4();
        let token = issue(user_id, SECRET, 60).unwrap();

        let claims = verify(&token, SECRET).unwrap();

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.exp - claims.iat, 60);
    }

    #[test]
    fn token_signed_with_another_secret_is_rejected() {
        let token = issue(Uuid::new_v4(), "other-secret", 60).unwrap();

        assert!(matches!(
            verify(&token, SECRET),
            Err(AppError::Unauthorized)
        ));
    }

    #[test]
    fn tampered_token_is_rejected() {
        let token = issue(Uuid::new_v4(), SECRET, 60).unwrap();
        let (rest, signature) = token.rsplit_once('.').unwrap();
        let flipped = if signature.starts_with('A') { 'B' } else { 'A' };
        let tampered = format!("{rest}.{flipped}{}", &signature[1..]);

        assert!(matches!(
            verify(&tampered, SECRET),
            Err(AppError::Unauthorized)
        ));
    }

    #[test]
    fn expired_token_is_rejected() {
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: Uuid::new_v4(),
            iat: now - 7200,
            exp: now - 3600,
        };
        let token = jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap();

        assert!(matches!(
            verify(&token, SECRET),
            Err(AppError::Unauthorized)
        ));
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(matches!(
            verify("not.a.jwt", SECRET),
            Err(AppError::Unauthorized)
        ));
    }
}
