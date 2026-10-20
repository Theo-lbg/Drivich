// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Listing what goes into a .zip download of folders. The caller has already
//! authorised each root; everything below a root is covered by the same grant
//! (shares apply to whole subtrees). Trashed items are left out.

use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::perms::NodeRef;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// A download bigger than this many entries is refused (keeps the listing small).
pub const MAX_ENTRIES: usize = 50_000;
const MAX_DEPTH: i32 = 64;

#[derive(Debug, Clone)]
pub struct Entry {
    /// Path inside the archive; ends with `/` for directories.
    pub path: String,
    /// `None` for directories.
    pub blob_id: Option<Uuid>,
    pub size: i64,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct Row {
    kind: String,
    blob_id: Option<Uuid>,
    size: i64,
    updated_at: DateTime<Utc>,
    path: String,
}

/// Archive-safe version of one path component.
pub fn safe_component(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let s = s.trim().to_string();
    match s.as_str() {
        "" | "." | ".." => "_".to_string(),
        _ => s,
    }
}

/// Split `name.ext` so a duplicate can become `name (2).ext`.
fn with_suffix(path: &str, n: usize) -> String {
    let (dir, last) = match path.trim_end_matches('/').rsplit_once('/') {
        Some((d, l)) => (format!("{d}/"), l.to_string()),
        None => (String::new(), path.trim_end_matches('/').to_string()),
    };
    let trailing = if path.ends_with('/') { "/" } else { "" };
    match last.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{dir}{stem} ({n}).{ext}{trailing}"),
        _ => format!("{dir}{last} ({n}){trailing}"),
    }
}

struct Names(HashSet<String>);

impl Names {
    /// A path not used yet (case-insensitively, as most extractors are).
    fn unique(&mut self, path: String) -> String {
        let mut candidate = path.clone();
        let mut n = 1;
        while !self
            .0
            .insert(candidate.trim_end_matches('/').to_lowercase())
        {
            n += 1;
            candidate = with_suffix(&path, n);
        }
        candidate
    }
}

/// Everything to put in the archive for `roots` (files and folders).
pub async fn collect(pool: &Pool, roots: &[NodeRef]) -> Result<Vec<Entry>> {
    let mut names = Names(HashSet::new());
    let mut out: Vec<Entry> = Vec::new();
    for root in roots {
        let top = safe_component(&root.name);
        if root.kind == "file" {
            let Some(blob) = root.blob_id else { continue };
            let modified: DateTime<Utc> =
                sqlx::query_scalar("SELECT updated_at FROM nodes WHERE id = $1")
                    .bind(root.id)
                    .fetch_one(pool)
                    .await?;
            out.push(Entry {
                path: names.unique(top),
                blob_id: Some(blob),
                size: root.size,
                modified,
            });
            continue;
        }
        let top = names.unique(format!("{top}/"));
        let top = top.trim_end_matches('/').to_string();
        let remaining = (MAX_ENTRIES + 1).saturating_sub(out.len()) as i64;
        let rows: Vec<Row> = sqlx::query_as(
            "WITH RECURSIVE t(id, kind, blob_id, size, updated_at, path, depth) AS ( \
                 SELECT c.id, c.kind, c.blob_id, c.size, c.updated_at, c.name, 1 \
                 FROM nodes c WHERE c.parent_id = $1 AND c.trashed_at IS NULL \
                 UNION ALL \
                 SELECT c.id, c.kind, c.blob_id, c.size, c.updated_at, t.path || '/' || c.name, t.depth + 1 \
                 FROM nodes c JOIN t ON c.parent_id = t.id \
                 WHERE c.trashed_at IS NULL AND t.depth < $3 \
             ) SELECT kind, blob_id, size, updated_at, path FROM t ORDER BY path LIMIT $2",
        )
        .bind(root.id)
        .bind(remaining)
        .bind(MAX_DEPTH)
        .fetch_all(pool)
        .await?;
        out.push(Entry {
            path: format!("{top}/"),
            blob_id: None,
            size: 0,
            modified: Utc::now(),
        });
        // Raw folder path -> final directory path in the archive. `ORDER BY path`
        // lists a folder before everything inside it.
        let mut dirs: HashMap<String, String> = HashMap::new();
        for r in rows {
            let (parent_raw, last) = match r.path.rsplit_once('/') {
                Some((p, l)) => (Some(p.to_string()), l),
                None => (None, r.path.as_str()),
            };
            let parent_final = match &parent_raw {
                Some(p) => dirs.get(p).cloned().unwrap_or_else(|| top.clone()),
                None => top.clone(),
            };
            let is_dir = r.kind == "folder";
            let wanted = format!(
                "{parent_final}/{}{}",
                safe_component(last),
                if is_dir { "/" } else { "" }
            );
            let path = names.unique(wanted);
            if is_dir {
                dirs.insert(r.path.clone(), path.trim_end_matches('/').to_string());
            }
            out.push(Entry {
                path,
                blob_id: if is_dir { None } else { r.blob_id },
                size: if is_dir { 0 } else { r.size },
                modified: r.updated_at,
            });
            if out.len() > MAX_ENTRIES {
                return Err(CoreError::bad(format!(
                    "too many items to archive (limit {MAX_ENTRIES})"
                )));
            }
        }
    }
    if out.len() > MAX_ENTRIES {
        return Err(CoreError::bad(format!(
            "too many items to archive (limit {MAX_ENTRIES})"
        )));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_are_made_safe() {
        assert_eq!(safe_component("a/b"), "a_b");
        assert_eq!(safe_component(".."), "_");
        assert_eq!(safe_component(" x "), "x");
        assert_eq!(safe_component("é.txt"), "é.txt");
    }

    #[test]
    fn duplicates_get_numbered_before_the_extension() {
        let mut n = Names(HashSet::new());
        assert_eq!(n.unique("d/a.txt".into()), "d/a.txt");
        assert_eq!(n.unique("d/A.txt".into()), "d/A (2).txt");
        assert_eq!(n.unique("d/a.txt".into()), "d/a (3).txt");
        assert_eq!(n.unique("d/x/".into()), "d/x/");
        assert_eq!(n.unique("d/x/".into()), "d/x (2)/");
    }
}
