// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Thumbnails, checksums and text extraction: the work done by the worker.
//!
//! Decoding is bounded (pixel and allocation limits) and runs on blocking
//! threads so a hostile image cannot exhaust memory or stall the async runtime.

use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::storage::Storage;
use anyhow::Context;
use image::{ImageReader, Limits};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

const MAX_PIXELS: u64 = 120_000_000;
const MAX_DECODE_ALLOC: u64 = 768 * 1024 * 1024;
const TEXT_INDEX_BYTES: usize = 256 * 1024;

/// Build the thumbnail of a blob and record the result in `blobs.thumb_state`.
/// Unsupported or undecodable files are marked so the UI shows a type icon
/// instead of retrying forever.
pub async fn make_thumbnail(pool: &Pool, storage: &Storage, blob: Uuid, max_px: u32) -> Result<()> {
    let mime: Option<String> = sqlx::query_scalar("SELECT mime FROM blobs WHERE id = $1")
        .bind(blob)
        .fetch_optional(pool)
        .await?;
    let Some(mime) = mime else { return Ok(()) }; // deleted meanwhile
    let src = storage.blob_path(blob);
    let dest = storage.thumb_path(blob);
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = dest.with_extension("jpg.tmp");

    let result = if mime.starts_with("video/") {
        video_frame(&src, &tmp, max_px).await
    } else if mime.starts_with("image/") {
        let (s, t) = (src.clone(), tmp.clone());
        tokio::task::spawn_blocking(move || image_thumb(&s, &t, max_px))
            .await
            .map_err(|e| anyhow::anyhow!(e))?
    } else {
        sqlx::query("UPDATE blobs SET thumb_state = 'unsupported' WHERE id = $1")
            .bind(blob)
            .execute(pool)
            .await?;
        return Ok(());
    };

    match result {
        Ok(()) => {
            tokio::fs::rename(&tmp, &dest).await?;
            sqlx::query("UPDATE blobs SET thumb_state = 'ready' WHERE id = $1")
                .bind(blob)
                .execute(pool)
                .await?;
            Ok(())
        }
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            tracing::warn!(%blob, error = %e, "thumbnail failed");
            sqlx::query("UPDATE blobs SET thumb_state = 'failed' WHERE id = $1")
                .bind(blob)
                .execute(pool)
                .await?;
            Ok(())
        }
    }
}

fn image_thumb(src: &Path, dest: &Path, max_px: u32) -> anyhow::Result<()> {
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    let mut reader = ImageReader::open(src)?.with_guessed_format()?;
    reader.limits(limits);
    let (w, h) = reader
        .into_dimensions()
        .context("cannot read image dimensions")?;
    if (w as u64) * (h as u64) > MAX_PIXELS {
        anyhow::bail!("image too large ({w}x{h})");
    }
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    let mut reader = ImageReader::open(src)?.with_guessed_format()?;
    reader.limits(limits);
    let img = reader.decode().context("decode failed")?;
    let thumb = img.thumbnail(max_px, max_px).to_rgb8();
    let mut out = std::fs::File::create(dest)?;
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 80);
    thumb.write_with_encoder(enc)?;
    Ok(())
}

async fn video_frame(src: &Path, dest: &Path, max_px: u32) -> anyhow::Result<()> {
    let scale = format!("scale='min({max_px},iw)':-2");
    let run = |seek: &str| {
        let mut cmd = tokio::process::Command::new("ffmpeg");
        cmd.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-ss",
            seek,
            "-i",
        ])
        .arg(src)
        .args([
            "-frames:v",
            "1",
            "-vf",
            &scale,
            "-f",
            "image2",
            "-c:v",
            "mjpeg",
        ])
        .arg(dest)
        .kill_on_drop(true);
        cmd
    };
    // Try one second in, fall back to the very first frame for short clips.
    for seek in ["1", "0"] {
        let out = tokio::time::timeout(std::time::Duration::from_secs(60), run(seek).output())
            .await
            .map_err(|_| anyhow::anyhow!("ffmpeg timed out"))?
            .context("ffmpeg is not available")?;
        if out.status.success()
            && tokio::fs::metadata(dest)
                .await
                .map(|m| m.len() > 0)
                .unwrap_or(false)
        {
            return Ok(());
        }
    }
    anyhow::bail!("ffmpeg could not extract a frame")
}

/// SHA-256 of a blob, streamed in 1 MiB steps.
pub async fn compute_checksum(pool: &Pool, storage: &Storage, blob: Uuid) -> Result<()> {
    let path: PathBuf = storage.blob_path(blob);
    let mut f = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = f.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    sqlx::query("UPDATE blobs SET sha256 = $2 WHERE id = $1")
        .bind(blob)
        .bind(hex::encode(hasher.finalize()))
        .execute(pool)
        .await?;
    Ok(())
}

/// Extract the first bytes of a text-like file into the full-text index.
pub async fn index_text(pool: &Pool, storage: &Storage, blob: Uuid) -> Result<()> {
    let mut f = match tokio::fs::File::open(storage.blob_path(blob)).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let mut buf = vec![0u8; TEXT_INDEX_BYTES];
    let mut filled = 0;
    while filled < buf.len() {
        let n = f.read(&mut buf[filled..]).await?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    buf.truncate(filled);
    // Binary data masquerading as text is not worth indexing.
    if buf.iter().take(4096).filter(|b| **b == 0).count() > 0 {
        sqlx::query("UPDATE blobs SET indexed = true WHERE id = $1")
            .bind(blob)
            .execute(pool)
            .await?;
        return Ok(());
    }
    let text = String::from_utf8_lossy(&buf).replace('\0', " ");
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO blob_text (blob_id, tsv) VALUES ($1, to_tsvector('simple', $2)) \
         ON CONFLICT (blob_id) DO UPDATE SET tsv = EXCLUDED.tsv",
    )
    .bind(blob)
    .bind(text)
    .execute(&mut *tx)
    .await
    .map_err(CoreError::from)?;
    sqlx::query("UPDATE blobs SET indexed = true WHERE id = $1")
        .bind(blob)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Delete blob rows nothing references (crash leftovers) and their files.
pub async fn gc_orphan_blobs(pool: &Pool, storage: &Storage) -> Result<u64> {
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "DELETE FROM blobs b WHERE NOT EXISTS (SELECT 1 FROM nodes n WHERE n.blob_id = b.id) \
         AND NOT EXISTS (SELECT 1 FROM node_versions v WHERE v.blob_id = b.id) \
         AND b.created_at < now() - interval '1 hour' RETURNING b.id",
    )
    .fetch_all(pool)
    .await?;
    for id in &ids {
        storage.remove_blob(*id).await;
    }
    Ok(ids.len() as u64)
}
