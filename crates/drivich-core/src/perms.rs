// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Central authorization. Every operation on a node goes through
//! [`authorize`]; nothing else decides who may touch what.
//!
//! Rules:
//! * the owner has every right on their own tree;
//! * a share on a folder applies to the whole subtree below it;
//! * trashed items are visible to their owner only;
//! * a caller with no access at all gets `NotFound`, never `Forbidden`,
//!   so the existence of private items is not revealed.

use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::models::Role;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct NodeRef {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub kind: String,
    pub name: String,
    pub blob_id: Option<Uuid>,
    pub size: i64,
    pub mime: Option<String>,
    pub trashed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub trash_root_id: Option<Uuid>,
}

const NODE_REF_COLS: &str =
    "id, owner_id, parent_id, kind, name, blob_id, size, mime, trashed_at, trash_root_id";

pub async fn node_ref(pool: &Pool, id: Uuid) -> Result<NodeRef> {
    let sql = format!("SELECT {NODE_REF_COLS} FROM nodes WHERE id = $1");
    Ok(sqlx::query_as(&sql).bind(id).fetch_one(pool).await?)
}

/// Highest role granted to `user` on `node` through shares on the node itself
/// or any of its ancestors.
async fn shared_role(pool: &Pool, user: Uuid, node: Uuid) -> Result<Role> {
    let roles: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE anc AS ( \
             SELECT id, parent_id FROM nodes WHERE id = $1 \
             UNION ALL \
             SELECT n.id, n.parent_id FROM nodes n JOIN anc a ON n.id = a.parent_id \
         ) \
         SELECT s.role FROM shares s WHERE s.grantee_id = $2 AND s.node_id IN (SELECT id FROM anc)",
    )
    .bind(node)
    .bind(user)
    .fetch_all(pool)
    .await?;
    Ok(roles
        .iter()
        .map(|r| Role::parse(r))
        .max()
        .unwrap_or(Role::None))
}

pub async fn role_on(pool: &Pool, user: Uuid, node: &NodeRef) -> Result<Role> {
    if node.owner_id == user {
        return Ok(Role::Owner);
    }
    if node.trashed_at.is_some() {
        return Ok(Role::None);
    }
    shared_role(pool, user, node.id).await
}

/// Load a node and verify the caller holds at least `needed` on it.
pub async fn authorize(pool: &Pool, user: Uuid, id: Uuid, needed: Role) -> Result<(NodeRef, Role)> {
    let node = node_ref(pool, id).await?;
    let role = role_on(pool, user, &node).await?;
    if role == Role::None {
        return Err(CoreError::NotFound);
    }
    if role < needed {
        return Err(CoreError::Forbidden);
    }
    Ok((node, role))
}

/// Is `node` equal to, or inside, the subtree rooted at `root`?
pub async fn is_within(pool: &Pool, node: Uuid, root: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "WITH RECURSIVE anc AS ( \
             SELECT id, parent_id FROM nodes WHERE id = $1 \
             UNION ALL \
             SELECT n.id, n.parent_id FROM nodes n JOIN anc a ON n.id = a.parent_id \
         ) SELECT EXISTS (SELECT 1 FROM anc WHERE id = $2)",
    )
    .bind(node)
    .bind(root)
    .fetch_one(pool)
    .await?)
}

/// Check a destination folder (or the caller's root when `None`) accepts new
/// children from `user`, returning the owner of that tree.
pub async fn authorize_parent(pool: &Pool, user: Uuid, parent: Option<Uuid>) -> Result<Uuid> {
    match parent {
        None => Ok(user),
        Some(pid) => {
            let (p, _) = authorize(pool, user, pid, Role::Editor).await?;
            if p.kind != "folder" {
                return Err(CoreError::bad("the destination is not a folder"));
            }
            if p.trashed_at.is_some() {
                return Err(CoreError::bad("the destination folder is in the trash"));
            }
            Ok(p.owner_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_ordering_matches_privilege() {
        assert!(Role::Owner > Role::Editor);
        assert!(Role::Editor > Role::Commenter);
        assert!(Role::Commenter > Role::Viewer);
        assert!(Role::Viewer > Role::None);
        assert_eq!(Role::parse("editor"), Role::Editor);
        assert_eq!(Role::parse("garbage"), Role::None);
    }
}
