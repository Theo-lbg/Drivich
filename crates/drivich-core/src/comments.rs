// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Comments on files and folders.
//!
//! Anyone who can open an item can read its comments; writing needs the
//! `commenter` role or better (owner, editor). Authors can edit and delete
//! their own comments; the owner of the item can delete any of them.
//! Public links never expose comments.

use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::models::Role;
use crate::perms;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

pub const MAX_BODY_CHARS: usize = 4000;
pub const MAX_PER_NODE: i64 = 1000;

#[derive(Debug, Clone, FromRow)]
struct Row {
    id: Uuid,
    node_id: Uuid,
    author_id: Uuid,
    author_name: String,
    author_avatar_version: i64,
    body: String,
    created_at: DateTime<Utc>,
    edited_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Comment {
    pub id: Uuid,
    pub node_id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub author_avatar_version: i64,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    /// The caller wrote it.
    pub can_edit: bool,
    /// The caller wrote it or owns the item.
    pub can_delete: bool,
}

const SELECT: &str = "SELECT c.id, c.node_id, c.author_id, u.name AS author_name, \
    u.avatar_version AS author_avatar_version, c.body, c.created_at, c.edited_at \
    FROM comments c JOIN users u ON u.id = c.author_id";

fn present(r: Row, user: Uuid, is_owner: bool) -> Comment {
    let mine = r.author_id == user;
    Comment {
        id: r.id,
        node_id: r.node_id,
        author_id: r.author_id,
        author_name: r.author_name,
        author_avatar_version: r.author_avatar_version,
        body: r.body,
        created_at: r.created_at,
        edited_at: r.edited_at,
        can_edit: mine,
        can_delete: mine || is_owner,
    }
}

pub fn clean_body(body: &str) -> Result<String> {
    let b = body.trim();
    if b.is_empty() {
        return Err(CoreError::bad("the comment is empty"));
    }
    if b.chars().count() > MAX_BODY_CHARS {
        return Err(CoreError::bad(format!(
            "the comment is too long ({MAX_BODY_CHARS} characters max)"
        )));
    }
    if b.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
        return Err(CoreError::bad("the comment contains forbidden characters"));
    }
    Ok(b.to_string())
}

pub async fn list(pool: &Pool, user: Uuid, node: Uuid) -> Result<Vec<Comment>> {
    let (_, role) = perms::authorize(pool, user, node, Role::Viewer).await?;
    let sql =
        format!("{SELECT} WHERE c.node_id = $1 ORDER BY c.created_at, c.id LIMIT {MAX_PER_NODE}");
    let rows: Vec<Row> = sqlx::query_as(&sql).bind(node).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|r| present(r, user, role == Role::Owner))
        .collect())
}

pub async fn add(pool: &Pool, user: Uuid, node: Uuid, body: &str) -> Result<Comment> {
    let (n, role) = perms::authorize(pool, user, node, Role::Commenter).await?;
    if n.trashed_at.is_some() {
        return Err(CoreError::bad("the item is in the trash"));
    }
    let body = clean_body(body)?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM comments WHERE node_id = $1")
        .bind(node)
        .fetch_one(pool)
        .await?;
    if count >= MAX_PER_NODE {
        return Err(CoreError::TooManyRequests);
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO comments (node_id, author_id, body) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(node)
    .bind(user)
    .bind(&body)
    .fetch_one(pool)
    .await?;
    let sql = format!("{SELECT} WHERE c.id = $1");
    let row: Row = sqlx::query_as(&sql).bind(id).fetch_one(pool).await?;
    Ok(present(row, user, role == Role::Owner))
}

/// Load a comment together with the caller's role on its item. A caller who
/// cannot see the item gets `NotFound`, like everywhere else.
async fn load(pool: &Pool, user: Uuid, id: Uuid) -> Result<(Row, Role)> {
    let sql = format!("{SELECT} WHERE c.id = $1");
    let row: Row = sqlx::query_as(&sql).bind(id).fetch_one(pool).await?;
    let (_, role) = perms::authorize(pool, user, row.node_id, Role::Viewer).await?;
    Ok((row, role))
}

pub async fn edit(pool: &Pool, user: Uuid, id: Uuid, body: &str) -> Result<Comment> {
    let (row, role) = load(pool, user, id).await?;
    if row.author_id != user {
        return Err(CoreError::Forbidden);
    }
    let body = clean_body(body)?;
    sqlx::query("UPDATE comments SET body = $2, edited_at = now() WHERE id = $1")
        .bind(id)
        .bind(&body)
        .execute(pool)
        .await?;
    let sql = format!("{SELECT} WHERE c.id = $1");
    let row: Row = sqlx::query_as(&sql).bind(id).fetch_one(pool).await?;
    Ok(present(row, user, role == Role::Owner))
}

pub async fn delete(pool: &Pool, user: Uuid, id: Uuid) -> Result<()> {
    let (row, role) = load(pool, user, id).await?;
    if row.author_id != user && role != Role::Owner {
        return Err(CoreError::Forbidden);
    }
    sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bodies_are_validated() {
        assert_eq!(clean_body("  hi\nthere ").unwrap(), "hi\nthere");
        assert!(clean_body("   ").is_err());
        assert!(clean_body(&"x".repeat(MAX_BODY_CHARS + 1)).is_err());
        assert!(clean_body("a\u{0}b").is_err());
        assert!(clean_body(&"é".repeat(MAX_BODY_CHARS)).is_ok());
    }
}
