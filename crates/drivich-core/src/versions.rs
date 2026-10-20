// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! File versions. A node always points at its current blob; when a new
//! upload replaces the content, the previous blob is archived in
//! `node_versions` (newest first). Only the last `max_versions` are kept.
//! Archived versions count towards the owner's quota.

use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::models::{Node, Role};
use crate::nodes;
use crate::perms::{self, NodeRef};
use crate::storage::Storage;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

pub const DEFAULT_MAX_VERSIONS: i64 = 20;

#[derive(Debug, Clone, FromRow, Serialize, utoipa::ToSchema)]
pub struct Version {
    pub id: Uuid,
    pub size: i64,
    pub mime: String,
    /// When this content was first stored.
    pub saved_at: DateTime<Utc>,
    /// When it stopped being the current content.
    pub replaced_at: DateTime<Utc>,
}

async fn file_node(pool: &Pool, user: Uuid, id: Uuid, needed: Role) -> Result<NodeRef> {
    let (n, _) = perms::authorize(pool, user, id, needed).await?;
    if n.kind != "file" || n.blob_id.is_none() {
        return Err(CoreError::bad("folders have no versions"));
    }
    Ok(n)
}

/// Older versions of a file, newest first (the current content is the node itself).
pub async fn list(pool: &Pool, user: Uuid, node: Uuid) -> Result<Vec<Version>> {
    file_node(pool, user, node, Role::Viewer).await?;
    Ok(sqlx::query_as(
        "SELECT v.id, v.size, v.mime, b.created_at AS saved_at, v.created_at AS replaced_at \
         FROM node_versions v JOIN blobs b ON b.id = v.blob_id \
         WHERE v.node_id = $1 ORDER BY v.created_at DESC, v.id",
    )
    .bind(node)
    .fetch_all(pool)
    .await?)
}

/// The node and the blob/mime of one archived version, for downloading it.
pub async fn locate(
    pool: &Pool,
    user: Uuid,
    node: Uuid,
    version: Uuid,
) -> Result<(NodeRef, Uuid, String)> {
    let n = file_node(pool, user, node, Role::Viewer).await?;
    let (blob, mime): (Uuid, String) =
        sqlx::query_as("SELECT blob_id, mime FROM node_versions WHERE id = $1 AND node_id = $2")
            .bind(version)
            .bind(node)
            .fetch_one(pool)
            .await?;
    Ok((n, blob, mime))
}

/// Delete versions beyond the newest `keep`; returns the blobs they used.
pub(crate) async fn prune_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: Uuid,
    keep: i64,
) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "DELETE FROM node_versions WHERE node_id = $1 AND id IN ( \
             SELECT id FROM node_versions WHERE node_id = $1 \
             ORDER BY created_at DESC, id OFFSET $2) RETURNING blob_id",
    )
    .bind(node)
    .bind(keep.max(0))
    .fetch_all(&mut **tx)
    .await?)
}

/// Make an archived version the current content. The content it replaces is
/// archived in turn, so nothing is ever lost by restoring.
pub async fn restore(
    pool: &Pool,
    storage: &Storage,
    user: Uuid,
    node: Uuid,
    version: Uuid,
    keep: i64,
) -> Result<Node> {
    let n = file_node(pool, user, node, Role::Editor).await?;
    if n.trashed_at.is_some() {
        return Err(CoreError::bad("the file is in the trash"));
    }
    let mut tx = pool.begin().await?;
    // Lock the node so concurrent replacements serialise.
    let cur: (Uuid, i64, Option<String>) =
        sqlx::query_as("SELECT blob_id, size, mime FROM nodes WHERE id = $1 FOR UPDATE")
            .bind(node)
            .fetch_one(&mut *tx)
            .await?;
    let (vblob, vsize, vmime): (Uuid, i64, String) = sqlx::query_as(
        "DELETE FROM node_versions WHERE id = $1 AND node_id = $2 RETURNING blob_id, size, mime",
    )
    .bind(version)
    .bind(node)
    .fetch_one(&mut *tx)
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
    .bind(vblob)
    .bind(vsize)
    .bind(&vmime)
    .execute(&mut *tx)
    .await?;
    let dropped = prune_tx(&mut tx, node, keep).await?;
    tx.commit().await?;
    nodes::free_orphans(pool, storage, &dropped).await?;
    nodes::get(pool, user, node).await
}

pub async fn delete(
    pool: &Pool,
    storage: &Storage,
    user: Uuid,
    node: Uuid,
    version: Uuid,
) -> Result<()> {
    file_node(pool, user, node, Role::Editor).await?;
    let blob: Uuid = sqlx::query_scalar(
        "DELETE FROM node_versions WHERE id = $1 AND node_id = $2 RETURNING blob_id",
    )
    .bind(version)
    .bind(node)
    .fetch_one(pool)
    .await?;
    nodes::free_orphans(pool, storage, &[blob]).await
}
