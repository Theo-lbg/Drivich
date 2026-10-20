// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Resumable upload protocol (a small tus-like design).
//!
//! * `POST   /uploads`        `{parent_id, name, size}` -> session, or
//!   `{replace_node_id, size}` to upload a new version of an existing file
//! * `HEAD   /uploads/{id}`   `Upload-Offset` header = bytes already stored
//! * `PATCH  /uploads/{id}`   header `Upload-Offset: N`, body = raw bytes
//! * `DELETE /uploads/{id}`   cancel
//!
//! The last PATCH (when all bytes have arrived) answers with the new file.

use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::state::AppState;
use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::DefaultBodyLimit;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{head, post};
use drivich_core::{CoreError, uploads};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new().route("/uploads", post(create)).route(
        "/uploads/{id}",
        head(status)
            .get(status_json)
            .patch(append)
            .delete(cancel)
            .layer(DefaultBodyLimit::disable()),
    )
}

#[derive(Deserialize)]
struct NewUpload {
    parent_id: Option<Uuid>,
    /// Required unless `replace_node_id` is set.
    name: Option<String>,
    /// Upload a new version of this file instead of a new file.
    replace_node_id: Option<Uuid>,
    size: i64,
}

async fn create(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Json(b): Json<NewUpload>,
) -> ApiResult<Json<Value>> {
    let info = match (b.replace_node_id, b.name) {
        (Some(node), _) => {
            uploads::create_version(&st.pool, &st.storage, &st.limits, u.id, node, b.size).await?
        }
        (None, Some(name)) => {
            uploads::create(
                &st.pool,
                &st.storage,
                &st.limits,
                u.id,
                b.parent_id,
                &name,
                b.size,
            )
            .await?
        }
        (None, None) => return Err(CoreError::bad("name is required").into()),
    };
    Ok(Json(json!({ "upload": info })))
}

fn offset_headers(info: &uploads::UploadInfo) -> HeaderMap {
    let mut h = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(&info.received.to_string()) {
        h.insert("upload-offset", v);
    }
    if let Ok(v) = HeaderValue::from_str(&info.total_size.to_string()) {
        h.insert("upload-length", v);
    }
    h.insert("cache-control", HeaderValue::from_static("no-store"));
    h
}

async fn status(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let info = uploads::status(&st.pool, &st.storage, u.id, id).await?;
    Ok((StatusCode::OK, offset_headers(&info)).into_response())
}

async fn status_json(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let info = uploads::status(&st.pool, &st.storage, u.id, id).await?;
    Ok((offset_headers(&info), Json(json!({ "upload": info }))).into_response())
}

async fn append(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    body: Body,
) -> ApiResult<Response> {
    let offset: u64 = headers
        .get("upload-offset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse().ok())
        .ok_or_else(|| CoreError::bad("missing or invalid Upload-Offset header"))?;
    let stream = body
        .into_data_stream()
        .map(|r| r.map_err(std::io::Error::other));
    let out = uploads::append(
        &st.pool,
        &st.storage,
        &st.limits,
        u.id,
        id,
        offset,
        Box::pin(stream),
    )
    .await?;
    let headers = offset_headers(&out.upload);
    Ok((
        headers,
        Json(json!({ "upload": out.upload, "node": out.node })),
    )
        .into_response())
}

async fn cancel(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    uploads::cancel(&st.pool, &st.storage, u.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}
