// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::routes::files::{self, Mode};
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use drivich_core::models::LinkEntry;
use drivich_core::nodes::{self, Page};
use drivich_core::shares::{self, ShareInfo};
use drivich_core::{CoreError, perms};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/nodes/{id}/shares", get(list).post(add))
        .route("/shares/{id}", patch(update).delete(remove))
        .route("/nodes/{id}/links", post(create_link))
        .route("/links/{id}", delete(revoke_link))
        // Public (no account needed): the token is the credential.
        .route("/public/{token}", get(public_root))
        .route("/public/{token}/nodes/{id}", get(public_node))
        .route("/public/{token}/nodes/{id}/children", get(public_children))
        .route("/public/{token}/nodes/{id}/download", get(public_download))
        .route("/public/{token}/nodes/{id}/content", get(public_content))
        .route("/public/{token}/nodes/{id}/zip", get(public_zip))
        .route(
            "/public/{token}/nodes/{id}/thumbnail",
            get(public_thumbnail),
        )
}

async fn list(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ShareInfo>> {
    Ok(Json(shares::list(&st.pool, u.id, id).await?))
}

#[derive(Deserialize)]
struct AddShare {
    email: String,
    role: String,
}

async fn add(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
    Json(b): Json<AddShare>,
) -> ApiResult<Json<Value>> {
    let entry = shares::add_user(&st.pool, u.id, id, &b.email, &b.role).await?;
    Ok(Json(json!({ "share": entry })))
}

#[derive(Deserialize)]
struct UpdateShare {
    role: String,
}

async fn update(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
    Json(b): Json<UpdateShare>,
) -> ApiResult<Json<Value>> {
    shares::update_role(&st.pool, u.id, id, &b.role).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn remove(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    shares::remove(&st.pool, u.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize, Default)]
struct NewLink {
    expires_at: Option<DateTime<Utc>>,
}

/// The raw token is returned exactly once, in `token`; only its hash is stored.
async fn create_link(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
    body: Option<Json<NewLink>>,
) -> ApiResult<Json<Value>> {
    let expires = body.and_then(|Json(b)| b.expires_at);
    let (link, token): (LinkEntry, String) =
        shares::create_link(&st.pool, u.id, id, expires).await?;
    Ok(Json(
        json!({ "link": link, "token": token, "path": format!("/s/{token}") }),
    ))
}

async fn revoke_link(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    shares::revoke_link(&st.pool, u.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

// ---- public links -----------------------------------------------------------

/// A node reachable through the link: the shared root or something inside it.
pub(crate) async fn scoped(
    st: &AppState,
    token: &str,
    id: Uuid,
) -> Result<perms::NodeRef, CoreError> {
    let root = shares::resolve_link(&st.pool, token).await?;
    let node = perms::node_ref(&st.pool, id).await?;
    if node.trashed_at.is_some() || !perms::is_within(&st.pool, node.id, root.id).await? {
        return Err(CoreError::NotFound);
    }
    Ok(node)
}

async fn public_root(
    State(st): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<Value>> {
    let root = shares::resolve_link(&st.pool, &token).await?;
    let node = nodes::get_public(&st.pool, root.id).await?;
    Ok(Json(json!({ "root": node })))
}

async fn public_node(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Value>> {
    let n = scoped(&st, &token, id).await?;
    Ok(Json(
        json!({ "node": nodes::get_public(&st.pool, n.id).await? }),
    ))
}

#[derive(Deserialize)]
struct PageQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

async fn public_children(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, Uuid)>,
    Query(q): Query<PageQuery>,
) -> ApiResult<Json<Page>> {
    let n = scoped(&st, &token, id).await?;
    if n.kind != "folder" {
        return Err(CoreError::bad("not a folder").into());
    }
    Ok(Json(
        nodes::list_public_children(&st.pool, n.id, q.limit.unwrap_or(100), q.cursor.as_deref())
            .await?,
    ))
}

async fn public_download(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let n = scoped(&st, &token, id).await?;
    files::serve_file(&st, &n, &headers, Mode::Download).await
}

async fn public_content(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let n = scoped(&st, &token, id).await?;
    files::serve_file(&st, &n, &headers, Mode::Inline).await
}

async fn public_thumbnail(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let n = scoped(&st, &token, id).await?;
    files::serve_thumbnail(&st, &n, &headers).await
}

async fn public_zip(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, Uuid)>,
) -> ApiResult<Response> {
    let n = scoped(&st, &token, id).await?;
    if n.kind != "folder" {
        return Err(CoreError::bad("only folders can be downloaded as a .zip").into());
    }
    let name = format!("{}.zip", drivich_core::archive::safe_component(&n.name));
    crate::routes::archive::stream_zip(&st, vec![n], &name).await
}
