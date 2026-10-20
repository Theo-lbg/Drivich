// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Instance settings stored as JSON in the `settings` table.

use crate::db::Pool;
use crate::error::Result;
use serde_json::Value;

pub async fn get(pool: &Pool, key: &str) -> Result<Option<Value>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM settings WHERE key = $1")
            .bind(key)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn set(pool: &Pool, key: &str, value: &Value) -> Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES ($1, $2) \
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_bool(pool: &Pool, key: &str, default: bool) -> Result<bool> {
    Ok(get(pool, key)
        .await?
        .and_then(|v| v.as_bool())
        .unwrap_or(default))
}

pub async fn get_i64(pool: &Pool, key: &str) -> Result<Option<i64>> {
    Ok(get(pool, key).await?.and_then(|v| v.as_i64()))
}

pub const ALLOW_SIGNUP: &str = "allow_signup";
pub const DEFAULT_QUOTA: &str = "default_quota_bytes";
pub const IMMICH_URL: &str = "immich_url";

/// The Immich address to show to `user`: their own, else the server default.
pub async fn effective_immich_url(pool: &Pool, personal: Option<&str>) -> Result<Option<String>> {
    if let Some(u) = personal.filter(|s| !s.is_empty()) {
        return Ok(Some(u.to_string()));
    }
    Ok(get(pool, IMMICH_URL)
        .await?
        .and_then(|v| v.as_str().map(String::from))
        .filter(|s| !s.is_empty()))
}
