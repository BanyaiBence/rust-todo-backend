use axum::{Json, extract::State, http::StatusCode};

use crate::{
    auth::{jwt, password},
    error::AppError,
    extract::AppJson,
    models::user::{Credentials, TokenResponse, UserResponse},
    repo::user_repo,
    state::AppState,
};

pub async fn register(
    State(state): State<AppState>,
    AppJson(credentials): AppJson<Credentials>,
) -> Result<(StatusCode, Json<UserResponse>), AppError> {
    credentials.validate_for_registration()?;

    let password_hash = password::hash(credentials.password).await?;
    let user = user_repo::create(&state.pool, &credentials.username, &password_hash).await?;

    tracing::info!(user_id = %user.id, "user registered");
    Ok((StatusCode::CREATED, Json(user.into())))
}

pub async fn login(
    State(state): State<AppState>,
    AppJson(credentials): AppJson<Credentials>,
) -> Result<Json<TokenResponse>, AppError> {
    let user = user_repo::find_by_username(&state.pool, &credentials.username).await?;

    // Always run a full verification so unknown usernames cannot be detected by timing.
    let (user_id, password_hash) = match user {
        Some(user) => (Some(user.id), user.password_hash),
        None => (None, password::dummy_hash()),
    };
    let password_ok = password::verify(credentials.password, password_hash).await?;

    let user_id = match (password_ok, user_id) {
        (true, Some(user_id)) => user_id,
        _ => return Err(AppError::Unauthorized),
    };

    let ttl = state.config.jwt_ttl_seconds;
    let token = jwt::issue(user_id, &state.config.jwt_secret, ttl)?;

    tracing::info!(%user_id, "user logged in");
    Ok(Json(TokenResponse {
        token,
        token_type: "Bearer",
        expires_in: ttl,
    }))
}
