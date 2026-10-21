// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use drivich_core::models::{Node, UserBrief};
use drivich_core::nodes::{self, BatchResult, ListParams, Page, SortBy, View};
use drivich_core::search::{self, SearchParams, Suggestion};
use drivich_core::{CoreError, uploads, users};
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/nodes", get(list))
        .route("/nodes/folder", post(create_folder))
        .route("/nodes/file", post(create_file))
        .route("/nodes/batch", post(batch))
        .route(
            "/nodes/{id}",
            get(get_node).patch(update).delete(delete_forever),
        )
        .route("/nodes/{id}/path", get(path))
        .route("/nodes/{id}/trash", post(trash))
        .route("/nodes/{id}/restore", post(restore))
        .route("/trash/empty", post(empty_trash))
        .route("/search", get(search_all))
        .route("/search/suggest", get(suggest))
        .route("/users/lookup", get(lookup_users))
}

#[derive(Deserialize)]
struct ListQuery {
    view: Option<String>,
    parent: Option<String>,
    sort: Option<String>,
    order: Option<String>,
    #[serde(rename = "type")]
    type_filter: Option<String>,
    modified_from: Option<DateTime<Utc>>,
    owner: Option<String>,
    location: Option<String>,
    limit: Option<i64>,
    cursor: Option<String>,
}

fn parse_parent(raw: Option<&str>) -> Result<Option<Uuid>, CoreError> {
    match raw {
        None | Some("") | Some("root") => Ok(None),
        Some(s) => Uuid::parse_str(s)
            .map(Some)
            .map_err(|_| CoreError::bad("invalid parent id")),
    }
}

async fn list(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Page>> {
    let view = match q.view.as_deref() {
        None => View::Drive,
        Some(v) => View::parse(v).ok_or_else(|| CoreError::bad("unknown view"))?,
    };
    let sort = match q.sort.as_deref() {
        None => SortBy::Name,
        Some(s) => SortBy::parse(s).ok_or_else(|| CoreError::bad("unknown sort"))?,
    };
    let desc = match q.order.as_deref() {
        None | Some("asc") => false,
        Some("desc") => true,
        Some(_) => return Err(CoreError::bad("order must be asc or desc").into()),
    };
    let params = ListParams {
        view,
        parent: parse_parent(q.parent.as_deref())?,
        sort,
        desc,
        type_filter: q.type_filter.filter(|t| !t.is_empty() && t != "any"),
        modified_from: q.modified_from,
        owner: q.owner,
        location: q.location,
        limit: q.limit.unwrap_or(nodes::DEFAULT_PAGE),
        cursor: q.cursor,
    };
    Ok(Json(nodes::list(&st.pool, u.id, params).await?))
}

async fn get_node(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Node>> {
    Ok(Json(nodes::get(&st.pool, u.id, id).await?))
}

async fn path(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let items: Vec<Value> = nodes::breadcrumb(&st.pool, u.id, id)
        .await?
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();
    Ok(Json(json!({ "items": items })))
}

#[derive(Deserialize)]
struct NewFolder {
    parent_id: Option<Uuid>,
    name: String,
}

async fn create_folder(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Json(b): Json<NewFolder>,
) -> ApiResult<Json<Node>> {
    Ok(Json(
        nodes::create_folder(&st.pool, u.id, b.parent_id, &b.name).await?,
    ))
}

#[derive(Deserialize)]
struct NewBlank {
    parent_id: Option<Uuid>,
    /// docx, xlsx, pptx, md or txt.
    kind: String,
    name: String,
}

async fn create_file(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Json(b): Json<NewBlank>,
) -> ApiResult<Json<Node>> {
    Ok(Json(
        uploads::create_blank(
            &st.pool,
            &st.storage,
            &st.limits,
            u.id,
            b.parent_id,
            &b.kind,
            &b.name,
        )
        .await?,
    ))
}

/// Distinguishes "field absent" (None) from "explicit null" (Some(None)).
fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(d)?))
}

#[derive(Deserialize)]
struct Patch {
    name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    parent_id: Option<Option<Uuid>>,
    starred: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    description: Option<Option<String>>,
}

async fn update(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
    Json(b): Json<Patch>,
) -> ApiResult<Json<Node>> {
    let mut last = None;
    if let Some(name) = &b.name {
        last = Some(nodes::rename(&st.pool, u.id, id, name).await?);
    }
    if let Some(parent) = b.parent_id {
        last = Some(nodes::move_to(&st.pool, u.id, id, parent).await?);
    }
    if let Some(starred) = b.starred {
        last = Some(nodes::set_starred(&st.pool, u.id, id, starred).await?);
    }
    if let Some(desc) = &b.description {
        last = Some(nodes::set_description(&st.pool, u.id, id, desc.as_deref()).await?);
    }
    match last {
        Some(n) => Ok(Json(n)),
        None => Ok(Json(nodes::get(&st.pool, u.id, id).await?)),
    }
}

async fn trash(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    nodes::trash(&st.pool, u.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn restore(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Node>> {
    Ok(Json(nodes::restore(&st.pool, u.id, id).await?))
}

async fn delete_forever(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    nodes::delete_permanently(&st.pool, &st.storage, u.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn empty_trash(State(st): State<AppState>, AuthUser(u): AuthUser) -> ApiResult<Json<Value>> {
    let n = nodes::empty_trash(&st.pool, &st.storage, u.id).await?;
    Ok(Json(json!({ "deleted": n })))
}

#[derive(Deserialize)]
struct Batch {
    ids: Vec<Uuid>,
    action: String,
    parent_id: Option<Uuid>,
}

/// Multi-selection actions: trash, restore, star, unstar, move, delete (forever).
async fn batch(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Json(b): Json<Batch>,
) -> ApiResult<Json<BatchResult>> {
    if b.ids.is_empty() || b.ids.len() > 1000 {
        return Err(CoreError::bad("between 1 and 1000 items expected").into());
    }
    let mut res = BatchResult {
        ok: vec![],
        failed: vec![],
    };
    for id in b.ids {
        let r: Result<(), CoreError> = match b.action.as_str() {
            "trash" => nodes::trash(&st.pool, u.id, id).await,
            "restore" => nodes::restore(&st.pool, u.id, id).await.map(|_| ()),
            "star" => nodes::set_starred(&st.pool, u.id, id, true)
                .await
                .map(|_| ()),
            "unstar" => nodes::set_starred(&st.pool, u.id, id, false)
                .await
                .map(|_| ()),
            "move" => nodes::move_to(&st.pool, u.id, id, b.parent_id)
                .await
                .map(|_| ()),
            "delete" => nodes::delete_permanently(&st.pool, &st.storage, u.id, id).await,
            _ => return Err(CoreError::bad("unknown action").into()),
        };
        match r {
            Ok(()) => res.ok.push(id),
            Err(CoreError::Internal(e)) => return Err(CoreError::Internal(e).into()),
            Err(e) => res.failed.push((id, e.to_string())),
        }
    }
    Ok(Json(res))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    name: Option<String>,
    words: Option<String>,
    #[serde(rename = "type")]
    type_filter: Option<String>,
    owner: Option<String>,
    modified_from: Option<DateTime<Utc>>,
    modified_to: Option<DateTime<Utc>>,
    location: Option<String>,
    in_folder: Option<Uuid>,
    limit: Option<i64>,
    cursor: Option<String>,
}

async fn search_all(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<Page>> {
    let offset = match q.cursor.as_deref() {
        None => 0,
        Some(c) => c
            .parse::<i64>()
            .ok()
            .filter(|n| *n >= 0)
            .ok_or_else(|| CoreError::bad("invalid cursor"))?,
    };
    let p = SearchParams {
        q: q.q,
        name: q.name,
        words: q.words,
        type_filter: q.type_filter,
        owner: q.owner,
        modified_from: q.modified_from,
        modified_to: q.modified_to,
        location: q.location.filter(|l| !l.is_empty()),
        in_folder: q.in_folder,
        limit: q.limit.unwrap_or(50),
        offset,
    };
    Ok(Json(search::search(&st.pool, u.id, p).await?))
}

#[derive(Deserialize)]
struct SuggestQuery {
    q: String,
}

async fn suggest(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Query(q): Query<SuggestQuery>,
) -> ApiResult<Json<Value>> {
    let items: Vec<Suggestion> = search::suggest(&st.pool, u.id, &q.q).await?;
    Ok(Json(json!({ "items": items })))
}

#[derive(Deserialize)]
struct LookupQuery {
    q: String,
}

async fn lookup_users(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Query(q): Query<LookupQuery>,
) -> ApiResult<Json<Value>> {
    let items: Vec<UserBrief> = users::lookup(&st.pool, &q.q, u.id).await?;
    Ok(Json(json!({ "items": items })))
}
