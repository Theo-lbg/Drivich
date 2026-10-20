// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Comments on files and folders. Rights are decided in `drivich_core::comments`.

use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::state::AppState;
use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::routing::{get, patch};
use drivich_core::comments;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/nodes/{id}/comments", get(list).post(add))
        .route("/comments/{id}", patch(edit).delete(remove))
}

#[derive(Deserialize)]
struct Body {
    body: String,
}

async fn list(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    Ok(Json(
        json!({ "items": comments::list(&st.pool, u.id, id).await? }),
    ))
}

async fn add(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
    Json(b): Json<Body>,
) -> ApiResult<Json<Value>> {
    Ok(Json(
        json!({ "comment": comments::add(&st.pool, u.id, id, &b.body).await? }),
    ))
}

async fn edit(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
    Json(b): Json<Body>,
) -> ApiResult<Json<Value>> {
    Ok(Json(
        json!({ "comment": comments::edit(&st.pool, u.id, id, &b.body).await? }),
    ))
}

async fn remove(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    comments::delete(&st.pool, u.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}
