// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, utoipa::ToSchema)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    #[serde(skip)]
    pub password_hash: String,
    pub is_admin: bool,
    pub disabled: bool,
    pub quota_bytes: Option<i64>,
    /// Personal link to the user's Immich instance (overrides the server default).
    pub immich_url: Option<String>,
    /// Profile picture version (0 = none).
    pub avatar_version: i64,
    /// Interface language (BCP 47); `None` = follow the browser.
    pub locale: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Minimal public view of a user, used by the sharing dialog.
#[derive(Debug, Clone, FromRow, Serialize, utoipa::ToSchema)]
pub struct UserBrief {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub avatar_version: i64,
}

/// A file or folder as returned by the API.
#[derive(Debug, Clone, FromRow, Serialize, utoipa::ToSchema)]
pub struct Node {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub mime: Option<String>,
    pub size: i64,
    pub parent_id: Option<Uuid>,
    pub owner_id: Uuid,
    pub owner_name: String,
    pub owner_avatar_version: i64,
    pub starred: bool,
    pub trashed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_opened_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
    /// `none`, `pending`, `ready`, `failed` or `unsupported` (files only).
    pub thumb_state: Option<String>,
    /// Effective role of the caller: `owner`, `editor`, `commenter` or `viewer`.
    pub role: String,
}

/// Columns and joins shared by every node query. `$ROLE` is replaced by a SQL
/// expression computing the caller's role (a literal for the owner's own views).
pub const NODE_SELECT: &str = "SELECT n.id, n.kind, n.name, n.mime, n.size, n.parent_id, n.owner_id, \
    u.name AS owner_name, u.avatar_version AS owner_avatar_version, n.starred, n.trashed_at, n.created_at, n.updated_at, n.last_opened_at, \
    n.description, b.thumb_state, $ROLE AS role \
    FROM nodes n JOIN users u ON u.id = n.owner_id LEFT JOIN blobs b ON b.id = n.blob_id";

pub fn node_select(role_expr: &str) -> String {
    NODE_SELECT.replace("$ROLE", role_expr)
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    None,
    Viewer,
    Commenter,
    Editor,
    Owner,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::None => "none",
            Role::Viewer => "viewer",
            Role::Commenter => "commenter",
            Role::Editor => "editor",
            Role::Owner => "owner",
        }
    }
    pub fn parse(s: &str) -> Role {
        match s {
            "owner" => Role::Owner,
            "editor" => Role::Editor,
            "commenter" => Role::Commenter,
            "viewer" => Role::Viewer,
            _ => Role::None,
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, utoipa::ToSchema)]
pub struct ShareEntry {
    pub id: Uuid,
    pub grantee_id: Uuid,
    pub grantee_name: String,
    pub grantee_email: String,
    pub grantee_avatar_version: i64,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, utoipa::ToSchema)]
pub struct LinkEntry {
    pub id: Uuid,
    pub role: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
