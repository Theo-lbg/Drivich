// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! On-disk layout. File bytes are addressed by blob id, never by user-chosen
//! names, so renames and moves never touch the disk and no path can escape the
//! data directory.
//!
//! ```text
//! <data>/files/ab/cd/<blob-uuid>        original bytes
//! <data>/cache/thumbs/ab/<blob-uuid>.jpg  generated thumbnails
//! <data>/tmp/uploads/<upload-uuid>.part   in-flight uploads
//! <data>/avatars/<user-uuid>.jpg          profile pictures (256x256 JPEG)
//! ```

use crate::error::{CoreError, Result};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Storage {
    root: PathBuf,
}

impl Storage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Create the directory skeleton.
    pub async fn init(&self) -> Result<()> {
        for d in ["files", "cache/thumbs", "tmp/uploads", "avatars", "backups"] {
            tokio::fs::create_dir_all(self.root.join(d)).await?;
        }
        Ok(())
    }

    /// Startup safety check, modelled on Immich's mount check.
    ///
    /// Each data folder gets a hidden `.drivich` marker that is written and read
    /// back (this proves the volume is mounted and writable by this user). The
    /// dangerous case is refused: if the database already references stored
    /// files but `files/` is empty and has no marker, the volume is most likely
    /// not mounted, and starting would silently create a second, empty library.
    pub async fn verify_mounts(&self, db_has_files: bool, ignore: bool) -> Result<()> {
        match self.check_mounts(db_has_files).await {
            Ok(()) => Ok(()),
            Err(e) if ignore => {
                tracing::warn!(error = %e, "mount check failed but DRIVICH_IGNORE_MOUNT_CHECK_ERRORS is set");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    async fn check_mounts(&self, db_has_files: bool) -> Result<()> {
        const MARKER: &str = ".drivich";
        for d in ["files", "cache", "tmp", "avatars", "backups"] {
            let dir = self.root.join(d);
            let marker = dir.join(MARKER);
            let had_marker = tokio::fs::try_exists(&marker).await.unwrap_or(false);
            if !had_marker && d == "files" && db_has_files && dir_is_empty(&dir).await? {
                return Err(CoreError::Internal(anyhow::anyhow!(
                    "the database references stored files but {} is empty and has no .drivich marker: \
                     the storage volume is probably not mounted. Refusing to start so no second, empty \
                     library gets created. Check UPLOAD_LOCATION, or set \
                     DRIVICH_IGNORE_MOUNT_CHECK_ERRORS=true if you know what you are doing.",
                    dir.display()
                )));
            }
            // Write, read back and overwrite: verifies permissions as well as the mount.
            let stamp = crate::VERSION.as_bytes();
            let ctx =
                |what: &str| format!("storage check failed: cannot {what} {}", marker.display());
            tokio::fs::write(&marker, stamp)
                .await
                .map_err(|e| CoreError::Internal(anyhow::anyhow!("{}: {e}", ctx("write"))))?;
            let back = tokio::fs::read(&marker)
                .await
                .map_err(|e| CoreError::Internal(anyhow::anyhow!("{}: {e}", ctx("read"))))?;
            if back != stamp {
                return Err(CoreError::Internal(anyhow::anyhow!(
                    "{}: read-back differs",
                    ctx("verify")
                )));
            }
        }
        Ok(())
    }

    pub fn blob_path(&self, id: Uuid) -> PathBuf {
        let s = id.simple().to_string();
        self.root
            .join("files")
            .join(&s[0..2])
            .join(&s[2..4])
            .join(&s)
    }

    pub fn thumb_path(&self, id: Uuid) -> PathBuf {
        let s = id.simple().to_string();
        self.root
            .join("cache/thumbs")
            .join(&s[0..2])
            .join(format!("{s}.jpg"))
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    pub fn avatar_path(&self, user: Uuid) -> PathBuf {
        self.root
            .join("avatars")
            .join(format!("{}.jpg", user.simple()))
    }

    pub fn upload_path(&self, id: Uuid) -> PathBuf {
        self.root
            .join("tmp/uploads")
            .join(format!("{}.part", id.simple()))
    }

    /// Move a finished upload into its final place (atomic: same filesystem).
    pub async fn commit_upload(&self, upload: Uuid, blob: Uuid) -> Result<()> {
        let dest = self.blob_path(blob);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::rename(self.upload_path(upload), dest).await?;
        Ok(())
    }

    /// Best-effort removal of a blob and its thumbnail.
    pub async fn remove_blob(&self, id: Uuid) {
        for p in [self.blob_path(id), self.thumb_path(id)] {
            if let Err(e) = tokio::fs::remove_file(&p).await {
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(path = %p.display(), error = %e, "could not remove file");
                }
            }
        }
    }

    /// Free and total bytes of the filesystem holding the data directory.
    #[cfg(unix)]
    pub fn disk_space(&self) -> Option<(u64, u64)> {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let c = CString::new(self.root.as_os_str().as_bytes()).ok()?;
        let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
        // SAFETY: `c` is a valid NUL-terminated path and `st` is a valid out-pointer.
        let rc = unsafe { libc::statvfs(c.as_ptr(), &mut st) };
        if rc != 0 {
            return None;
        }
        let frsize = st.f_frsize as u64;
        Some((st.f_bavail as u64 * frsize, st.f_blocks as u64 * frsize))
    }

    #[cfg(not(unix))]
    pub fn disk_space(&self) -> Option<(u64, u64)> {
        None
    }
}

/// True when `dir` holds nothing but our own marker file.
async fn dir_is_empty(dir: &Path) -> Result<bool> {
    let mut rd = tokio::fs::read_dir(dir).await?;
    while let Some(e) = rd.next_entry().await? {
        if e.file_name() != ".drivich" {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Best-effort MIME detection: magic bytes first (for the common formats the
/// viewer handles), file extension as fallback.
pub fn detect_mime(name: &str, head: &[u8]) -> String {
    let sniffed = if head.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some("image/webp")
    } else if head.starts_with(b"%PDF-") {
        Some("application/pdf")
    } else {
        None
    };
    if let Some(m) = sniffed {
        return m.to_string();
    }
    mime_guess::from_path(name)
        .first_or_octet_stream()
        .essence_str()
        .to_string()
}

pub fn is_text_like(mime: &str, name: &str) -> bool {
    if mime.starts_with("text/") {
        return true;
    }
    matches!(
        mime,
        "application/json"
            | "application/xml"
            | "application/javascript"
            | "application/x-yaml"
            | "application/toml"
            | "image/svg+xml"
    ) || matches!(
        name.rsplit('.')
            .next()
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some(
            "md" | "rs"
                | "toml"
                | "yaml"
                | "yml"
                | "json"
                | "csv"
                | "log"
                | "ts"
                | "tsx"
                | "js"
                | "svelte"
                | "py"
                | "go"
                | "c"
                | "h"
                | "cpp"
                | "java"
                | "sh"
                | "sql"
                | "ini"
                | "conf"
                | "txt"
        )
    )
}

/// Does `mime` have a thumbnail the worker can build?
pub fn thumbnailable(mime: &str) -> bool {
    matches!(
        mime,
        "image/jpeg" | "image/png" | "image/gif" | "image/webp" | "image/bmp" | "image/tiff"
    ) || mime.starts_with("video/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_storage() -> Storage {
        Storage::new(std::env::temp_dir().join(format!("drivich-mount-{}", Uuid::new_v4())))
    }

    #[tokio::test]
    async fn fresh_install_creates_markers() {
        let s = temp_storage();
        s.init().await.unwrap();
        s.verify_mounts(false, false).await.unwrap();
        for d in ["files", "cache", "tmp", "avatars", "backups"] {
            assert!(s.root().join(d).join(".drivich").exists(), "{d}");
        }
        // A second start is fine too.
        s.verify_mounts(true, false).await.unwrap();
    }

    #[tokio::test]
    async fn missing_volume_is_refused_when_the_database_has_files() {
        let s = temp_storage(); // freshly created = what an unmounted volume looks like
        s.init().await.unwrap();
        let err = s.verify_mounts(true, false).await.unwrap_err().to_string();
        assert!(err.contains("not mounted"), "{err}");
        // The escape hatch works.
        s.verify_mounts(true, true).await.unwrap();
    }

    #[tokio::test]
    async fn existing_library_without_markers_is_adopted() {
        // An install from before the check existed: files present, no markers yet.
        let s = temp_storage();
        s.init().await.unwrap();
        std::fs::create_dir_all(s.root().join("files/ab")).unwrap();
        s.verify_mounts(true, false).await.unwrap();
        assert!(s.root().join("files/.drivich").exists());
    }

    #[test]
    fn blob_paths_are_sharded_and_stay_in_root() {
        let s = Storage::new("/data");
        let id = Uuid::parse_str("0123abcd-0000-4000-8000-000000000000").unwrap();
        let p = s.blob_path(id);
        assert!(p.starts_with("/data/files/01/23/"));
        assert!(!p.to_string_lossy().contains(".."));
    }

    #[test]
    fn mime_sniffing_prefers_magic_over_extension() {
        assert_eq!(
            detect_mime("photo.txt", &[0x89, b'P', b'N', b'G', 0, 0]),
            "image/png"
        );
        assert_eq!(detect_mime("doc.pdf", b"%PDF-1.7"), "application/pdf");
        assert_eq!(detect_mime("notes.md", b"# hi"), "text/markdown");
        assert_eq!(
            detect_mime("unknown.zzzz", b"abc"),
            "application/octet-stream"
        );
    }

    #[test]
    fn text_detection() {
        assert!(is_text_like("text/plain", "a.txt"));
        assert!(is_text_like("application/octet-stream", "main.rs"));
        assert!(!is_text_like("application/zip", "a.zip"));
    }
}
