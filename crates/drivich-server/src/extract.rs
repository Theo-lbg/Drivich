// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{FromRequestParts, Request};
use axum::http::{HeaderMap, Method, StatusCode, request::Parts};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::cookie::CookieJar;
use drivich_core::CoreError;
use drivich_core::auth;
use drivich_core::models::User;

pub const COOKIE: &str = "drivich_session";

/// The authenticated user, resolved from the session cookie.
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(COOKIE)
            .map(|c| c.value().to_string())
            .ok_or(CoreError::Unauthorized)?;
        let user = auth::user_for_token(&state.pool, &token).await?;
        Ok(AuthUser(user))
    }
}

/// An authenticated administrator.
pub struct AdminUser(pub User);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let AuthUser(u) = AuthUser::from_request_parts(parts, state).await?;
        if !u.is_admin {
            return Err(CoreError::Forbidden.into());
        }
        Ok(AdminUser(u))
    }
}

/// Best-effort client address (only trusts proxy headers when configured).
pub fn client_ip(headers: &HeaderMap, trust_proxy: bool) -> String {
    if trust_proxy {
        if let Some(v) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
            return v.trim().to_string();
        }
        if let Some(v) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            if let Some(first) = v.split(',').next() {
                return first.trim().to_string();
            }
        }
    }
    "direct".to_string()
}

/// CSRF defence for cookie authentication: every state-changing request must
/// carry a custom header. Browsers cannot add one cross-site without a CORS
/// preflight, which this server never grants.
pub async fn require_csrf_header(req: Request, next: Next) -> Response {
    let safe = matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    if !safe && !req.headers().contains_key("x-drivich-csrf") {
        let body = serde_json::json!({ "error": { "code": "csrf", "message": "Missing X-Drivich-CSRF header" } });
        return (StatusCode::FORBIDDEN, axum::Json(body)).into_response();
    }
    next.run(req).await
}

/// Baseline security headers on every response (handlers may override).
pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    for (k, v) in [
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "same-origin"),
        ("x-frame-options", "SAMEORIGIN"),
        ("cross-origin-resource-policy", "same-origin"),
    ] {
        if !h.contains_key(k) {
            h.insert(k, axum::http::HeaderValue::from_static(v));
        }
    }
    res
}
