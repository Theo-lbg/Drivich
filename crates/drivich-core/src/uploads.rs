// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Resumable uploads.
//!
//! 1. `create` reserves a session (checks rights, size limit and quota).
//! 2. `append` receives the file in as many chunks as the client wants. The
//!    length of the `.part` file on disk is the source of truth for the offset,
//!    so an interrupted request never corrupts anything: the client asks
//!    where to resume (`get`) and continues from there.
//! 3. When the last byte arrives the upload is finalised atomically: the
//!    file is renamed into the blob store and the blob and node rows are
//!    inserted in one transaction.
//!
//! Chunks are streamed to disk; no file is ever held in memory.

use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::jobs;
use crate::models::{Node, Role};
use crate::nodes::{self, clean_name};
use crate::perms;
use crate::settings;
use crate::storage::{Storage, detect_mime, is_text_like, thumbnailable};
use bytes::Bytes;
use chrono::{DateTime, Duration, Utc};
use futures_util::{Stream, StreamExt};
use serde::Serialize;
use sqlx::FromRow;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

const MAX_PENDING_PER_USER: i64 = 200;

#[derive(Debug, Clone)]
pub struct Limits {
    pub max_file_bytes: u64,
    pub ttl_hours: i64,
    /// Older versions kept per file.
    pub max_versions: i64,
}

#[derive(Debug, Clone, FromRow, Serialize, utoipa::ToSchema)]
pub struct UploadInfo {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub total_size: i64,
    pub received: i64,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AppendOutcome {
    pub upload: UploadInfo,
    /// Set when the last byte arrived and the file now exists.
    pub node: Option<Node>,
}

const INFO_COLS: &str = "id, name, parent_id, total_size, received, expires_at";

/// Quota that applies to `user`: their own, or the instance default, or none.
pub async fn effective_quota(pool: &Pool, user: Uuid) -> Result<Option<i64>> {
    let own: Option<i64> = sqlx::query_scalar("SELECT quota_bytes FROM users WHERE id = $1")
        .bind(user)
        .fetch_one(pool)
        .await?;
    match own {
        Some(q) => Ok(Some(q)),
        None => settings::get_i64(pool, settings::DEFAULT_QUOTA).await,
    }
}

pub async fn create(
    pool: &Pool,
    storage: &Storage,
    limits: &Limits,
    user: Uuid,
    parent: Option<Uuid>,
    name: &str,
    total_size: i64,
) -> Result<UploadInfo> {
    let name = clean_name(name)?;
    check_size(limits, total_size)?;
    let tree_owner = perms::authorize_parent(pool, user, parent).await?;
    check_quota(pool, tree_owner, total_size).await?;
    open_session(pool, storage, limits, user, parent, None, &name, total_size).await
}

/// Start an upload that will become a new version of an existing file.
pub async fn create_version(
    pool: &Pool,
    storage: &Storage,
    limits: &Limits,
    user: Uuid,
    node: Uuid,
    total_size: i64,
) -> Result<UploadInfo> {
    check_size(limits, total_size)?;
    let (n, _) = perms::authorize(pool, user, node, Role::Editor).await?;
    if n.kind != "file" || n.trashed_at.is_some() {
        return Err(CoreError::bad(
            "only a file outside the trash can get a new version",
        ));
    }
    // The previous content is kept as a version, so the whole new size counts.
    check_quota(pool, n.owner_id, total_size).await?;
    open_session(
        pool,
        storage,
        limits,
        user,
        n.parent_id,
        Some(n.id),
        &n.name,
        total_size,
    )
    .await
}

fn check_size(limits: &Limits, total_size: i64) -> Result<()> {
    if total_size < 0 {
        return Err(CoreError::bad("invalid size"));
    }
    if limits.max_file_bytes > 0 && total_size as u64 > limits.max_file_bytes {
        return Err(CoreError::TooLarge);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn open_session(
    pool: &Pool,
    storage: &Storage,
    limits: &Limits,
    user: Uuid,
    parent: Option<Uuid>,
    replace: Option<Uuid>,
    name: &str,
    total_size: i64,
) -> Result<UploadInfo> {
    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM upload_sessions WHERE owner_id = $1")
            .bind(user)
            .fetch_one(pool)
            .await?;
    if pending >= MAX_PENDING_PER_USER {
        return Err(CoreError::TooManyRequests);
    }

    let id = Uuid::new_v4();
    let mime = detect_mime(name, &[]);
    let sql = format!(
        "INSERT INTO upload_sessions (id, owner_id, parent_id, name, mime, total_size, expires_at, replace_node_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {INFO_COLS}"
    );
    let info: UploadInfo = sqlx::query_as(&sql)
        .bind(id)
        .bind(user)
        .bind(parent)
        .bind(name)
        .bind(mime)
        .bind(total_size)
        .bind(Utc::now() + Duration::hours(limits.ttl_hours))
        .bind(replace)
        .fetch_one(pool)
        .await?;
    tokio::fs::File::create(storage.upload_path(id)).await?;
    Ok(info)
}

async fn check_quota(pool: &Pool, tree_owner: Uuid, incoming: i64) -> Result<()> {
    if let Some(quota) = effective_quota(pool, tree_owner).await? {
        let used = crate::users::used_bytes(pool, tree_owner).await?;
        let pending: i64 = sqlx::query_scalar(
            "SELECT COALESCE(sum(total_size), 0)::bigint FROM upload_sessions u \
             WHERE COALESCE((SELECT owner_id FROM nodes WHERE id = u.replace_node_id), (SELECT owner_id FROM nodes WHERE id = u.parent_id), u.owner_id) = $1",
        )
        .bind(tree_owner)
        .fetch_one(pool)
        .await?;
        if used + pending + incoming > quota {
            return Err(CoreError::QuotaExceeded);
        }
    }
    Ok(())
}

#[derive(Debug, FromRow)]
struct Session {
    id: Uuid,
    owner_id: Uuid,
    parent_id: Option<Uuid>,
    replace_node_id: Option<Uuid>,
    name: String,
    total_size: i64,
    expires_at: DateTime<Utc>,
}

async fn load(pool: &Pool, user: Uuid, id: Uuid) -> Result<Session> {
    let s: Session = sqlx::query_as(
        "SELECT id, owner_id, parent_id, replace_node_id, name, total_size, expires_at FROM upload_sessions WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(user)
    .fetch_one(pool)
    .await?;
    if s.expires_at < Utc::now() {
        return Err(CoreError::NotFound);
    }
    Ok(s)
}

async fn part_len(storage: &Storage, id: Uuid) -> Result<u64> {
    Ok(tokio::fs::metadata(storage.upload_path(id)).await?.len())
}

/// Current state of an upload; `received` reflects the bytes really on disk.
pub async fn status(pool: &Pool, storage: &Storage, user: Uuid, id: Uuid) -> Result<UploadInfo> {
    let s = load(pool, user, id).await?;
    let len = part_len(storage, id).await? as i64;
    Ok(UploadInfo {
        id: s.id,
        name: s.name,
        parent_id: s.parent_id,
        total_size: s.total_size,
        received: len,
        expires_at: s.expires_at,
    })
}

/// Append a chunk at `offset`. The offset must equal the bytes already stored.
pub async fn append<S>(
    pool: &Pool,
    storage: &Storage,
    limits: &Limits,
    user: Uuid,
    id: Uuid,
    offset: u64,
    mut body: S,
) -> Result<AppendOutcome>
where
    S: Stream<Item = std::io::Result<Bytes>> + Unpin,
{
    let s = load(pool, user, id).await?;
    let current = part_len(storage, id).await?;
    if offset != current {
        return Err(CoreError::OffsetMismatch { current });
    }
    let total = s.total_size as u64;
    let mut file = tokio::fs::OpenOptions::new()
        .append(true)
        .open(storage.upload_path(id))
        .await?;
    let mut written: u64 = 0;
    let mut failure: Option<CoreError> = None;
    while let Some(chunk) = body.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                // Connection dropped: keep what arrived, the client resumes later.
                failure = Some(CoreError::Internal(anyhow::anyhow!(
                    "upload interrupted: {e}"
                )));
                break;
            }
        };
        if current + written + chunk.len() as u64 > total {
            failure = Some(CoreError::bad("more data received than announced"));
            break;
        }
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
    }
    file.flush().await?;
    drop(file);

    let received = current + written;
    sqlx::query("UPDATE upload_sessions SET received = $2, updated_at = now(), expires_at = $3 WHERE id = $1")
        .bind(id)
        .bind(received as i64)
        .bind(Utc::now() + Duration::hours(limits.ttl_hours))
        .execute(pool)
        .await?;
    if let Some(e) = failure {
        return Err(e);
    }

    let info = UploadInfo {
        id: s.id,
        name: s.name.clone(),
        parent_id: s.parent_id,
        total_size: s.total_size,
        received: received as i64,
        expires_at: s.expires_at,
    };
    if received == total {
        let node = finalize(pool, storage, limits, &s).await?;
        return Ok(AppendOutcome {
            upload: info,
            node: Some(node),
        });
    }
    Ok(AppendOutcome {
        upload: info,
        node: None,
    })
}

const BLANK_DOCX: &[u8] = include_bytes!("../assets/blank.docx");
const BLANK_XLSX: &[u8] = include_bytes!("../assets/blank.xlsx");
const BLANK_PPTX: &[u8] = include_bytes!("../assets/blank.pptx");

/// Starting content of a new document of `kind`, plus the extension to give it.
pub fn blank_content(kind: &str) -> Option<(&'static str, &'static [u8])> {
    match kind {
        "docx" => Some(("docx", BLANK_DOCX)),
        "xlsx" => Some(("xlsx", BLANK_XLSX)),
        "pptx" => Some(("pptx", BLANK_PPTX)),
        "md" => Some(("md", b"")),
        "txt" => Some(("txt", b"")),
        _ => None,
    }
}

/// Create a new file holding the blank template of `kind` (docx, xlsx, pptx, md, txt).
pub async fn create_blank(
    pool: &Pool,
    storage: &Storage,
    limits: &Limits,
    user: Uuid,
    parent: Option<Uuid>,
    kind: &str,
    name: &str,
) -> Result<Node> {
    let (ext, bytes) = blank_content(kind).ok_or_else(|| CoreError::bad("unknown document kind"))?;
    let name = if name.to_lowercase().ends_with(&format!(".{ext}")) {
        name.to_string()
    } else {
        format!("{name}.{ext}")
    };
    let info = create(pool, storage, limits, user, parent, &name, bytes.len() as i64).await?;
    if bytes.is_empty() {
        return finish_empty(pool, storage, limits, user, info.id).await;
    }
    let chunk: std::io::Result<Bytes> = Ok(Bytes::from_static(bytes));
    let out = append(
        pool,
        storage,
        limits,
        user,
        info.id,
        0,
        Box::pin(futures_util::stream::iter([chunk])),
    )
    .await?;
    out.node
        .ok_or_else(|| CoreError::bad("the document could not be created"))
}

/// Finalise an empty file (no chunk will ever be sent for it).
pub async fn finish_empty(
    pool: &Pool,
    storage: &Storage,
    limits: &Limits,
    user: Uuid,
    id: Uuid,
) -> Result<Node> {
    let s = load(pool, user, id).await?;
    if s.total_size != 0 || part_len(storage, id).await? != 0 {
        return Err(CoreError::bad("upload is not empty"));
    }
    finalize(pool, storage, limits, &s).await
}

async fn finalize(pool: &Pool, storage: &Storage, limits: &Limits, s: &Session) -> Result<Node> {
    if let Some(node) = s.replace_node_id {
        return finalize_version(pool, storage, limits, s, node).await;
    }
    // Rights may have changed during a long upload: check again.
    let tree_owner = perms::authorize_parent(pool, s.owner_id, s.parent_id).await?;

    let mut head = vec![0u8; 512];
    let mut f = tokio::fs::File::open(storage.upload_path(s.id)).await?;
    let n = f.read(&mut head).await?;
    head.truncate(n);
    drop(f);
    let mime = detect_mime(&s.name, &head);

    let blob = Uuid::new_v4();
    let thumb_state = if thumbnailable(&mime) {
        "pending"
    } else {
        "unsupported"
    };
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO blobs (id, size, mime, thumb_state) VALUES ($1, $2, $3, $4)")
        .bind(blob)
        .bind(s.total_size)
        .bind(&mime)
        .bind(thumb_state)
        .execute(&mut *tx)
        .await?;
    let node_id: Uuid = sqlx::query_scalar(
        "INSERT INTO nodes (owner_id, parent_id, kind, name, mime, size, blob_id) \
         VALUES ($1, $2, 'file', $3, $4, $5, $6) RETURNING id",
    )
    .bind(tree_owner)
    .bind(s.parent_id)
    .bind(&s.name)
    .bind(&mime)
    .bind(s.total_size)
    .bind(blob)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM upload_sessions WHERE id = $1")
        .bind(s.id)
        .execute(&mut *tx)
        .await?;
    storage.commit_upload(s.id, blob).await?;
    if let Err(e) = tx.commit().await {
        storage.remove_blob(blob).await;
        return Err(e.into());
    }

    jobs::enqueue_blob(pool, jobs::kind::CHECKSUM, blob).await?;
    if thumbnailable(&mime) {
        jobs::enqueue_blob(pool, jobs::kind::THUMBNAIL, blob).await?;
    }
    if is_text_like(&mime, &s.name) {
        jobs::enqueue_blob(pool, jobs::kind::INDEX_TEXT, blob).await?;
    }
    nodes::get(pool, s.owner_id, node_id).await
}

/// The upload replaces the content of `node`: the old blob becomes a version.
async fn finalize_version(
    pool: &Pool,
    storage: &Storage,
    limits: &Limits,
    s: &Session,
    node: Uuid,
) -> Result<Node> {
    // Rights may have changed during a long upload: check again.
    let (n, _) = perms::authorize(pool, s.owner_id, node, Role::Editor).await?;
    if n.kind != "file" || n.trashed_at.is_some() {
        return Err(CoreError::bad("the file is no longer available"));
    }
    let mut head = vec![0u8; 512];
    let mut f = tokio::fs::File::open(storage.upload_path(s.id)).await?;
    let read = f.read(&mut head).await?;
    head.truncate(read);
    drop(f);
    let mime = detect_mime(&n.name, &head);
    let blob = Uuid::new_v4();
    let thumb_state = if thumbnailable(&mime) {
        "pending"
    } else {
        "unsupported"
    };

    let mut tx = pool.begin().await?;
    // Serialise concurrent replacements of the same file.
    let cur: (Uuid, i64, Option<String>) =
        sqlx::query_as("SELECT blob_id, size, mime FROM nodes WHERE id = $1 FOR UPDATE")
            .bind(node)
            .fetch_one(&mut *tx)
            .await?;
    sqlx::query("INSERT INTO blobs (id, size, mime, thumb_state) VALUES ($1, $2, $3, $4)")
        .bind(blob)
        .bind(s.total_size)
        .bind(&mime)
        .bind(thumb_state)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO node_versions (node_id, blob_id, size, mime) VALUES ($1, $2, $3, $4)")
        .bind(node)
        .bind(cur.0)
        .bind(cur.1)
        .bind(cur.2.unwrap_or_else(|| "application/octet-stream".into()))
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE nodes SET blob_id = $2, size = $3, mime = $4, updated_at = now() WHERE id = $1",
    )
    .bind(node)
    .bind(blob)
    .bind(s.total_size)
    .bind(&mime)
    .execute(&mut *tx)
    .await?;
    let dropped = crate::versions::prune_tx(&mut tx, node, limits.max_versions).await?;
    sqlx::query("DELETE FROM upload_sessions WHERE id = $1")
        .bind(s.id)
        .execute(&mut *tx)
        .await?;
    storage.commit_upload(s.id, blob).await?;
    if let Err(e) = tx.commit().await {
        storage.remove_blob(blob).await;
        return Err(e.into());
    }
    nodes::free_orphans(pool, storage, &dropped).await?;

    jobs::enqueue_blob(pool, jobs::kind::CHECKSUM, blob).await?;
    if thumbnailable(&mime) {
        jobs::enqueue_blob(pool, jobs::kind::THUMBNAIL, blob).await?;
    }
    if is_text_like(&mime, &n.name) {
        jobs::enqueue_blob(pool, jobs::kind::INDEX_TEXT, blob).await?;
    }
    nodes::get(pool, s.owner_id, node).await
}

pub async fn cancel(pool: &Pool, storage: &Storage, user: Uuid, id: Uuid) -> Result<()> {
    let r = sqlx::query("DELETE FROM upload_sessions WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(user)
        .execute(pool)
        .await?;
    if r.rows_affected() == 0 {
        return Err(CoreError::NotFound);
    }
    let _ = tokio::fs::remove_file(storage.upload_path(id)).await;
    Ok(())
}

/// Remove expired sessions and any stray `.part` file without a session.
pub async fn cleanup_expired(pool: &Pool, storage: &Storage) -> Result<u64> {
    let ids: Vec<Uuid> =
        sqlx::query_scalar("DELETE FROM upload_sessions WHERE expires_at < now() RETURNING id")
            .fetch_all(pool)
            .await?;
    for id in &ids {
        let _ = tokio::fs::remove_file(storage.upload_path(*id)).await;
    }
    // Stray part files older than a day with no session row.
    let dir = storage.root().join("tmp/uploads");
    let mut removed = ids.len() as u64;
    if let Ok(mut rd) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(e)) = rd.next_entry().await {
            let name = e.file_name().to_string_lossy().to_string();
            let Some(stem) = name.strip_suffix(".part") else {
                continue;
            };
            let Ok(id) = Uuid::parse_str(stem) else {
                continue;
            };
            let old = e
                .metadata()
                .await
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|d| d.as_secs() > 86_400);
            if !old {
                continue;
            }
            let has: bool =
                sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM upload_sessions WHERE id = $1)")
                    .bind(id)
                    .fetch_one(pool)
                    .await?;
            if !has && tokio::fs::remove_file(e.path()).await.is_ok() {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod blank_tests {
    use super::blank_content;

    #[test]
    fn office_templates_are_zip_archives() {
        for kind in ["docx", "xlsx", "pptx"] {
            let (ext, bytes) = blank_content(kind).unwrap();
            assert_eq!(ext, kind);
            assert!(bytes.starts_with(b"PK\x03\x04"), "{kind} is not a zip");
        }
        assert!(blank_content("md").unwrap().1.is_empty());
        assert!(blank_content("exe").is_none());
    }
}
