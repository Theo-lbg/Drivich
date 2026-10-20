// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Automatic database dumps, like Immich's daily dumps.
//!
//! The worker runs `pg_dump` (custom format: compressed and restorable with
//! `pg_restore`) into `<data>/backups/`, keeps the newest N and deletes the
//! rest. The database holds *only metadata*: file contents live in `files/`
//! and must be backed up with it (see the README).

use crate::error::{CoreError, Result};
use crate::storage::Storage;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;

const PREFIX: &str = "drivich-";
const SUFFIX: &str = ".dump";

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Backup {
    pub name: String,
    pub size: u64,
    pub created_at: DateTime<Utc>,
}

/// Only names we generate ourselves are ever served or deleted
/// (`drivich-YYYYMMDD-HHMMSS.dump`): no path can be smuggled in.
pub fn is_valid_name(name: &str) -> bool {
    let Some(mid) = name
        .strip_prefix(PREFIX)
        .and_then(|n| n.strip_suffix(SUFFIX))
    else {
        return false;
    };
    let b = mid.as_bytes();
    mid.len() == 15
        && b[8] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 8 || c.is_ascii_digit())
}

pub fn backup_path(storage: &Storage, name: &str) -> Result<PathBuf> {
    if !is_valid_name(name) {
        return Err(CoreError::NotFound);
    }
    Ok(storage.backups_dir().join(name))
}

/// Existing dumps, newest first.
pub async fn list(storage: &Storage) -> Result<Vec<Backup>> {
    let mut out = Vec::new();
    let mut rd = match tokio::fs::read_dir(storage.backups_dir()).await {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };
    while let Some(e) = rd.next_entry().await? {
        let name = e.file_name().to_string_lossy().to_string();
        if !is_valid_name(&name) {
            continue;
        }
        let meta = e.metadata().await?;
        let created = meta
            .modified()
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(|_| Utc::now());
        out.push(Backup {
            name,
            size: meta.len(),
            created_at: created,
        });
    }
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

/// Run `pg_dump` against `database_url` and return the new dump's name.
/// The password never appears on a command line: connection settings go
/// through `PG*` environment variables of the child process.
pub async fn create(storage: &Storage, database_url: &str) -> Result<String> {
    let url = url::Url::parse(database_url)
        .map_err(|_| CoreError::Internal(anyhow::anyhow!("DATABASE_URL is not a valid URL")))?;
    let name = format!("{PREFIX}{}{SUFFIX}", Utc::now().format("%Y%m%d-%H%M%S"));
    let dir = storage.backups_dir();
    tokio::fs::create_dir_all(&dir).await?;
    let dest = dir.join(&name);
    let tmp = dir.join(format!("{name}.tmp"));

    let decode = |s: &str| {
        percent_encoding::percent_decode_str(s)
            .decode_utf8_lossy()
            .to_string()
    };
    let mut cmd = tokio::process::Command::new("pg_dump");
    cmd.args(["--format=custom", "--no-owner", "--file"])
        .arg(&tmp)
        .env("PGHOST", url.host_str().unwrap_or("localhost"))
        .env("PGPORT", url.port().unwrap_or(5432).to_string())
        .env("PGUSER", decode(url.username()))
        .env("PGPASSWORD", decode(url.password().unwrap_or("")))
        .env("PGDATABASE", url.path().trim_start_matches('/'))
        .kill_on_drop(true);
    let out = tokio::time::timeout(Duration::from_secs(30 * 60), cmd.output())
        .await
        .map_err(|_| CoreError::Internal(anyhow::anyhow!("pg_dump timed out")))?
        .map_err(|e| CoreError::Internal(anyhow::anyhow!("pg_dump could not be started (is the PostgreSQL client installed in this image?): {e}")))?;
    if !out.status.success() {
        let _ = tokio::fs::remove_file(&tmp).await;
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(CoreError::Internal(anyhow::anyhow!(
            "pg_dump failed: {}",
            stderr.trim()
        )));
    }
    tokio::fs::rename(&tmp, &dest).await?;
    Ok(name)
}

/// Keep the newest `keep` dumps, delete the others. Returns how many were deleted.
pub async fn prune(storage: &Storage, keep: usize) -> Result<usize> {
    let all = list(storage).await?;
    let mut removed = 0;
    for b in all.into_iter().skip(keep.max(1)) {
        if tokio::fs::remove_file(storage.backups_dir().join(&b.name))
            .await
            .is_ok()
        {
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn only_our_own_file_names_are_valid() {
        assert!(is_valid_name("drivich-20261020-021500.dump"));
        for bad in [
            "../drivich-20261020-021500.dump",
            "drivich-20261020-021500.dump/../x",
            "drivich-2026102-021500.dump",
            "drivich-20261020-021500.sql",
            "other-20261020-021500.dump",
            "drivich-20261020_021500.dump",
            "drivich-20261020-02150a.dump",
            "",
        ] {
            assert!(!is_valid_name(bad), "{bad}");
        }
        let s = Storage::new("/data");
        assert!(backup_path(&s, "../etc/passwd").is_err());
    }

    #[tokio::test]
    async fn newest_dumps_are_kept_and_the_rest_pruned() {
        let s = Storage::new(std::env::temp_dir().join(format!("drivich-bk-{}", Uuid::new_v4())));
        tokio::fs::create_dir_all(s.backups_dir()).await.unwrap();
        for d in 10..=16 {
            tokio::fs::write(
                s.backups_dir()
                    .join(format!("drivich-202610{d}-020000.dump")),
                b"x",
            )
            .await
            .unwrap();
        }
        tokio::fs::write(s.backups_dir().join("notes.txt"), b"keep me")
            .await
            .unwrap();
        assert_eq!(list(&s).await.unwrap().len(), 7);
        assert_eq!(prune(&s, 3).await.unwrap(), 4);
        let left: Vec<String> = list(&s)
            .await
            .unwrap()
            .into_iter()
            .map(|b| b.name)
            .collect();
        assert_eq!(
            left,
            [
                "drivich-20261016-020000.dump",
                "drivich-20261015-020000.dump",
                "drivich-20261014-020000.dump"
            ]
        );
        assert!(
            s.backups_dir().join("notes.txt").exists(),
            "foreign files are never touched"
        );
    }
}
