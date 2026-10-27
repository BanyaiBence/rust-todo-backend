use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Todo {
    pub id: Uuid,
    #[serde(skip)]
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewTodo {
    pub title: String,
    pub description: Option<String>,
}

/// Partial update: `None` leaves a field unchanged. `description: Some(None)` clears it.
#[derive(Debug, Clone, Default)]
pub struct TodoChanges {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub completed: Option<bool>,
}
