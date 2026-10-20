// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

use axum::Json;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use drivich_core::CoreError;
use serde_json::json;

/// Wrapper that turns core errors into JSON HTTP responses:
/// `{ "error": { "code": "...", "message": "..." } }`.
#[derive(Debug)]
pub struct ApiError(pub CoreError);

pub type ApiResult<T> = Result<T, ApiError>;

impl<E: Into<CoreError>> From<E> for ApiError {
    fn from(e: E) -> Self {
        ApiError(e.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self.0 {
            CoreError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Authentication required".to_string(),
            ),
            CoreError::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "You do not have permission to do this".to_string(),
            ),
            CoreError::NotFound => (StatusCode::NOT_FOUND, "not_found", "Not found".to_string()),
            CoreError::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m.clone()),
            CoreError::Conflict(m) => (StatusCode::CONFLICT, "conflict", m.clone()),
            CoreError::QuotaExceeded => (
                StatusCode::INSUFFICIENT_STORAGE,
                "quota_exceeded",
                "Storage quota exceeded".to_string(),
            ),
            CoreError::TooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "too_large",
                "File too large".to_string(),
            ),
            CoreError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                "too_many_requests",
                "Too many requests, try again later".to_string(),
            ),
            CoreError::OffsetMismatch { .. } => (
                StatusCode::CONFLICT,
                "offset_mismatch",
                "Upload offset does not match the server".to_string(),
            ),
            CoreError::Internal(e) => {
                tracing::error!(error = ?e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "Internal server error".to_string(),
                )
            }
        };
        let mut res = (
            status,
            Json(json!({ "error": { "code": code, "message": message } })),
        )
            .into_response();
        if let CoreError::OffsetMismatch { current } = self.0 {
            if let Ok(v) = HeaderValue::from_str(&current.to_string()) {
                res.headers_mut().insert("upload-offset", v);
            }
        }
        res
    }
}
