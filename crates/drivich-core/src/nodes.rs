// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Files and folders: listing, creation, rename, move, star, trash.
//!
//! Trash model: trashing a folder marks the folder and everything below it with
//! the same `trash_root_id`. Only trash roots are listed in the Trash view, and
//! restoring a root restores exactly what was trashed with it.

use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::models::{Node, Role, node_select};
use crate::perms::{self, NodeRef};
use crate::storage::Storage;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

pub const MAX_PAGE: i64 = 200;
pub const DEFAULT_PAGE: i64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Drive,
    Shared,
    Recent,
    Starred,
    Trash,
}

impl View {
    pub fn parse(s: &str) -> Option<View> {
        Some(match s {
            "drive" => View::Drive,
            "shared" => View::Shared,
            "recent" => View::Recent,
            "starred" => View::Starred,
            "trash" => View::Trash,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Modified,
    Created,
    Size,
}

impl SortBy {
    pub fn parse(s: &str) -> Option<SortBy> {
        Some(match s {
            "name" => SortBy::Name,
            "modified" => SortBy::Modified,
            "created" => SortBy::Created,
            "size" => SortBy::Size,
            _ => return None,
        })
    }
    fn column(self) -> &'static str {
        match self {
            SortBy::Name => "lower(n.name)",
            SortBy::Modified => "n.updated_at",
            SortBy::Created => "n.created_at",
            SortBy::Size => "n.size",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListParams {
    pub view: View,
    pub parent: Option<Uuid>,
    pub sort: SortBy,
    pub desc: bool,
    pub type_filter: Option<String>,
    /// Only items modified at or after this instant.
    pub modified_from: Option<DateTime<Utc>>,
    /// `me` or `others`.
    pub owner: Option<String>,
    /// Recent view only: `drive` (my files) or `shared` (shared with me).
    pub location: Option<String>,
    pub limit: i64,
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Page {
    pub items: Vec<Node>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Cursor {
    /// Folder-first bucket (0 folders, 1 files).
    f: i32,
    t: Option<String>,
    ts: Option<DateTime<Utc>>,
    i: Option<i64>,
    id: Uuid,
}

/// SQL predicate for a "Type" filter chip (shared with the search module).
pub fn type_predicate(t: &str) -> Option<&'static str> {
    Some(match t {
        "folder" => "n.kind = 'folder'",
        "image" => "n.mime LIKE 'image/%'",
        "video" => "n.mime LIKE 'video/%'",
        "audio" => "n.mime LIKE 'audio/%'",
        "pdf" => "n.mime = 'application/pdf'",
        "text" => {
            "(n.mime LIKE 'text/%' OR n.mime IN ('application/json','application/xml','application/javascript'))"
        }
        "document" => {
            "(n.mime IN ('application/msword','application/rtf','application/vnd.oasis.opendocument.text') \
                       OR n.mime LIKE 'application/vnd.openxmlformats-officedocument.%' OR n.mime LIKE 'application/vnd.ms-%')"
        }
        "archive" => {
            "n.mime IN ('application/zip','application/x-tar','application/gzip','application/x-7z-compressed','application/vnd.rar','application/x-rar-compressed','application/x-apple-diskimage')"
        }
        _ => return None,
    })
}

pub fn clean_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() {
        return Err(CoreError::bad("the name cannot be empty"));
    }
    if n.chars().count() > 255 {
        return Err(CoreError::bad("the name is too long (255 characters max)"));
    }
    if n == "."
        || n == ".."
        || n.contains('/')
        || n.contains('\\')
        || n.chars().any(|c| c.is_control())
    {
        return Err(CoreError::bad("the name contains forbidden characters"));
    }
    Ok(n.to_string())
}

fn role_literal(role: Role) -> String {
    format!("'{}'::text", role.as_str())
}

pub async fn get(pool: &Pool, user: Uuid, id: Uuid) -> Result<Node> {
    let (n, role) = perms::authorize(pool, user, id, Role::Viewer).await?;
    fetch_one(pool, n.id, role).await
}

async fn fetch_one(pool: &Pool, id: Uuid, role: Role) -> Result<Node> {
    let sql = format!("{} WHERE n.id = $1", node_select(&role_literal(role)));
    Ok(sqlx::query_as(&sql).bind(id).fetch_one(pool).await?)
}

pub async fn list(pool: &Pool, user: Uuid, p: ListParams) -> Result<Page> {
    let limit = p.limit.clamp(1, MAX_PAGE);
    match p.view {
        View::Recent => return list_recent(pool, user, limit, &p).await,
        View::Trash => return list_trash(pool, user, limit, p.cursor.as_deref()).await,
        _ => {}
    }

    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("");
    match p.view {
        View::Drive => {
            let role = match p.parent {
                None => Role::Owner,
                Some(pid) => perms::authorize(pool, user, pid, Role::Viewer).await?.1,
            };
            qb.push(node_select(&role_literal(role)));
            qb.push(" WHERE n.trashed_at IS NULL AND ");
            match p.parent {
                None => {
                    qb.push("n.parent_id IS NULL AND n.owner_id = ")
                        .push_bind(user);
                }
                Some(pid) => {
                    qb.push("n.parent_id = ").push_bind(pid);
                }
            }
        }
        View::Starred => {
            qb.push(node_select("'owner'::text"));
            qb.push(" WHERE n.trashed_at IS NULL AND n.starred AND n.owner_id = ")
                .push_bind(user);
        }
        View::Shared => {
            qb.push(node_select("s.role"));
            qb.push(
                " JOIN shares s ON s.node_id = n.id WHERE n.trashed_at IS NULL AND s.grantee_id = ",
            )
            .push_bind(user);
        }
        _ => unreachable!(),
    }
    if let Some(t) = p.type_filter.as_deref() {
        let pred = type_predicate(t).ok_or_else(|| CoreError::bad("unknown type filter"))?;
        qb.push(" AND ").push(pred);
    }
    if let Some(from) = p.modified_from {
        qb.push(" AND n.updated_at >= ").push_bind(from);
    }
    match p.owner.as_deref() {
        None | Some("") | Some("any") => {}
        Some("me") => {
            qb.push(" AND n.owner_id = ").push_bind(user);
        }
        Some("others") => {
            qb.push(" AND n.owner_id <> ").push_bind(user);
        }
        Some(_) => return Err(CoreError::bad("unknown owner filter")),
    }

    let folder_bucket = "(CASE WHEN n.kind = 'folder' THEN 0 ELSE 1 END)";
    let col = p.sort.column();
    if let Some(raw) = p.cursor.as_deref() {
        let c = decode_cursor(raw)?;
        let cmp = if p.desc { "<" } else { ">" };
        qb.push(format!(" AND ({folder_bucket} > ")).push_bind(c.f);
        qb.push(format!(" OR ({folder_bucket} = ")).push_bind(c.f);
        qb.push(format!(" AND {col} {cmp} "));
        push_sort_value(&mut qb, p.sort, &c)?;
        qb.push(format!(" ) OR ({folder_bucket} = ")).push_bind(c.f);
        qb.push(format!(" AND {col} = "));
        push_sort_value(&mut qb, p.sort, &c)?;
        qb.push(" AND n.id > ").push_bind(c.id);
        qb.push("))");
    }
    qb.push(format!(
        " ORDER BY {folder_bucket} ASC, {col} {}, n.id ASC LIMIT ",
        if p.desc { "DESC" } else { "ASC" }
    ));
    qb.push_bind(limit + 1);

    let mut rows: Vec<Node> = qb.build_query_as().fetch_all(pool).await?;
    let next_cursor = if rows.len() as i64 > limit {
        rows.truncate(limit as usize);
        rows.last().map(|n| encode_cursor(n, p.sort))
    } else {
        None
    };
    Ok(Page {
        items: rows,
        next_cursor,
    })
}

fn push_sort_value(qb: &mut QueryBuilder<'_, Postgres>, sort: SortBy, c: &Cursor) -> Result<()> {
    match sort {
        SortBy::Name => {
            qb.push_bind(
                c.t.clone()
                    .ok_or_else(|| CoreError::bad("invalid cursor"))?,
            );
        }
        SortBy::Modified | SortBy::Created => {
            qb.push_bind(c.ts.ok_or_else(|| CoreError::bad("invalid cursor"))?);
        }
        SortBy::Size => {
            qb.push_bind(c.i.ok_or_else(|| CoreError::bad("invalid cursor"))?);
        }
    }
    Ok(())
}

fn encode_cursor(n: &Node, sort: SortBy) -> String {
    let c = Cursor {
        f: if n.kind == "folder" { 0 } else { 1 },
        t: matches!(sort, SortBy::Name).then(|| n.name.to_lowercase()),
        ts: match sort {
            SortBy::Modified => Some(n.updated_at),
            SortBy::Created => Some(n.created_at),
            _ => None,
        },
        i: matches!(sort, SortBy::Size).then_some(n.size),
        id: n.id,
    };
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&c).unwrap_or_default())
}

fn decode_cursor(raw: &str) -> Result<Cursor> {
    let bytes = URL_SAFE_NO_PAD
        .decode(raw)
        .map_err(|_| CoreError::bad("invalid cursor"))?;
    serde_json::from_slice(&bytes).map_err(|_| CoreError::bad("invalid cursor"))
}

fn offset_cursor(raw: Option<&str>) -> Result<i64> {
    match raw {
        None => Ok(0),
        Some(s) => s
            .parse::<i64>()
            .ok()
            .filter(|n| *n >= 0)
            .ok_or_else(|| CoreError::bad("invalid cursor")),
    }
}

/// "Récents": files by last activity (opened or modified) — the caller's own and
/// those shared with them — with Drive's Type / Contacts / Date / Source filters.
async fn list_recent(pool: &Pool, user: Uuid, limit: i64, p: &ListParams) -> Result<Page> {
    const ACTIVITY: &str = "GREATEST(COALESCE(n.last_opened_at, n.updated_at), n.updated_at)";
    let offset = offset_cursor(p.cursor.as_deref())?;
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "WITH RECURSIVE shared_tree AS ( \
             SELECT n.id FROM nodes n JOIN shares s ON s.node_id = n.id WHERE s.grantee_id = ",
    );
    qb.push_bind(user);
    qb.push(" UNION SELECT c.id FROM nodes c JOIN shared_tree t ON c.parent_id = t.id) ");
    // Shared files get a conservative role label; actions are re-authorized server-side.
    qb.push(node_select(&format!(
        "CASE WHEN n.owner_id = '{user}'::uuid THEN 'owner' ELSE 'viewer' END"
    )));
    qb.push(" WHERE n.kind = 'file' AND n.trashed_at IS NULL");
    match p.location.as_deref() {
        None | Some("") | Some("any") => {
            qb.push(" AND (n.owner_id = ").push_bind(user);
            qb.push(" OR n.id IN (SELECT id FROM shared_tree))");
        }
        Some("drive") => {
            qb.push(" AND n.owner_id = ").push_bind(user);
        }
        Some("shared") => {
            qb.push(" AND n.owner_id <> ").push_bind(user);
            qb.push(" AND n.id IN (SELECT id FROM shared_tree)");
        }
        Some(_) => return Err(CoreError::bad("unknown source filter")),
    }
    if let Some(t) = p.type_filter.as_deref() {
        let pred = type_predicate(t).ok_or_else(|| CoreError::bad("unknown type filter"))?;
        qb.push(" AND ").push(pred);
    }
    if let Some(from) = p.modified_from {
        qb.push(format!(" AND {ACTIVITY} >= ")).push_bind(from);
    }
    match p.owner.as_deref() {
        None | Some("") | Some("any") => {}
        Some("me") => {
            qb.push(" AND n.owner_id = ").push_bind(user);
        }
        Some("others") => {
            qb.push(" AND n.owner_id <> ").push_bind(user);
        }
        Some(_) => return Err(CoreError::bad("unknown owner filter")),
    }
    qb.push(format!(" ORDER BY {ACTIVITY} DESC, n.id LIMIT "))
        .push_bind(limit + 1);
    qb.push(" OFFSET ").push_bind(offset);

    let mut rows: Vec<Node> = qb.build_query_as().fetch_all(pool).await?;
    let next = if rows.len() as i64 > limit {
        rows.truncate(limit as usize);
        Some((offset + limit).to_string())
    } else {
        None
    };
    Ok(Page {
        items: rows,
        next_cursor: next,
    })
}

async fn list_trash(pool: &Pool, user: Uuid, limit: i64, cursor: Option<&str>) -> Result<Page> {
    let offset = offset_cursor(cursor)?;
    let sql = format!(
        "{} WHERE n.owner_id = $1 AND n.trashed_at IS NOT NULL AND n.trash_root_id = n.id \
         ORDER BY n.trashed_at DESC, n.id LIMIT $2 OFFSET $3",
        node_select("'owner'::text")
    );
    let mut rows: Vec<Node> = sqlx::query_as(&sql)
        .bind(user)
        .bind(limit + 1)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    let next = if rows.len() as i64 > limit {
        rows.truncate(limit as usize);
        Some((offset + limit).to_string())
    } else {
        None
    };
    Ok(Page {
        items: rows,
        next_cursor: next,
    })
}

pub async fn create_folder(
    pool: &Pool,
    user: Uuid,
    parent: Option<Uuid>,
    name: &str,
) -> Result<Node> {
    let name = clean_name(name)?;
    let owner = perms::authorize_parent(pool, user, parent).await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO nodes (owner_id, parent_id, kind, name) VALUES ($1, $2, 'folder', $3) RETURNING id",
    )
    .bind(owner)
    .bind(parent)
    .bind(name)
    .fetch_one(pool)
    .await?;
    let role = if owner == user {
        Role::Owner
    } else {
        Role::Editor
    };
    fetch_one(pool, id, role).await
}

pub async fn rename(pool: &Pool, user: Uuid, id: Uuid, name: &str) -> Result<Node> {
    let name = clean_name(name)?;
    let (n, role) = perms::authorize(pool, user, id, Role::Editor).await?;
    if n.trashed_at.is_some() {
        return Err(CoreError::bad("restore the item before renaming it"));
    }
    sqlx::query("UPDATE nodes SET name = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(name)
        .execute(pool)
        .await?;
    fetch_one(pool, id, role).await
}

pub async fn set_description(
    pool: &Pool,
    user: Uuid,
    id: Uuid,
    description: Option<&str>,
) -> Result<Node> {
    let (_, role) = perms::authorize(pool, user, id, Role::Editor).await?;
    let d = description.map(|s| s.trim()).filter(|s| !s.is_empty());
    if d.is_some_and(|s| s.chars().count() > 2000) {
        return Err(CoreError::bad("description too long"));
    }
    sqlx::query("UPDATE nodes SET description = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(d)
        .execute(pool)
        .await?;
    fetch_one(pool, id, role).await
}

/// Stars are personal to the owner (shared items cannot be starred by others).
pub async fn set_starred(pool: &Pool, user: Uuid, id: Uuid, starred: bool) -> Result<Node> {
    let (n, role) = perms::authorize(pool, user, id, Role::Owner).await?;
    if n.trashed_at.is_some() {
        return Err(CoreError::bad("item is in the trash"));
    }
    sqlx::query("UPDATE nodes SET starred = $2 WHERE id = $1")
        .bind(id)
        .bind(starred)
        .execute(pool)
        .await?;
    fetch_one(pool, id, role).await
}

pub async fn move_to(pool: &Pool, user: Uuid, id: Uuid, new_parent: Option<Uuid>) -> Result<Node> {
    let (n, role) = perms::authorize(pool, user, id, Role::Editor).await?;
    if n.trashed_at.is_some() {
        return Err(CoreError::bad("restore the item before moving it"));
    }
    let dest_owner = perms::authorize_parent(pool, user, new_parent).await?;
    if dest_owner != n.owner_id {
        return Err(CoreError::bad(
            "items cannot be moved into another person's Drive",
        ));
    }
    if let Some(p) = new_parent {
        if p == id || perms::is_within(pool, p, id).await? {
            return Err(CoreError::bad("a folder cannot be moved into itself"));
        }
    }
    if n.parent_id == new_parent {
        return fetch_one(pool, id, role).await;
    }
    sqlx::query("UPDATE nodes SET parent_id = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(new_parent)
        .execute(pool)
        .await?;
    fetch_one(pool, id, role).await
}

/// Record that the owner opened a file (feeds the "Recent" view).
pub async fn mark_opened(pool: &Pool, user: Uuid, id: Uuid) -> Result<()> {
    let (n, _) = perms::authorize(pool, user, id, Role::Viewer).await?;
    if n.owner_id == user && n.kind == "file" {
        sqlx::query("UPDATE nodes SET last_opened_at = now() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Ancestors from the top of what the caller may see down to the node itself.
pub async fn breadcrumb(pool: &Pool, user: Uuid, id: Uuid) -> Result<Vec<(Uuid, String)>> {
    let (node, _) = perms::authorize(pool, user, id, Role::Viewer).await?;
    let rows: Vec<(Uuid, String, Option<Uuid>, bool)> = sqlx::query_as(
        "WITH RECURSIVE anc AS ( \
             SELECT id, parent_id, name, 0 AS depth FROM nodes WHERE id = $1 \
             UNION ALL \
             SELECT n.id, n.parent_id, n.name, a.depth + 1 FROM nodes n JOIN anc a ON n.id = a.parent_id \
         ) \
         SELECT a.id, a.name, a.parent_id, \
                EXISTS (SELECT 1 FROM shares s WHERE s.node_id = a.id AND s.grantee_id = $2) AS shared_with_me \
         FROM anc a ORDER BY a.depth DESC",
    )
    .bind(id)
    .bind(user)
    .fetch_all(pool)
    .await?;
    if node.owner_id == user {
        return Ok(rows.into_iter().map(|(i, n, _, _)| (i, n)).collect());
    }
    // A shared-with user starts at the highest folder shared with them.
    let start = rows.iter().position(|r| r.3).unwrap_or(0);
    Ok(rows
        .into_iter()
        .skip(start)
        .map(|(i, n, _, _)| (i, n))
        .collect())
}

// ---- Trash ---------------------------------------------------------------

pub async fn trash(pool: &Pool, user: Uuid, id: Uuid) -> Result<()> {
    let (n, _) = perms::authorize(pool, user, id, Role::Owner).await?;
    if n.trashed_at.is_some() {
        return Ok(());
    }
    sqlx::query(
        "WITH RECURSIVE sub AS ( \
             SELECT id FROM nodes WHERE id = $1 \
             UNION ALL \
             SELECT c.id FROM nodes c JOIN sub s ON c.parent_id = s.id \
         ) \
         UPDATE nodes SET trashed_at = now(), trash_root_id = $1, updated_at = now() \
         WHERE id IN (SELECT id FROM sub) AND trashed_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn restore(pool: &Pool, user: Uuid, id: Uuid) -> Result<Node> {
    let (n, _) = perms::authorize(pool, user, id, Role::Owner).await?;
    if n.trashed_at.is_none() {
        return fetch_one(pool, id, Role::Owner).await;
    }
    let root = n.trash_root_id.unwrap_or(id);
    let mut tx = pool.begin().await?;
    // If the original parent is still in the trash, restore at the top level.
    sqlx::query(
        "UPDATE nodes SET parent_id = NULL WHERE id = $1 AND parent_id IN \
         (SELECT id FROM nodes WHERE trashed_at IS NOT NULL AND trash_root_id IS DISTINCT FROM $1)",
    )
    .bind(root)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE nodes SET trashed_at = NULL, trash_root_id = NULL, updated_at = now() WHERE trash_root_id = $1")
        .bind(root)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    fetch_one(pool, root, Role::Owner).await
}

/// Permanently delete an item that is in the trash, and free orphaned blobs.
pub async fn delete_permanently(
    pool: &Pool,
    storage: &Storage,
    user: Uuid,
    id: Uuid,
) -> Result<()> {
    let (n, _) = perms::authorize(pool, user, id, Role::Owner).await?;
    if n.trashed_at.is_none() {
        return Err(CoreError::bad(
            "only items in the trash can be deleted permanently",
        ));
    }
    let blobs: Vec<Uuid> = sqlx::query_scalar(
        "WITH RECURSIVE sub AS ( \
             SELECT id, blob_id FROM nodes WHERE id = $1 \
             UNION ALL \
             SELECT c.id, c.blob_id FROM nodes c JOIN sub s ON c.parent_id = s.id \
         ) SELECT blob_id FROM sub WHERE blob_id IS NOT NULL \
           UNION SELECT v.blob_id FROM node_versions v JOIN sub ON v.node_id = sub.id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    sqlx::query("DELETE FROM nodes WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    free_orphans(pool, storage, &blobs).await
}

pub async fn empty_trash(pool: &Pool, storage: &Storage, user: Uuid) -> Result<u64> {
    let blobs: Vec<Uuid> = sqlx::query_scalar(
        "SELECT blob_id FROM nodes WHERE owner_id = $1 AND trashed_at IS NOT NULL AND blob_id IS NOT NULL \
         UNION SELECT v.blob_id FROM node_versions v JOIN nodes n ON n.id = v.node_id \
               WHERE n.owner_id = $1 AND n.trashed_at IS NOT NULL",
    )
    .bind(user)
    .fetch_all(pool)
    .await?;
    let res = sqlx::query(
        "DELETE FROM nodes WHERE owner_id = $1 AND trashed_at IS NOT NULL AND trash_root_id = id",
    )
    .bind(user)
    .execute(pool)
    .await?;
    free_orphans(pool, storage, &blobs).await?;
    Ok(res.rows_affected())
}

/// Delete blob rows (and their files) that no node references any more.
pub async fn free_orphans(pool: &Pool, storage: &Storage, candidates: &[Uuid]) -> Result<()> {
    if candidates.is_empty() {
        return Ok(());
    }
    let orphans: Vec<Uuid> = sqlx::query_scalar(
        "DELETE FROM blobs b WHERE b.id = ANY($1) AND NOT EXISTS (SELECT 1 FROM nodes n WHERE n.blob_id = b.id) AND NOT EXISTS (SELECT 1 FROM node_versions v WHERE v.blob_id = b.id) RETURNING b.id",
    )
    .bind(candidates)
    .fetch_all(pool)
    .await?;
    for id in orphans {
        storage.remove_blob(id).await;
    }
    Ok(())
}

/// Items trashed longer than `days` ago, owned by anyone (worker maintenance).
pub async fn purge_old_trash(pool: &Pool, storage: &Storage, days: i64) -> Result<u64> {
    let blobs: Vec<Uuid> = sqlx::query_scalar(
        "SELECT blob_id FROM nodes WHERE trashed_at < now() - make_interval(days => $1::int) AND blob_id IS NOT NULL \
         UNION SELECT v.blob_id FROM node_versions v JOIN nodes n ON n.id = v.node_id \
               WHERE n.trashed_at < now() - make_interval(days => $1::int)",
    )
    .bind(days as i32)
    .fetch_all(pool)
    .await?;
    let res = sqlx::query(
        "DELETE FROM nodes WHERE trashed_at < now() - make_interval(days => $1::int) AND trash_root_id = id",
    )
    .bind(days as i32)
    .execute(pool)
    .await?;
    free_orphans(pool, storage, &blobs).await?;
    Ok(res.rows_affected())
}

// ---- Public links ---------------------------------------------------------

/// A node as seen through a public link (always read-only).
pub async fn get_public(pool: &Pool, id: Uuid) -> Result<Node> {
    fetch_one(pool, id, Role::Viewer).await
}

/// Children of a folder reached through a public link: folders first, then by name.
pub async fn list_public_children(
    pool: &Pool,
    parent: Uuid,
    limit: i64,
    cursor: Option<&str>,
) -> Result<Page> {
    let limit = limit.clamp(1, MAX_PAGE);
    let offset = offset_cursor(cursor)?;
    let sql = format!(
        "{} WHERE n.parent_id = $1 AND n.trashed_at IS NULL \
         ORDER BY (CASE WHEN n.kind = 'folder' THEN 0 ELSE 1 END), lower(n.name), n.id LIMIT $2 OFFSET $3",
        node_select("'viewer'::text")
    );
    let mut rows: Vec<Node> = sqlx::query_as(&sql)
        .bind(parent)
        .bind(limit + 1)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    let next = if rows.len() as i64 > limit {
        rows.truncate(limit as usize);
        Some((offset + limit).to_string())
    } else {
        None
    };
    Ok(Page {
        items: rows,
        next_cursor: next,
    })
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BatchResult {
    pub ok: Vec<Uuid>,
    pub failed: Vec<(Uuid, String)>,
}

pub async fn load_ref(pool: &Pool, user: Uuid, id: Uuid, needed: Role) -> Result<NodeRef> {
    Ok(perms::authorize(pool, user, id, needed).await?.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_validated() {
        assert_eq!(clean_name("  rapport.pdf ").unwrap(), "rapport.pdf");
        assert!(clean_name("").is_err());
        assert!(clean_name("a/b").is_err());
        assert!(clean_name("..").is_err());
        assert!(clean_name("tab\there").is_err());
        assert!(clean_name(&"x".repeat(256)).is_err());
    }

    #[test]
    fn cursors_roundtrip() {
        let c = Cursor {
            f: 1,
            t: Some("abc".into()),
            ts: None,
            i: None,
            id: Uuid::nil(),
        };
        let enc = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&c).unwrap());
        let back = decode_cursor(&enc).unwrap();
        assert_eq!(back.f, 1);
        assert_eq!(back.t.as_deref(), Some("abc"));
        assert!(decode_cursor("@@@").is_err());
    }

    #[test]
    fn type_filters_exist() {
        for t in [
            "folder", "image", "video", "audio", "pdf", "text", "document", "archive",
        ] {
            assert!(type_predicate(t).is_some(), "{t}");
        }
        assert!(type_predicate("nope").is_none());
    }
}
