// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! File versions: list, download, restore and delete older contents.
//! New versions are uploaded through `POST /uploads` with `replace_node_id`.

use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::routes::files::{self, Mode};
use crate::state::AppState;
use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, post};
use drivich_core::versions;
use serde_json::{Value, json};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/nodes/{id}/versions", get(list))
        .route(
            "/nodes/{id}/versions/{version}",
            axum::routing::delete(remove),
        )
        .route("/nodes/{id}/versions/{version}/download", get(download))
        .route("/nodes/{id}/versions/{version}/restore", post(restore))
}

async fn list(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let items = versions::list(&st.pool, u.id, id).await?;
    Ok(Json(
        json!({ "items": items, "max_versions": st.cfg.max_versions }),
    ))
}

async fn download(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path((id, version)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let (mut node, blob, mime) = versions::locate(&st.pool, u.id, id, version).await?;
    node.blob_id = Some(blob);
    node.mime = Some(mime);
    files::serve_file(&st, &node, &headers, Mode::Download).await
}

async fn restore(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path((id, version)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Value>> {
    let node = versions::restore(
        &st.pool,
        &st.storage,
        u.id,
        id,
        version,
        st.cfg.max_versions,
    )
    .await?;
    Ok(Json(json!({ "node": node })))
}

async fn remove(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path((id, version)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Value>> {
    versions::delete(&st.pool, &st.storage, u.id, id, version).await?;
    Ok(Json(json!({ "ok": true })))
}
