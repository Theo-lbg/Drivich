// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

use crate::auth;
use crate::db::Pool;
use crate::error::{CoreError, Result};
use crate::models::{User, UserBrief};
use uuid::Uuid;

const USER_COLS: &str = "id, email, name, password_hash, is_admin, disabled, quota_bytes, immich_url, avatar_version, locale, created_at";

pub fn normalize_email(email: &str) -> Result<String> {
    let e = email.trim().to_lowercase();
    let ok = e.len() <= 254
        && e.matches('@').count() == 1
        && !e.starts_with('@')
        && !e.ends_with('@')
        && e.split('@')
            .nth(1)
            .is_some_and(|d| d.contains('.') && !d.ends_with('.'))
        && !e.chars().any(|c| c.is_whitespace() || c.is_control());
    if ok {
        Ok(e)
    } else {
        Err(CoreError::bad("invalid email address"))
    }
}

fn clean_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 80 || n.chars().any(|c| c.is_control()) {
        return Err(CoreError::bad("invalid name"));
    }
    Ok(n.to_string())
}

pub async fn count(pool: &Pool) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(pool)
        .await?)
}

pub async fn create(
    pool: &Pool,
    email: &str,
    name: &str,
    password: &str,
    is_admin: bool,
    quota_bytes: Option<i64>,
) -> Result<User> {
    let email = normalize_email(email)?;
    let name = clean_name(name)?;
    let hash = auth::hash_password(password)?;
    let sql = format!(
        "INSERT INTO users (email, name, password_hash, is_admin, quota_bytes) \
         VALUES ($1, $2, $3, $4, $5) RETURNING {USER_COLS}"
    );
    let res = sqlx::query_as::<_, User>(&sql)
        .bind(email)
        .bind(name)
        .bind(hash)
        .bind(is_admin)
        .bind(quota_bytes)
        .fetch_one(pool)
        .await;
    match res {
        Ok(u) => Ok(u),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(CoreError::conflict(
            "an account with this email already exists",
        )),
        Err(e) => Err(e.into()),
    }
}

/// Create the very first account (administrator). Fails if any user exists,
/// atomically, so two concurrent setups cannot both become admin.
pub async fn create_first_admin(
    pool: &Pool,
    email: &str,
    name: &str,
    password: &str,
) -> Result<User> {
    let email = normalize_email(email)?;
    let name = clean_name(name)?;
    let hash = auth::hash_password(password)?;
    let sql = format!(
        "INSERT INTO users (email, name, password_hash, is_admin) \
         SELECT $1, $2, $3, true WHERE NOT EXISTS (SELECT 1 FROM users) RETURNING {USER_COLS}"
    );
    let u: Option<User> = sqlx::query_as(&sql)
        .bind(email)
        .bind(name)
        .bind(hash)
        .fetch_optional(pool)
        .await?;
    u.ok_or_else(|| CoreError::conflict("setup already completed"))
}

pub async fn find_by_email(pool: &Pool, email: &str) -> Result<Option<User>> {
    let sql = format!("SELECT {USER_COLS} FROM users WHERE lower(email) = lower($1)");
    Ok(sqlx::query_as(&sql)
        .bind(email.trim())
        .fetch_optional(pool)
        .await?)
}

pub async fn get(pool: &Pool, id: Uuid) -> Result<User> {
    let sql = format!("SELECT {USER_COLS} FROM users WHERE id = $1");
    Ok(sqlx::query_as(&sql).bind(id).fetch_one(pool).await?)
}

pub async fn list(pool: &Pool) -> Result<Vec<User>> {
    let sql = format!("SELECT {USER_COLS} FROM users ORDER BY created_at");
    Ok(sqlx::query_as(&sql).fetch_all(pool).await?)
}

/// Verify credentials. Always runs a hash verification so timing does not
/// reveal whether the email exists.
pub async fn authenticate(pool: &Pool, email: &str, password: &str) -> Result<User> {
    // A real hash of a throwaway password, computed once, so that unknown
    // emails cost the same time as known ones.
    static DUMMY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let user = find_by_email(pool, email).await?;
    let ok = match &user {
        Some(u) => auth::verify_password(password, &u.password_hash),
        None => {
            let dummy = DUMMY.get_or_init(|| {
                auth::hash_password("timing-equalisation-only").unwrap_or_default()
            });
            let _ = auth::verify_password(password, dummy);
            false
        }
    };
    match user {
        Some(u) if ok && !u.disabled => Ok(u),
        _ => Err(CoreError::Unauthorized),
    }
}

pub async fn lookup(pool: &Pool, q: &str, exclude: Uuid) -> Result<Vec<UserBrief>> {
    let q = q.trim().to_lowercase();
    if q.chars().count() < 2 {
        return Ok(vec![]);
    }
    let like = format!(
        "{}%",
        q.replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    Ok(sqlx::query_as(
        "SELECT id, email, name, avatar_version FROM users WHERE NOT disabled AND id <> $2 \
         AND (lower(email) LIKE $1 OR lower(name) LIKE $1) ORDER BY name LIMIT 8",
    )
    .bind(like)
    .bind(exclude)
    .fetch_all(pool)
    .await?)
}

pub async fn update_password(pool: &Pool, id: Uuid, new_password: &str) -> Result<()> {
    let hash = auth::hash_password(new_password)?;
    sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
        .bind(id)
        .bind(hash)
        .execute(pool)
        .await?;
    Ok(())
}

/// Validate a link to an external app (Immich): plain http(s), no embedded
/// credentials, no control characters. Returns the normalised URL.
pub fn normalize_external_url(raw: &str) -> Result<String> {
    let u = raw.trim();
    let lower = u.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .ok_or_else(|| CoreError::bad("the address must start with http:// or https://"))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if u.len() > 300
        || authority.is_empty()
        || authority.contains('@')
        || u.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(CoreError::bad("invalid address"));
    }
    Ok(u.trim_end_matches('/').to_string())
}

/// Validate a language tag (BCP 47, lenient): `fr`, `en`, `pt-BR`, `zh-Hant`...
/// Adding a translation never needs a backend change.
pub fn normalize_locale(raw: &str) -> Result<String> {
    let l = raw.trim();
    let ok = (2..=12).contains(&l.len())
        && l.split('-')
            .all(|p| !p.is_empty() && p.len() <= 8 && p.chars().all(|c| c.is_ascii_alphanumeric()))
        && l.split('-')
            .next()
            .is_some_and(|p| p.len() <= 3 && p.chars().all(|c| c.is_ascii_alphabetic()));
    if ok {
        Ok(l.to_string())
    } else {
        Err(CoreError::bad("invalid language tag"))
    }
}

/// Set (or clear with `None`) the user's interface language.
pub async fn set_locale(pool: &Pool, id: Uuid, locale: Option<&str>) -> Result<User> {
    let locale = match locale.map(str::trim).filter(|s| !s.is_empty()) {
        Some(l) => Some(normalize_locale(l)?),
        None => None,
    };
    let sql = format!("UPDATE users SET locale = $2 WHERE id = $1 RETURNING {USER_COLS}");
    Ok(sqlx::query_as(&sql)
        .bind(id)
        .bind(locale)
        .fetch_one(pool)
        .await?)
}

/// Set (or clear with `None`) the user's personal Immich address.
pub async fn set_immich_url(pool: &Pool, id: Uuid, url: Option<&str>) -> Result<User> {
    let url = match url.map(str::trim).filter(|s| !s.is_empty()) {
        Some(u) => Some(normalize_external_url(u)?),
        None => None,
    };
    let sql = format!("UPDATE users SET immich_url = $2 WHERE id = $1 RETURNING {USER_COLS}");
    Ok(sqlx::query_as(&sql)
        .bind(id)
        .bind(url)
        .fetch_one(pool)
        .await?)
}

pub async fn update_profile(pool: &Pool, id: Uuid, name: &str) -> Result<User> {
    let name = clean_name(name)?;
    let sql = format!("UPDATE users SET name = $2 WHERE id = $1 RETURNING {USER_COLS}");
    Ok(sqlx::query_as(&sql)
        .bind(id)
        .bind(name)
        .fetch_one(pool)
        .await?)
}

pub struct AdminUpdate {
    pub is_admin: Option<bool>,
    pub disabled: Option<bool>,
    /// `Some(None)` clears the quota (unlimited).
    pub quota_bytes: Option<Option<i64>>,
}

pub async fn admin_update(pool: &Pool, id: Uuid, acting: Uuid, upd: AdminUpdate) -> Result<User> {
    if id == acting && (upd.is_admin == Some(false) || upd.disabled == Some(true)) {
        return Err(CoreError::bad(
            "you cannot remove your own admin rights or disable yourself",
        ));
    }
    let mut tx = pool.begin().await?;
    if let Some(v) = upd.is_admin {
        sqlx::query("UPDATE users SET is_admin = $2 WHERE id = $1")
            .bind(id)
            .bind(v)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(v) = upd.disabled {
        sqlx::query("UPDATE users SET disabled = $2 WHERE id = $1")
            .bind(id)
            .bind(v)
            .execute(&mut *tx)
            .await?;
        if v {
            sqlx::query("DELETE FROM sessions WHERE user_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
    }
    if let Some(q) = upd.quota_bytes {
        if matches!(q, Some(n) if n < 0) {
            return Err(CoreError::bad("quota must be positive"));
        }
        sqlx::query("UPDATE users SET quota_bytes = $2 WHERE id = $1")
            .bind(id)
            .bind(q)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    get(pool, id).await
}

/// Bytes stored by a user (trash included: it still occupies disk).
pub async fn used_bytes(pool: &Pool, user_id: Uuid) -> Result<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(size), 0)::bigint FROM nodes WHERE owner_id = $1 AND kind = 'file'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_urls_are_validated() {
        assert_eq!(
            normalize_external_url(" https://photos.example.org/ ").unwrap(),
            "https://photos.example.org"
        );
        assert_eq!(
            normalize_external_url("http://192.168.1.20:2283").unwrap(),
            "http://192.168.1.20:2283"
        );
        for bad in [
            "javascript:alert(1)",
            "ftp://x.test",
            "photos.example.org",
            "https://user:pw@photos.example.org",
            "https:///nohost",
            "https://a b.test",
            "",
        ] {
            assert!(normalize_external_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn language_tags_are_validated() {
        for ok in ["fr", "en", "pt-BR", "zh-Hant", "es-419"] {
            assert_eq!(normalize_locale(ok).unwrap(), ok);
        }
        for bad in [
            "",
            "f",
            "english-us-way-too-long",
            "fr_FR",
            "fr-",
            "1a",
            "fr--BE",
            "<x>",
        ] {
            assert!(normalize_locale(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn emails_are_validated_and_lowercased() {
        assert_eq!(
            normalize_email("  Foo@Example.COM ").unwrap(),
            "foo@example.com"
        );
        assert!(normalize_email("nope").is_err());
        assert!(normalize_email("a@b").is_err());
        assert!(normalize_email("a b@c.de").is_err());
        assert!(normalize_email("@c.de").is_err());
    }
}
