// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Sharing with people (viewer / commenter / editor) and public links.
//! Only the owner of an item can manage its sharing.

use crate::auth;
use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::models::{LinkEntry, Role, ShareEntry, UserBrief};
use crate::perms::{self, NodeRef};
use crate::users;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ShareInfo {
    pub owner: UserBrief,
    pub users: Vec<ShareEntry>,
    pub links: Vec<LinkEntry>,
}

fn parse_share_role(role: &str) -> Result<&'static str> {
    match role {
        "viewer" => Ok("viewer"),
        "commenter" => Ok("commenter"),
        "editor" => Ok("editor"),
        _ => Err(CoreError::bad("role must be viewer, commenter or editor")),
    }
}

pub async fn list(pool: &Pool, user: Uuid, node: Uuid) -> Result<ShareInfo> {
    let (n, _) = perms::authorize(pool, user, node, Role::Owner).await?;
    let owner: UserBrief =
        sqlx::query_as("SELECT id, email, name, avatar_version FROM users WHERE id = $1")
            .bind(n.owner_id)
            .fetch_one(pool)
            .await?;
    let users = sqlx::query_as(
        "SELECT s.id, s.grantee_id, u.name AS grantee_name, u.email AS grantee_email, u.avatar_version AS grantee_avatar_version, s.role, s.created_at \
         FROM shares s JOIN users u ON u.id = s.grantee_id WHERE s.node_id = $1 ORDER BY u.name",
    )
    .bind(node)
    .fetch_all(pool)
    .await?;
    let links = sqlx::query_as(
        "SELECT id, role, expires_at, revoked_at, created_at FROM share_links \
         WHERE node_id = $1 AND revoked_at IS NULL ORDER BY created_at DESC",
    )
    .bind(node)
    .fetch_all(pool)
    .await?;
    Ok(ShareInfo {
        owner,
        users,
        links,
    })
}

pub async fn add_user(
    pool: &Pool,
    user: Uuid,
    node: Uuid,
    email: &str,
    role: &str,
) -> Result<ShareEntry> {
    let role = parse_share_role(role)?;
    let (n, _) = perms::authorize(pool, user, node, Role::Owner).await?;
    if n.trashed_at.is_some() {
        return Err(CoreError::bad("this item is in the trash"));
    }
    let grantee = users::find_by_email(pool, email)
        .await?
        .filter(|u| !u.disabled)
        .ok_or_else(|| CoreError::bad("no active user with this email"))?;
    if grantee.id == user {
        return Err(CoreError::bad("you already own this item"));
    }
    Ok(sqlx::query_as(
        "WITH up AS ( \
             INSERT INTO shares (node_id, grantee_id, role, created_by) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (node_id, grantee_id) DO UPDATE SET role = EXCLUDED.role RETURNING * \
         ) SELECT up.id, up.grantee_id, u.name AS grantee_name, u.email AS grantee_email, u.avatar_version AS grantee_avatar_version, up.role, up.created_at \
           FROM up JOIN users u ON u.id = up.grantee_id",
    )
    .bind(node)
    .bind(grantee.id)
    .bind(role)
    .bind(user)
    .fetch_one(pool)
    .await?)
}

#[derive(sqlx::FromRow)]
struct ShareRow {
    node_id: Uuid,
    grantee_id: Uuid,
}

pub async fn update_role(pool: &Pool, user: Uuid, share: Uuid, role: &str) -> Result<()> {
    let role = parse_share_role(role)?;
    let row: ShareRow = sqlx::query_as("SELECT node_id, grantee_id FROM shares WHERE id = $1")
        .bind(share)
        .fetch_one(pool)
        .await?;
    perms::authorize(pool, user, row.node_id, Role::Owner).await?;
    sqlx::query("UPDATE shares SET role = $2 WHERE id = $1")
        .bind(share)
        .bind(role)
        .execute(pool)
        .await?;
    Ok(())
}

/// The owner can remove any share; a grantee can remove their own access.
pub async fn remove(pool: &Pool, user: Uuid, share: Uuid) -> Result<()> {
    let row: ShareRow = sqlx::query_as("SELECT node_id, grantee_id FROM shares WHERE id = $1")
        .bind(share)
        .fetch_one(pool)
        .await?;
    if row.grantee_id != user {
        perms::authorize(pool, user, row.node_id, Role::Owner).await?;
    }
    sqlx::query("DELETE FROM shares WHERE id = $1")
        .bind(share)
        .execute(pool)
        .await?;
    Ok(())
}

/// Create a public read-only link. The raw token is returned once; only its
/// hash is stored.
pub async fn create_link(
    pool: &Pool,
    user: Uuid,
    node: Uuid,
    expires_at: Option<DateTime<Utc>>,
) -> Result<(LinkEntry, String)> {
    let (n, _) = perms::authorize(pool, user, node, Role::Owner).await?;
    if n.trashed_at.is_some() {
        return Err(CoreError::bad("this item is in the trash"));
    }
    if expires_at.is_some_and(|e| e <= Utc::now()) {
        return Err(CoreError::bad("the expiry date is in the past"));
    }
    let token = auth::random_token();
    let link: LinkEntry = sqlx::query_as(
        "INSERT INTO share_links (node_id, token_hash, expires_at, created_by) VALUES ($1, $2, $3, $4) \
         RETURNING id, role, expires_at, revoked_at, created_at",
    )
    .bind(node)
    .bind(auth::hash_token(&token))
    .bind(expires_at)
    .bind(user)
    .fetch_one(pool)
    .await?;
    Ok((link, token))
}

pub async fn revoke_link(pool: &Pool, user: Uuid, link: Uuid) -> Result<()> {
    let node: Uuid = sqlx::query_scalar("SELECT node_id FROM share_links WHERE id = $1")
        .bind(link)
        .fetch_one(pool)
        .await?;
    perms::authorize(pool, user, node, Role::Owner).await?;
    sqlx::query("UPDATE share_links SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
        .bind(link)
        .execute(pool)
        .await?;
    Ok(())
}

/// Resolve a public token to the shared node. Unknown, revoked, expired links
/// and links to trashed items all answer `NotFound`.
pub async fn resolve_link(pool: &Pool, token: &str) -> Result<NodeRef> {
    let node: Option<Uuid> = sqlx::query_scalar(
        "SELECT l.node_id FROM share_links l JOIN nodes n ON n.id = l.node_id \
         WHERE l.token_hash = $1 AND l.revoked_at IS NULL AND (l.expires_at IS NULL OR l.expires_at > now()) AND n.trashed_at IS NULL",
    )
    .bind(auth::hash_token(token))
    .fetch_optional(pool)
    .await?;
    perms::node_ref(pool, node.ok_or(CoreError::NotFound)?).await
}

/// Nodes shared directly with `user` are listed by `nodes::list(View::Shared)`.
/// This counts them (sidebar badge, tests).
pub async fn count_shared_with(pool: &Pool, user: Uuid) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM shares WHERE grantee_id = $1")
            .bind(user)
            .fetch_one(pool)
            .await?,
    )
}
