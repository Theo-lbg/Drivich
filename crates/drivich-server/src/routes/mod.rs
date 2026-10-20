// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

pub mod admin;
pub mod archive;
pub mod auth;
pub mod avatars;
pub mod comments;
pub mod files;
pub mod nodes;
pub mod office;
pub mod shares;
pub mod uploads;
pub mod versions;

use crate::extract::{require_csrf_header, security_headers};
use crate::state::AppState;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router, middleware};
use serde_json::{Value, json};
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/openapi.json", get(openapi_json))
        .merge(auth::routes())
        .merge(avatars::routes())
        .merge(nodes::routes())
        .merge(files::routes())
        .merge(uploads::routes())
        .merge(archive::routes())
        .merge(office::routes())
        .merge(versions::routes())
        .merge(comments::routes())
        .merge(shares::routes())
        .merge(admin::routes())
        .layer(middleware::from_fn(require_csrf_header));

    Router::new()
        .nest("/api/v1", api)
        .layer(middleware::from_fn(security_headers))
        // Public-link tokens live in the URL path: never write them to the logs.
        .layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
            tracing::debug_span!("request", method = %req.method(), path = %redact_path(req.uri().path()))
        }))
        .with_state(state)
}

/// Liveness + database check (used by container healthchecks).
async fn health(State(st): State<AppState>) -> Result<Json<Value>, axum::http::StatusCode> {
    sqlx::query("SELECT 1")
        .execute(&st.pool)
        .await
        .map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(
        json!({ "status": "ok", "version": drivich_core::VERSION }),
    ))
}

/// The machine-readable API description (also committed as docs/openapi.json).
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(crate::openapi::spec())
}

/// `/api/v1/public/<token>/...` -> `/api/v1/public/<redacted>/...`
fn redact_path(path: &str) -> String {
    match path.strip_prefix("/api/v1/public/") {
        Some(rest) => match rest.split_once('/') {
            Some((_, tail)) => format!("/api/v1/public/<redacted>/{tail}"),
            None => "/api/v1/public/<redacted>".to_string(),
        },
        None => path.to_string(),
    }
}

#[cfg(test)]
mod log_tests {
    use super::redact_path;

    #[test]
    fn public_link_tokens_are_not_logged() {
        assert_eq!(
            redact_path("/api/v1/public/SECRET"),
            "/api/v1/public/<redacted>"
        );
        assert_eq!(
            redact_path("/api/v1/public/SECRET/nodes/1/download"),
            "/api/v1/public/<redacted>/nodes/1/download"
        );
        assert_eq!(redact_path("/api/v1/nodes"), "/api/v1/nodes");
    }
}
