// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

use std::path::PathBuf;

/// Runtime configuration, read from environment variables.
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub data_dir: PathBuf,
    pub bind: String,
    /// Hard limit for a single file, in bytes (0 = unlimited).
    pub max_file_bytes: u64,
    pub session_days: i64,
    /// Set the `Secure` flag on the session cookie (enable behind HTTPS).
    pub cookie_secure: bool,
    pub db_max_connections: u32,
    pub worker_concurrency: usize,
    pub thumb_max_px: u32,
    /// Uploads untouched for this long are discarded.
    pub upload_ttl_hours: i64,
    /// Items stay in the trash this many days before being purged.
    pub trash_days: i64,
    /// Daily database dumps (`pg_dump`, worker image) and how many to keep.
    pub backups_enabled: bool,
    pub backup_keep: usize,
    /// Older versions kept for each file.
    pub max_versions: i64,
    /// Simultaneous .zip downloads the API serves.
    pub max_zip_streams: usize,
    /// Trust `X-Forwarded-For` / `X-Real-IP` (only when behind your own reverse proxy).
    pub trust_proxy: bool,
    /// Skip the startup volume check (see `Storage::verify_mounts`). Dangerous.
    pub ignore_mount_check: bool,
    /// Shared secret with the ONLYOFFICE Document Server. Editing Office files
    /// in the browser is enabled only when it is set.
    pub office_jwt_secret: Option<String>,
    /// Where the server reaches the Document Server (internal address).
    pub office_server_url: String,
    /// Where the Document Server reaches this API (internal address).
    pub office_app_url: String,
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL is required"))?;
        Ok(Self {
            database_url,
            data_dir: PathBuf::from(env_or("DRIVICH_DATA_DIR", "./data")),
            bind: env_or("DRIVICH_BIND", "0.0.0.0:2283"),
            max_file_bytes: env_parse("DRIVICH_MAX_FILE_BYTES", 0u64),
            session_days: env_parse("DRIVICH_SESSION_DAYS", 30i64),
            cookie_secure: env_parse("DRIVICH_COOKIE_SECURE", false),
            db_max_connections: env_parse("DRIVICH_DB_MAX_CONNECTIONS", 10u32),
            worker_concurrency: env_parse("DRIVICH_WORKER_CONCURRENCY", 2usize).max(1),
            thumb_max_px: env_parse("DRIVICH_THUMB_MAX_PX", 480u32).clamp(64, 2048),
            upload_ttl_hours: env_parse("DRIVICH_UPLOAD_TTL_HOURS", 48i64),
            trash_days: env_parse("DRIVICH_TRASH_DAYS", 30i64),
            backups_enabled: env_parse("DRIVICH_BACKUPS_ENABLED", true),
            backup_keep: env_parse("DRIVICH_BACKUP_KEEP", 14usize).max(1),
            max_versions: env_parse("DRIVICH_MAX_VERSIONS", 20i64).clamp(1, 500),
            max_zip_streams: env_parse("DRIVICH_MAX_ZIP_STREAMS", 4usize).clamp(1, 64),
            trust_proxy: env_parse("DRIVICH_TRUST_PROXY", false),
            ignore_mount_check: env_parse("DRIVICH_IGNORE_MOUNT_CHECK_ERRORS", false),
            office_jwt_secret: std::env::var("DRIVICH_ONLYOFFICE_JWT_SECRET")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| v.len() >= 16),
            office_server_url: env_or("DRIVICH_ONLYOFFICE_URL", "http://onlyoffice:80")
                .trim_end_matches('/')
                .to_string(),
            office_app_url: env_or("DRIVICH_ONLYOFFICE_APP_URL", "http://server:2283")
                .trim_end_matches('/')
                .to_string(),
        })
    }
}
