// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Profile pictures.
//!
//! Uploads are never stored as received: the image is decoded under strict
//! limits, cropped to a centred square, resized to 256x256 and re-encoded as a
//! JPEG by our own encoder. That removes metadata (EXIF, GPS), neutralises
//! malformed or hostile files, and keeps every avatar tiny (a few KB).

use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::storage::Storage;
use image::imageops::FilterType;
use image::{DynamicImage, ImageReader, Limits, Rgb, RgbImage};
use std::io::Cursor;
use uuid::Uuid;

/// Largest accepted upload.
pub const MAX_BYTES: usize = 5 * 1024 * 1024;
/// Side of the stored square picture, in pixels.
pub const SIZE: u32 = 256;
const MAX_PIXELS: u64 = 60_000_000;
const MAX_DECODE_ALLOC: u64 = 512 * 1024 * 1024;

fn limits() -> Limits {
    let mut l = Limits::default();
    l.max_alloc = Some(MAX_DECODE_ALLOC);
    l
}

/// Decode, centre-crop, resize and re-encode. CPU-bound: call from a blocking thread.
pub fn process(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.is_empty() {
        return Err(CoreError::bad("empty image"));
    }
    if bytes.len() > MAX_BYTES {
        return Err(CoreError::TooLarge);
    }
    let reader = |b: &[u8]| -> Result<_> {
        let mut r = ImageReader::new(Cursor::new(b.to_vec()))
            .with_guessed_format()
            .map_err(|_| CoreError::bad("unsupported image"))?;
        r.limits(limits());
        Ok(r)
    };
    let (w, h) = reader(bytes)?
        .into_dimensions()
        .map_err(|_| CoreError::bad("this file is not a supported image (JPEG, PNG, GIF, WebP)"))?;
    if (w as u64) * (h as u64) > MAX_PIXELS {
        return Err(CoreError::bad("image too large (max 60 megapixels)"));
    }
    let img = reader(bytes)?
        .decode()
        .map_err(|_| CoreError::bad("this image cannot be read"))?;

    let side = w.min(h);
    let square = img
        .crop_imm((w - side) / 2, (h - side) / 2, side, side)
        .resize_exact(SIZE, SIZE, FilterType::Lanczos3);

    // Flatten transparency on white: JPEG has no alpha channel.
    let rgba = square.to_rgba8();
    let mut flat = RgbImage::new(SIZE, SIZE);
    for (x, y, p) in rgba.enumerate_pixels() {
        let a = p[3] as u32;
        let mix = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
        flat.put_pixel(x, y, Rgb([mix(p[0]), mix(p[1]), mix(p[2])]));
    }

    let mut out = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88);
    DynamicImage::ImageRgb8(flat)
        .write_with_encoder(enc)
        .map_err(|e| CoreError::Internal(anyhow::anyhow!("avatar encoding failed: {e}")))?;
    Ok(out)
}

/// Store a new profile picture; returns the new version number.
pub async fn set(pool: &Pool, storage: &Storage, user: Uuid, bytes: Vec<u8>) -> Result<i64> {
    let jpeg = tokio::task::spawn_blocking(move || process(&bytes))
        .await
        .map_err(|e| CoreError::Internal(anyhow::anyhow!(e)))??;
    let dest = storage.avatar_path(user);
    if let Some(dir) = dest.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    let tmp = dest.with_extension("jpg.tmp");
    tokio::fs::write(&tmp, &jpeg).await?;
    tokio::fs::rename(&tmp, &dest).await?;
    Ok(
        sqlx::query_scalar("UPDATE users SET avatar_version = avatar_version + 1 WHERE id = $1 RETURNING avatar_version")
            .bind(user)
            .fetch_one(pool)
            .await?,
    )
}

/// Remove the profile picture (back to the initial letter).
pub async fn remove(pool: &Pool, storage: &Storage, user: Uuid) -> Result<()> {
    sqlx::query("UPDATE users SET avatar_version = 0 WHERE id = $1")
        .bind(user)
        .execute(pool)
        .await?;
    if let Err(e) = tokio::fs::remove_file(storage.avatar_path(user)).await {
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(e.into());
        }
    }
    Ok(())
}

/// Bytes of a user's picture with its version, or `NotFound` when none is set.
pub async fn read(pool: &Pool, storage: &Storage, user: Uuid) -> Result<(Vec<u8>, i64)> {
    let version: i64 =
        sqlx::query_scalar("SELECT avatar_version FROM users WHERE id = $1 AND NOT disabled")
            .bind(user)
            .fetch_one(pool)
            .await?;
    if version == 0 {
        return Err(CoreError::NotFound);
    }
    Ok((tokio::fs::read(storage.avatar_path(user)).await?, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32, alpha: u8) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, alpha]));
        let mut b = Vec::new();
        DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut b), image::ImageFormat::Png)
            .unwrap();
        b
    }

    #[test]
    fn wide_images_are_cropped_to_a_256_square_jpeg() {
        let out = process(&png(900, 300, 255)).unwrap();
        assert!(out.starts_with(&[0xFF, 0xD8, 0xFF]), "output is a JPEG");
        let back = image::load_from_memory(&out).unwrap();
        assert_eq!((back.width(), back.height()), (SIZE, SIZE));
        assert!(out.len() < 40_000);
    }

    #[test]
    fn transparency_is_flattened_on_white() {
        let out = process(&png(64, 64, 0)).unwrap();
        let px = image::load_from_memory(&out)
            .unwrap()
            .to_rgb8()
            .get_pixel(128, 128)
            .0;
        assert!(
            px.iter().all(|c| *c > 240),
            "fully transparent becomes white, got {px:?}"
        );
    }

    #[test]
    fn garbage_and_oversized_input_is_rejected() {
        assert!(process(b"").is_err());
        assert!(
            process(b"<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script></svg>")
                .is_err()
        );
        assert!(process(b"definitely not an image").is_err());
        assert!(matches!(
            process(&vec![0u8; MAX_BYTES + 1]),
            Err(CoreError::TooLarge)
        ));
    }
}
