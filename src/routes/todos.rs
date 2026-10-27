use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    extract::{AppJson, AppPath},
    models::todo::{CreateTodo, Todo, UpdateTodo},
    repo::todo_repo,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<Json<Vec<Todo>>, AppError> {
    Ok(Json(todo_repo::list(&state.pool, user_id).await?))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    AppJson(body): AppJson<CreateTodo>,
) -> Result<(StatusCode, Json<Todo>), AppError> {
    let todo = todo_repo::create(&state.pool, user_id, body.validate()?).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}

pub async fn get(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    AppPath(id): AppPath<Uuid>,
) -> Result<Json<Todo>, AppError> {
    todo_repo::get(&state.pool, user_id, id)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

pub async fn update(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    AppPath(id): AppPath<Uuid>,
    AppJson(body): AppJson<UpdateTodo>,
) -> Result<Json<Todo>, AppError> {
    todo_repo::update(&state.pool, user_id, id, body.validate()?)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

pub async fn delete(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    AppPath(id): AppPath<Uuid>,
) -> Result<StatusCode, AppError> {
    if todo_repo::delete(&state.pool, user_id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
