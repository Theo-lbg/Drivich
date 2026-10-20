// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Global search, modelled on the Google Drive behaviour: instant suggestions
//! while typing, then a full results page with Type / Owner / Date / Location
//! filters and an "advanced search" form.
//!
//! What is searched: file and folder names (substring, accelerated by a
//! trigram index), and the text content of text-like files (extracted by the
//! worker into `blob_text`). Results are always restricted in SQL to what the
//! caller owns or that has been shared with them; nothing is filtered after the
//! fact.

use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::models::{Node, node_select};
use crate::nodes::{Page, type_predicate};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, Postgres, QueryBuilder};
use uuid::Uuid;

const VISIBLE_CTE: &str = "WITH RECURSIVE shared_tree AS ( \
    SELECT n.id FROM nodes n JOIN shares s ON s.node_id = n.id WHERE s.grantee_id = ";

#[derive(Debug, Clone, Default)]
pub struct SearchParams {
    /// Free text: matches names and content.
    pub q: Option<String>,
    /// Advanced: name contains.
    pub name: Option<String>,
    /// Advanced: content contains the words.
    pub words: Option<String>,
    pub type_filter: Option<String>,
    /// `me` or `others`.
    pub owner: Option<String>,
    pub modified_from: Option<DateTime<Utc>>,
    pub modified_to: Option<DateTime<Utc>>,
    /// `drive`, `shared`, `starred` or `trash`. Default: everything visible, not trashed.
    pub location: Option<String>,
    pub in_folder: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn clean(s: &Option<String>) -> Option<String> {
    s.as_deref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.chars().take(200).collect())
}

pub async fn search(pool: &Pool, user: Uuid, p: SearchParams) -> Result<Page> {
    let limit = p.limit.clamp(1, 100);
    let q = clean(&p.q);
    let name = clean(&p.name);
    let words = clean(&p.words);

    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(VISIBLE_CTE);
    qb.push_bind(user);
    qb.push(" UNION SELECT c.id FROM nodes c JOIN shared_tree t ON c.parent_id = t.id) ");
    // Non-owners get a conservative role label; actions are re-authorized server-side.
    let role_expr = format!("CASE WHEN n.owner_id = '{user}'::uuid THEN 'owner' ELSE 'viewer' END");
    qb.push(node_select(&role_expr));
    qb.push(" WHERE (n.owner_id = ").push_bind(user);
    qb.push(" OR n.id IN (SELECT id FROM shared_tree))");

    match p.location.as_deref() {
        None => {
            qb.push(" AND n.trashed_at IS NULL");
        }
        Some("drive") => {
            qb.push(" AND n.trashed_at IS NULL AND n.owner_id = ")
                .push_bind(user);
        }
        Some("shared") => {
            qb.push(" AND n.trashed_at IS NULL AND n.owner_id <> ")
                .push_bind(user);
        }
        Some("starred") => {
            qb.push(" AND n.trashed_at IS NULL AND n.starred AND n.owner_id = ")
                .push_bind(user);
        }
        Some("trash") => {
            qb.push(" AND n.trashed_at IS NOT NULL AND n.owner_id = ")
                .push_bind(user);
        }
        Some(_) => return Err(CoreError::bad("unknown location")),
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
    if let Some(t) = p
        .type_filter
        .as_deref()
        .filter(|t| !t.is_empty() && *t != "any")
    {
        let pred = type_predicate(t).ok_or_else(|| CoreError::bad("unknown type filter"))?;
        qb.push(" AND ").push(pred);
    }
    if let Some(from) = p.modified_from {
        qb.push(" AND n.updated_at >= ").push_bind(from);
    }
    if let Some(to) = p.modified_to {
        qb.push(" AND n.updated_at < ").push_bind(to);
    }
    if let Some(folder) = p.in_folder {
        // Direct children of the folder (Drive's "in this folder").
        qb.push(" AND n.parent_id = ").push_bind(folder);
    }
    if let Some(q) = &q {
        let like = format!("%{}%", escape_like(&q.to_lowercase()));
        qb.push(" AND (lower(n.name) LIKE ")
            .push_bind(like)
            .push(" ESCAPE '\\'");
        qb.push(" OR EXISTS (SELECT 1 FROM blob_text bt WHERE bt.blob_id = n.blob_id AND bt.tsv @@ plainto_tsquery('simple', ")
            .push_bind(q.clone())
            .push(")))");
    }
    if let Some(name) = &name {
        let like = format!("%{}%", escape_like(&name.to_lowercase()));
        qb.push(" AND lower(n.name) LIKE ")
            .push_bind(like)
            .push(" ESCAPE '\\'");
    }
    if let Some(w) = &words {
        qb.push(" AND EXISTS (SELECT 1 FROM blob_text bt WHERE bt.blob_id = n.blob_id AND bt.tsv @@ plainto_tsquery('simple', ")
            .push_bind(w.clone())
            .push("))");
    }

    qb.push(" ORDER BY ");
    if let Some(q) = q.as_ref().or(name.as_ref()) {
        let lq = q.to_lowercase();
        qb.push("CASE WHEN lower(n.name) = ").push_bind(lq.clone());
        qb.push(" THEN 0 WHEN lower(n.name) LIKE ")
            .push_bind(format!("{}%", escape_like(&lq)))
            .push(" ESCAPE '\\'");
        qb.push(" THEN 1 WHEN lower(n.name) LIKE ")
            .push_bind(format!("%{}%", escape_like(&lq)))
            .push(" ESCAPE '\\'");
        qb.push(" THEN 2 ELSE 3 END, ");
    }
    qb.push("n.updated_at DESC, n.id LIMIT ")
        .push_bind(limit + 1);
    qb.push(" OFFSET ").push_bind(p.offset.max(0));

    let mut rows: Vec<Node> = qb.build_query_as().fetch_all(pool).await?;
    let next = if rows.len() as i64 > limit {
        rows.truncate(limit as usize);
        Some((p.offset.max(0) + limit).to_string())
    } else {
        None
    };
    Ok(Page {
        items: rows,
        next_cursor: next,
    })
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct Suggestion {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub mime: Option<String>,
    pub parent_id: Option<Uuid>,
    pub parent_name: Option<String>,
    pub thumb_state: Option<String>,
}

/// Up to 8 quick matches for the dropdown under the search box: names only
/// (content matches belong to the full results page, to keep this fast).
pub async fn suggest(pool: &Pool, user: Uuid, q: &str) -> Result<Vec<Suggestion>> {
    let q = q.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let lq: String = q.to_lowercase().chars().take(100).collect();
    let sql = format!(
        "{VISIBLE_CTE} $1 UNION SELECT c.id FROM nodes c JOIN shared_tree t ON c.parent_id = t.id) \
         SELECT n.id, n.kind, n.name, n.mime, n.parent_id, p.name AS parent_name, b.thumb_state \
         FROM nodes n LEFT JOIN nodes p ON p.id = n.parent_id LEFT JOIN blobs b ON b.id = n.blob_id \
         WHERE (n.owner_id = $1 OR n.id IN (SELECT id FROM shared_tree)) AND n.trashed_at IS NULL \
           AND lower(n.name) LIKE $2 ESCAPE '\\' \
         ORDER BY CASE WHEN lower(n.name) = $3 THEN 0 WHEN lower(n.name) LIKE $4 ESCAPE '\\' THEN 1 ELSE 2 END, \
                  n.updated_at DESC LIMIT 8"
    );
    Ok(sqlx::query_as(&sql)
        .bind(user)
        .bind(format!("%{}%", escape_like(&lq)))
        .bind(&lq)
        .bind(format!("{}%", escape_like(&lq)))
        .fetch_all(pool)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_wildcards_are_escaped() {
        assert_eq!(escape_like("50%_off\\"), "50\\%\\_off\\\\");
    }
}
