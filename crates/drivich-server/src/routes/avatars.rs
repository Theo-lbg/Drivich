// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Profile pictures: `PUT/DELETE /me/avatar`, `GET /users/{id}/avatar`.

use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::state::AppState;
use axum::body::{Body, to_bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use axum::routing::{get, put};
use axum::{Json, Router};
use drivich_core::{CoreError, avatars, users};
use serde_json::{Value, json};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me/avatar", put(upload).delete(remove))
        .route("/users/{id}/avatar", get(show))
}

/// The body is the raw image (any common format); it is re-encoded server side.
async fn upload(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    body: Body,
) -> ApiResult<Json<Value>> {
    let bytes = to_bytes(body, avatars::MAX_BYTES)
        .await
        .map_err(|_| CoreError::TooLarge)?;
    avatars::set(&st.pool, &st.storage, u.id, bytes.to_vec()).await?;
    let user = users::get(&st.pool, u.id).await?;
    Ok(Json(json!({ "user": user })))
}

async fn remove(State(st): State<AppState>, AuthUser(u): AuthUser) -> ApiResult<Json<Value>> {
    avatars::remove(&st.pool, &st.storage, u.id).await?;
    let user = users::get(&st.pool, u.id).await?;
    Ok(Json(json!({ "user": user })))
}

/// Any signed-in user can see other users' pictures (needed for sharing and owners).
async fn show(
    State(st): State<AppState>,
    AuthUser(_): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let (bytes, version) = avatars::read(&st.pool, &st.storage, id).await?;
    let etag = format!("\"a-{}-{version}\"", id.simple());
    let builder = Response::builder()
        .header(header::ETAG, &etag)
        // The frontend adds ?v=<version>, so the URL changes with the picture.
        .header(header::CACHE_CONTROL, "private, max-age=86400");
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        == Some(etag.as_str())
    {
        return Ok(builder
            .status(StatusCode::NOT_MODIFIED)
            .body(Body::empty())
            .map_err(|e| CoreError::Internal(e.into()))?);
    }
    Ok(builder
        .header(header::CONTENT_TYPE, "image/jpeg")
        .body(Body::from(bytes))
        .map_err(|e| CoreError::Internal(e.into()))?)
}
