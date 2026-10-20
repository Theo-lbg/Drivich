// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Office documents (Word, Excel, PowerPoint, OpenDocument, RTF) are shown in the
//! viewer as a PDF rendition produced on demand by the worker with LibreOffice.
//!
//! The conversion is a background job started the first time somebody opens
//! the file. It runs in the worker, which has no network access in the
//! default compose files, one document at a time, with a time limit and
//! size caps. Without LibreOffice in the worker image the state becomes
//! `unsupported` and the viewer offers the download instead.

use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::jobs;
use crate::models::Role;
use crate::perms;
use crate::storage::Storage;
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

/// Largest document sent to the converter.
pub const MAX_INPUT_BYTES: i64 = 100 * 1024 * 1024;
const MAX_OUTPUT_BYTES: u64 = 300 * 1024 * 1024;
const CONVERT_TIMEOUT: Duration = Duration::from_secs(180);

pub const MIMES: &[&str] = &[
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.ms-excel",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.ms-powerpoint",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/vnd.oasis.opendocument.text",
    "application/vnd.oasis.opendocument.spreadsheet",
    "application/vnd.oasis.opendocument.presentation",
    "application/rtf",
    "text/rtf",
];

pub fn is_office(mime: &str) -> bool {
    MIMES.contains(&mime)
}

/// Extension LibreOffice needs to recognise the input, from the MIME type
/// (never from the user-supplied file name).
fn extension(mime: &str) -> &'static str {
    match mime {
        "application/msword" => "doc",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.ms-excel" => "xls",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "application/vnd.ms-powerpoint" => "ppt",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",
        "application/vnd.oasis.opendocument.text" => "odt",
        "application/vnd.oasis.opendocument.spreadsheet" => "ods",
        "application/vnd.oasis.opendocument.presentation" => "odp",
        _ => "rtf",
    }
}

/// State of the PDF rendition of a file: `none`, `pending`, `ready`, `failed`
/// or `unsupported`. Starts the conversion when it has not been requested yet.
pub async fn request(pool: &Pool, user: Uuid, node: Uuid) -> Result<String> {
    let (n, _) = perms::authorize(pool, user, node, Role::Viewer).await?;
    let (Some(blob), Some(mime)) = (n.blob_id, n.mime.as_deref()) else {
        return Err(CoreError::bad("this item is a folder"));
    };
    if !is_office(mime) || n.size > MAX_INPUT_BYTES {
        return Ok("unsupported".into());
    }
    let state: String = sqlx::query_scalar("SELECT preview_state FROM blobs WHERE id = $1")
        .bind(blob)
        .fetch_one(pool)
        .await?;
    if state != "none" {
        return Ok(state);
    }
    // Only the first caller flips none -> pending and enqueues.
    let flipped = sqlx::query(
        "UPDATE blobs SET preview_state = 'pending' WHERE id = $1 AND preview_state = 'none'",
    )
    .bind(blob)
    .execute(pool)
    .await?
    .rows_affected();
    if flipped > 0 {
        jobs::enqueue_blob(pool, jobs::kind::OFFICE_PREVIEW, blob).await?;
    }
    Ok("pending".into())
}

pub async fn state_of(pool: &Pool, blob: Uuid) -> Result<String> {
    Ok(
        sqlx::query_scalar("SELECT preview_state FROM blobs WHERE id = $1")
            .bind(blob)
            .fetch_one(pool)
            .await?,
    )
}

async fn set_state(pool: &Pool, blob: Uuid, state: &str) -> Result<()> {
    sqlx::query("UPDATE blobs SET preview_state = $2 WHERE id = $1")
        .bind(blob)
        .bind(state)
        .execute(pool)
        .await?;
    Ok(())
}

/// Is LibreOffice installed in this image?
pub async fn converter_available() -> bool {
    tokio::process::Command::new("soffice")
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Worker start-up: previews marked `unsupported` only because the converter
/// was missing become requestable again once it is installed.
pub async fn reset_unsupported(pool: &Pool) -> Result<u64> {
    let mimes: Vec<String> = MIMES.iter().map(|m| m.to_string()).collect();
    Ok(sqlx::query(
        "UPDATE blobs SET preview_state = 'none' WHERE preview_state = 'unsupported' AND mime = ANY($1)",
    )
    .bind(&mimes)
    .execute(pool)
    .await?
    .rows_affected())
}

/// Convert one blob. Never retried by the queue: the outcome is stored in
/// `preview_state` (`ready`, `failed`, `unsupported`).
pub async fn generate(pool: &Pool, storage: &Storage, blob: Uuid) -> Result<()> {
    let row: Option<(String, i64)> = sqlx::query_as("SELECT mime, size FROM blobs WHERE id = $1")
        .bind(blob)
        .fetch_optional(pool)
        .await?;
    let Some((mime, size)) = row else {
        return Ok(()); // deleted meanwhile
    };
    if !is_office(&mime) || size > MAX_INPUT_BYTES {
        return set_state(pool, blob, "unsupported").await;
    }
    if !converter_available().await {
        return set_state(pool, blob, "unsupported").await;
    }
    match convert(storage, blob, &mime).await {
        Ok(()) => set_state(pool, blob, "ready").await,
        Err(e) => {
            tracing::warn!(%blob, error = %e, "office preview failed");
            let _ = tokio::fs::remove_file(storage.preview_path(blob)).await;
            set_state(pool, blob, "failed").await
        }
    }
}

async fn convert(storage: &Storage, blob: Uuid, mime: &str) -> anyhow::Result<()> {
    let work = storage
        .office_tmp_dir()
        .join(Uuid::new_v4().simple().to_string());
    tokio::fs::create_dir_all(&work).await?;
    let result = convert_in(storage, blob, mime, &work).await;
    let _ = tokio::fs::remove_dir_all(&work).await;
    result
}

async fn convert_in(storage: &Storage, blob: Uuid, mime: &str, work: &Path) -> anyhow::Result<()> {
    let input = work.join(format!("document.{}", extension(mime)));
    tokio::fs::copy(storage.blob_path(blob), &input).await?;
    let profile = work.join("profile");
    let mut cmd = tokio::process::Command::new("soffice");
    cmd.arg("--headless")
        .arg("--norestore")
        .arg("--nolockcheck")
        .arg("--nodefault")
        .arg("--nofirststartwizard")
        .arg(format!(
            "-env:UserInstallation=file://{}",
            profile.display()
        ))
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(work)
        .arg(&input)
        .env("HOME", work)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let status = tokio::time::timeout(CONVERT_TIMEOUT, cmd.status())
        .await
        .map_err(|_| anyhow::anyhow!("conversion timed out"))??;
    if !status.success() {
        anyhow::bail!("converter exited with {status}");
    }
    let output = work.join("document.pdf");
    let meta = tokio::fs::metadata(&output).await?;
    if meta.len() == 0 || meta.len() > MAX_OUTPUT_BYTES {
        anyhow::bail!("unusable output ({} bytes)", meta.len());
    }
    let mut head = [0u8; 5];
    {
        use tokio::io::AsyncReadExt;
        let mut f = tokio::fs::File::open(&output).await?;
        f.read_exact(&mut head).await?;
    }
    if &head != b"%PDF-" {
        anyhow::bail!("output is not a PDF");
    }
    let dest = storage.preview_path(blob);
    if let Some(dir) = dest.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    let tmp = dest.with_extension("pdf.part");
    tokio::fs::copy(&output, &tmp).await?;
    tokio::fs::rename(&tmp, &dest).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn office_types_are_recognised() {
        assert!(is_office(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        ));
        assert!(is_office("application/rtf"));
        assert!(!is_office("application/pdf"));
        assert!(!is_office("text/plain"));
        assert_eq!(extension("application/vnd.ms-excel"), "xls");
    }
}
