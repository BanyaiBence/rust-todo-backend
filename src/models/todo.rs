use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::error::AppError;

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

/// Body of `POST /todos`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateTodo {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl CreateTodo {
    /// Validates the request and normalizes it (trimmed title) for storage.
    pub fn validate(self) -> Result<NewTodo, AppError> {
        Ok(NewTodo {
            title: validate_title(&self.title)?,
            description: self.description.map(validate_description).transpose()?,
        })
    }
}

/// Body of `PATCH /todos/{id}`. Omitted fields are left unchanged; `"description": null`
/// clears the description.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateTodo {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub description: Option<Option<String>>,
    #[serde(default)]
    pub completed: Option<bool>,
}

impl UpdateTodo {
    /// Validates the request and converts it into repository changes.
    pub fn validate(self) -> Result<TodoChanges, AppError> {
        if self.title.is_none() && self.description.is_none() && self.completed.is_none() {
            return Err(AppError::Validation(
                "at least one of title, description or completed is required".to_string(),
            ));
        }

        Ok(TodoChanges {
            title: self.title.as_deref().map(validate_title).transpose()?,
            description: self
                .description
                .map(|description| description.map(validate_description).transpose())
                .transpose()?,
            completed: self.completed,
        })
    }
}

const MAX_TITLE_CHARS: usize = 200;
const MAX_DESCRIPTION_CHARS: usize = 2000;

/// Returns the trimmed title, which must be 1-200 characters.
fn validate_title(title: &str) -> Result<String, AppError> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > MAX_TITLE_CHARS {
        return Err(AppError::Validation(format!(
            "title must be 1-{MAX_TITLE_CHARS} characters"
        )));
    }
    Ok(title.to_string())
}

fn validate_description(description: String) -> Result<String, AppError> {
    if description.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(AppError::Validation(format!(
            "description must be at most {MAX_DESCRIPTION_CHARS} characters"
        )));
    }
    Ok(description)
}

/// Marks a field as present (even when `null`) so it can be told apart from an omitted one.
fn present<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Query string of `GET /todos`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// A validated page window for list queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub limit: i64,
    pub offset: i64,
}

impl ListParams {
    pub const DEFAULT_LIMIT: u32 = 50;
    pub const MAX_LIMIT: u32 = 100;

    pub fn validate(self) -> Result<Page, AppError> {
        let limit = self.limit.unwrap_or(Self::DEFAULT_LIMIT);
        if !(1..=Self::MAX_LIMIT).contains(&limit) {
            return Err(AppError::Validation(format!(
                "limit must be between 1 and {}",
                Self::MAX_LIMIT
            )));
        }
        Ok(Page {
            limit: limit.into(),
            offset: self.offset.unwrap_or(0).into(),
        })
    }
}
