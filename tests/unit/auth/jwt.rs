use jsonwebtoken::{EncodingKey, Header};
use todo_backend::{
    auth::jwt::{Claims, issue, verify},
    error::AppError,
};
use uuid::Uuid;

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
