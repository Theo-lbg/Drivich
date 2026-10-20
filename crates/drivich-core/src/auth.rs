// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Password hashing and server-side sessions.
//!
//! Session tokens are 244 random bits handed to the browser in an HttpOnly
//! cookie; only their SHA-256 is stored, so a database leak does not leak
//! usable sessions.

use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::models::User;
use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash};
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const MIN_PASSWORD_LEN: usize = 10;

pub fn hash_password(password: &str) -> Result<String> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(CoreError::bad(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }
    if password.len() > 1024 {
        return Err(CoreError::bad("password too long"));
    }
    let hash = Argon2::default()
        .hash_password(password.as_bytes())
        .map_err(|e| CoreError::Internal(anyhow::anyhow!("password hashing failed: {e}")))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// A fresh unguessable token: two v4 UUIDs (OS randomness), 64 hex chars.
pub fn random_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Create a session and return the raw token (shown once, to the cookie).
pub async fn create_session(
    pool: &Pool,
    user_id: Uuid,
    days: i64,
    user_agent: Option<&str>,
) -> Result<String> {
    let token = random_token();
    let ua = user_agent.map(|s| s.chars().take(300).collect::<String>());
    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, user_agent, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(hash_token(&token))
    .bind(ua)
    .bind(Utc::now() + Duration::days(days))
    .execute(pool)
    .await?;
    Ok(token)
}

/// Resolve a session token to its (enabled) user. Slides `last_seen_at` at most
/// once per minute to avoid a write on every request.
pub async fn user_for_token(pool: &Pool, token: &str) -> Result<User> {
    let user: Option<User> = sqlx::query_as(
        "UPDATE sessions s SET last_seen_at = CASE WHEN s.last_seen_at < now() - interval '1 minute' \
         THEN now() ELSE s.last_seen_at END \
         FROM users u WHERE s.token_hash = $1 AND s.expires_at > now() AND u.id = s.user_id AND NOT u.disabled \
         RETURNING u.id, u.email, u.name, u.password_hash, u.is_admin, u.disabled, u.quota_bytes, u.immich_url, u.avatar_version, u.locale, u.created_at",
    )
    .bind(hash_token(token))
    .fetch_optional(pool)
    .await?;
    user.ok_or(CoreError::Unauthorized)
}

pub async fn delete_session(pool: &Pool, token: &str) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(hash_token(token))
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_user_sessions(pool: &Pool, user_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_roundtrip() {
        let h = hash_password("correct horse battery").unwrap();
        assert!(verify_password("correct horse battery", &h));
        assert!(!verify_password("wrong password!!", &h));
    }

    #[test]
    fn short_passwords_are_rejected() {
        assert!(hash_password("short").is_err());
    }

    #[test]
    fn tokens_are_unique_and_long() {
        let a = random_token();
        let b = random_token();
        assert_ne!(a, b);
        assert_eq!(a.len(), 64);
        assert_eq!(hash_token(&a).len(), 32);
    }
}
