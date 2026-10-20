// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

use crate::error::ApiResult;
use crate::extract::AdminUser;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::Response;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use drivich_core::users::AdminUpdate;
use drivich_core::{CoreError, auth, backups, jobs, settings, users};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/overview", get(overview))
        .route("/admin/users", get(list_users).post(create_user))
        .route("/admin/users/{id}", patch(update_user))
        .route("/admin/users/{id}/password", post(reset_password))
        .route("/admin/jobs/retry", post(retry_jobs))
        .route(
            "/admin/jobs/config",
            get(get_job_controls).put(put_job_controls),
        )
        .route("/admin/backups", get(list_backups).post(create_backup))
        .route("/admin/backups/{name}/download", get(download_backup))
        .route("/admin/settings", get(get_settings).put(put_settings))
}

async fn overview(State(st): State<AppState>, AdminUser(_): AdminUser) -> ApiResult<Json<Value>> {
    let db_ok = sqlx::query("SELECT 1").execute(&st.pool).await.is_ok();
    let db_version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&st.pool)
        .await
        .unwrap_or_default();
    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM users), \
                (SELECT count(*) FROM nodes WHERE kind = 'file'), \
                (SELECT count(*) FROM nodes WHERE kind = 'folder'), \
                (SELECT COALESCE(sum(size), 0)::bigint FROM blobs)",
    )
    .fetch_one(&st.pool)
    .await
    .map_err(CoreError::from)?;
    let disk = st
        .storage
        .disk_space()
        .map(|(free, total)| json!({ "free_bytes": free, "total_bytes": total }));
    let job_stats = jobs::stats(&st.pool).await?;
    let failures = jobs::recent_failures(&st.pool).await?;
    Ok(Json(json!({
        "version": drivich_core::VERSION,
        "database": { "ok": db_ok, "version": db_version },
        "counts": { "users": counts.0, "files": counts.1, "folders": counts.2, "stored_bytes": counts.3 },
        "disk": disk,
        "data_dir": st.storage.root().display().to_string(),
        "limits": { "max_file_bytes": st.cfg.max_file_bytes, "trash_days": st.cfg.trash_days, "upload_ttl_hours": st.cfg.upload_ttl_hours },
        "jobs": { "stats": job_stats, "recent_failures": failures },
    })))
}

async fn list_users(State(st): State<AppState>, AdminUser(_): AdminUser) -> ApiResult<Json<Value>> {
    let list = users::list(&st.pool).await?;
    let mut out = Vec::with_capacity(list.len());
    for u in list {
        let used = users::used_bytes(&st.pool, u.id).await?;
        out.push(json!({ "user": u, "used_bytes": used }));
    }
    Ok(Json(json!({ "items": out })))
}

#[derive(Deserialize)]
struct NewUser {
    email: String,
    name: String,
    password: String,
    #[serde(default)]
    is_admin: bool,
    quota_bytes: Option<i64>,
}

async fn create_user(
    State(st): State<AppState>,
    AdminUser(_): AdminUser,
    Json(b): Json<NewUser>,
) -> ApiResult<Json<Value>> {
    let u = users::create(
        &st.pool,
        &b.email,
        &b.name,
        &b.password,
        b.is_admin,
        b.quota_bytes,
    )
    .await?;
    Ok(Json(json!({ "user": u })))
}

#[derive(Deserialize)]
struct UserPatch {
    is_admin: Option<bool>,
    disabled: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    quota_bytes: Option<Option<i64>>,
}

fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(d)?))
}

async fn update_user(
    State(st): State<AppState>,
    AdminUser(me): AdminUser,
    Path(id): Path<Uuid>,
    Json(b): Json<UserPatch>,
) -> ApiResult<Json<Value>> {
    let u = users::admin_update(
        &st.pool,
        id,
        me.id,
        AdminUpdate {
            is_admin: b.is_admin,
            disabled: b.disabled,
            quota_bytes: b.quota_bytes,
        },
    )
    .await?;
    Ok(Json(json!({ "user": u })))
}

#[derive(Deserialize)]
struct Reset {
    password: String,
}

async fn reset_password(
    State(st): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<Uuid>,
    Json(b): Json<Reset>,
) -> ApiResult<Json<Value>> {
    users::update_password(&st.pool, id, &b.password).await?;
    auth::delete_user_sessions(&st.pool, id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn retry_jobs(State(st): State<AppState>, AdminUser(_): AdminUser) -> ApiResult<Json<Value>> {
    Ok(Json(
        json!({ "requeued": jobs::retry_failed(&st.pool).await? }),
    ))
}

async fn read_settings(st: &AppState) -> ApiResult<Json<Value>> {
    Ok(Json(json!({
        "allow_signup": settings::get_bool(&st.pool, settings::ALLOW_SIGNUP, false).await?,
        "default_quota_bytes": settings::get_i64(&st.pool, settings::DEFAULT_QUOTA).await?,
        "immich_url": settings::effective_immich_url(&st.pool, None).await?,
    })))
}

async fn get_settings(
    State(st): State<AppState>,
    AdminUser(_): AdminUser,
) -> ApiResult<Json<Value>> {
    read_settings(&st).await
}

#[derive(Deserialize)]
struct SettingsBody {
    allow_signup: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    default_quota_bytes: Option<Option<i64>>,
    /// Server-wide Immich address (users can override it with their own).
    #[serde(default, deserialize_with = "double_option")]
    immich_url: Option<Option<String>>,
}

async fn put_settings(
    State(st): State<AppState>,
    AdminUser(_): AdminUser,
    Json(b): Json<SettingsBody>,
) -> ApiResult<Json<Value>> {
    if let Some(v) = b.allow_signup {
        settings::set(&st.pool, settings::ALLOW_SIGNUP, &json!(v)).await?;
    }
    if let Some(q) = b.default_quota_bytes {
        if matches!(q, Some(n) if n < 0) {
            return Err(CoreError::bad("quota must be positive").into());
        }
        settings::set(&st.pool, settings::DEFAULT_QUOTA, &json!(q)).await?;
    }
    if let Some(url) = b.immich_url {
        let value = match url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
            Some(u) => Some(users::normalize_external_url(u)?),
            None => None,
        };
        settings::set(&st.pool, settings::IMMICH_URL, &json!(value)).await?;
    }
    read_settings(&st).await
}

// ---- database backups ------------------------------------------------------------

async fn list_backups(
    State(st): State<AppState>,
    AdminUser(_): AdminUser,
) -> ApiResult<Json<Value>> {
    let items = backups::list(&st.storage).await?;
    Ok(Json(json!({
        "items": items,
        "enabled": st.cfg.backups_enabled,
        "keep": st.cfg.backup_keep,
    })))
}

/// Queue a dump now; the worker (which has `pg_dump`) performs it.
async fn create_backup(
    State(st): State<AppState>,
    AdminUser(_): AdminUser,
) -> ApiResult<Json<Value>> {
    jobs::enqueue(
        &st.pool,
        jobs::kind::DB_DUMP,
        json!({ "manual": true }),
        Some("db_dump:manual"),
    )
    .await?;
    Ok(Json(json!({ "queued": true })))
}

/// A dump contains every user's metadata and password hashes: admin only.
async fn download_backup(
    State(st): State<AppState>,
    AdminUser(_): AdminUser,
    Path(name): Path<String>,
) -> ApiResult<Response> {
    let path = backups::backup_path(&st.storage, &name)?;
    let file = tokio::fs::File::open(&path)
        .await
        .map_err(CoreError::from)?;
    let len = file.metadata().await.map_err(CoreError::from)?.len();
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, len)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{name}\""),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(ReaderStream::with_capacity(
            file,
            256 * 1024,
        )))
        .map_err(|e| CoreError::Internal(e.into()))?)
}

// ---- queue controls (pause, concurrency) ---------------------------------------------

async fn read_job_controls(st: &AppState) -> ApiResult<Json<Value>> {
    let c = jobs::controls(&st.pool).await?;
    Ok(Json(
        json!({ "paused": c.paused, "limits": c.limits, "kinds": jobs::ALL_KINDS }),
    ))
}

async fn get_job_controls(
    State(st): State<AppState>,
    AdminUser(_): AdminUser,
) -> ApiResult<Json<Value>> {
    read_job_controls(&st).await
}

async fn put_job_controls(
    State(st): State<AppState>,
    AdminUser(_): AdminUser,
    Json(b): Json<jobs::JobControls>,
) -> ApiResult<Json<Value>> {
    jobs::set_controls(&st.pool, &b).await?;
    read_job_controls(&st).await
}
