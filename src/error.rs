use std::fmt;

/// Application-wide error type.
///
/// Repositories only return storage errors through this type; validation and
/// conflict errors are produced by services. The UI never panics on these: it
/// displays [`AppError::user_message`] and keeps running.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("migration failed: {0}")]
    Migration(String),

    #[error("integrity check failed: {0}")]
    Integrity(String),

    #[error("update error: {0}")]
    Update(String),

    #[error("network error: {0}")]
    Network(String),
}

impl AppError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn update(message: impl Into<String>) -> Self {
        Self::Update(message.into())
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::Network(message.into())
    }

    /// A message suitable for showing directly in the UI.
    pub fn user_message(&self) -> String {
        match self {
            Self::Database(err) => match err {
                rusqlite::Error::SqliteFailure(_e, Some(msg)) => {
                    format!("Database error: {msg}")
                }
                _ => format!("Database error: {err}"),
            },
            Self::Validation(message)
            | Self::NotFound(message)
            | Self::Conflict(message)
            | Self::Migration(message)
            | Self::Integrity(message)
            | Self::Update(message)
            | Self::Network(message) => message.clone(),
            Self::Io(err) => format!("File error: {err}"),
            Self::Serialization(err) => format!("Data format error: {err}"),
        }
    }
}

impl From<ureq::Error> for AppError {
    fn from(value: ureq::Error) -> Self {
        Self::Network(value.to_string())
    }
}

impl From<fmt::Error> for AppError {
    fn from(value: fmt::Error) -> Self {
        Self::Validation(value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
