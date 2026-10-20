// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

/// Errors returned by the core services. The HTTP layer maps each variant
/// to a status code; `Internal` details are logged, never sent to clients.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("not authenticated")]
    Unauthorized,
    /// The caller may know the object exists but lacks the needed role.
    #[error("forbidden")]
    Forbidden,
    /// Also used when the caller has no access at all (does not leak existence).
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error("quota exceeded")]
    QuotaExceeded,
    #[error("payload too large")]
    TooLarge,
    #[error("too many requests")]
    TooManyRequests,
    /// A resumable upload chunk arrived at the wrong offset.
    #[error("offset mismatch")]
    OffsetMismatch { current: u64 },
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;

impl From<sqlx::Error> for CoreError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => CoreError::NotFound,
            other => CoreError::Internal(other.into()),
        }
    }
}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::NotFound {
            CoreError::NotFound
        } else {
            CoreError::Internal(e.into())
        }
    }
}

impl CoreError {
    pub fn bad(msg: impl Into<String>) -> Self {
        CoreError::BadRequest(msg.into())
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        CoreError::Conflict(msg.into())
    }
}
