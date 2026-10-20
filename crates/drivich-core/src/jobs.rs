// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Background job queue stored in PostgreSQL.
//!
//! Producers insert rows (optionally deduplicated) and `NOTIFY` the worker.
//! Workers claim one job at a time with `FOR UPDATE SKIP LOCKED`, under a lease
//! (`locked_until`): if a worker dies, its job becomes claimable again when the
//! lease expires. Failures are retried with exponential backoff.

use crate::db::Pool;
use crate::error::Result;
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::FromRow;
use uuid::Uuid;

pub const CHANNEL: &str = "drivich_jobs";
pub const LEASE_SECONDS: i64 = 15 * 60;

pub mod kind {
    pub const THUMBNAIL: &str = "thumbnail";
    pub const INDEX_TEXT: &str = "index_text";
    pub const CHECKSUM: &str = "checksum";
    pub const PURGE_TRASH: &str = "purge_trash";
    pub const CLEANUP_UPLOADS: &str = "cleanup_uploads";
    pub const CLEANUP_SESSIONS: &str = "cleanup_sessions";
    pub const GC_BLOBS: &str = "gc_blobs";
    pub const DB_DUMP: &str = "db_dump";
}

#[derive(Debug, Clone, FromRow)]
pub struct Job {
    pub id: i64,
    pub kind: String,
    pub payload: Value,
    pub attempts: i32,
    pub max_attempts: i32,
}

/// Enqueue a job. With a `dedupe_key`, an identical queued/running job is kept
/// instead of inserting a duplicate.
pub async fn enqueue(
    pool: &Pool,
    kind: &str,
    payload: Value,
    dedupe_key: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO jobs (kind, payload, dedupe_key) VALUES ($1, $2, $3) \
         ON CONFLICT (dedupe_key) WHERE dedupe_key IS NOT NULL AND status IN ('queued','running') DO NOTHING",
    )
    .bind(kind)
    .bind(payload)
    .bind(dedupe_key)
    .execute(pool)
    .await?;
    // Wake up idle workers; failure to notify is harmless (they also poll).
    let _ = sqlx::query("SELECT pg_notify($1, $2)")
        .bind(CHANNEL)
        .bind(kind)
        .execute(pool)
        .await;
    Ok(())
}

pub async fn enqueue_blob(pool: &Pool, kind: &str, blob: Uuid) -> Result<()> {
    let key = format!("{kind}:{blob}");
    enqueue(pool, kind, json!({ "blob_id": blob }), Some(&key)).await
}

/// Every kind of job, for the admin controls.
pub const ALL_KINDS: [&str; 8] = [
    kind::THUMBNAIL,
    kind::CHECKSUM,
    kind::INDEX_TEXT,
    kind::DB_DUMP,
    kind::PURGE_TRASH,
    kind::CLEANUP_UPLOADS,
    kind::CLEANUP_SESSIONS,
    kind::GC_BLOBS,
];

const PAUSED_KEY: &str = "jobs_paused";
const LIMITS_KEY: &str = "job_limits";

/// Admin-controlled behaviour of the queue (stored in `settings`).
#[derive(Debug, Clone, Default, Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct JobControls {
    /// Kinds that are not started while paused (running jobs finish).
    pub paused: Vec<String>,
    /// Maximum simultaneous jobs per kind, across all workers (absent = no limit).
    pub limits: std::collections::BTreeMap<String, i64>,
}

pub async fn controls(pool: &Pool) -> Result<JobControls> {
    let paused: Vec<String> = crate::settings::get(pool, PAUSED_KEY)
        .await?
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let limits = crate::settings::get(pool, LIMITS_KEY)
        .await?
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    Ok(JobControls { paused, limits })
}

pub async fn set_controls(pool: &Pool, c: &JobControls) -> Result<()> {
    for k in c.paused.iter().chain(c.limits.keys()) {
        if !ALL_KINDS.contains(&k.as_str()) {
            return Err(crate::error::CoreError::bad(format!(
                "unknown job kind: {k}"
            )));
        }
    }
    if c.limits.values().any(|n| !(0..=32).contains(n)) {
        return Err(crate::error::CoreError::bad(
            "a limit must be between 0 and 32",
        ));
    }
    crate::settings::set(pool, PAUSED_KEY, &json!(c.paused)).await?;
    crate::settings::set(pool, LIMITS_KEY, &json!(c.limits)).await?;
    Ok(())
}

/// Claim the next runnable job (or a job whose lease expired), skipping paused
/// kinds and kinds already running at their concurrency limit.
pub async fn claim(pool: &Pool) -> Result<Option<Job>> {
    let c = controls(pool).await?;
    let mut blocked = c.paused.clone();
    if !c.limits.is_empty() {
        let running: Vec<(String, i64)> = sqlx::query_as(
            "SELECT kind, count(*)::bigint FROM jobs WHERE status = 'running' AND locked_until >= now() GROUP BY kind",
        )
        .fetch_all(pool)
        .await?;
        for (kind, n) in running {
            if c.limits.get(&kind).is_some_and(|limit| n >= *limit) {
                blocked.push(kind);
            }
        }
        // A limit of 0 means "do not start any".
        blocked.extend(
            c.limits
                .iter()
                .filter(|(_, l)| **l == 0)
                .map(|(k, _)| k.clone()),
        );
    }
    Ok(sqlx::query_as(
        "UPDATE jobs SET status = 'running', attempts = attempts + 1, \
             locked_until = now() + make_interval(secs => $1), updated_at = now() \
         WHERE id = ( \
             SELECT id FROM jobs \
             WHERE ((status = 'queued' AND run_at <= now()) OR (status = 'running' AND locked_until < now())) \
               AND kind <> ALL($2) \
             ORDER BY run_at, id FOR UPDATE SKIP LOCKED LIMIT 1) \
         RETURNING id, kind, payload, attempts, max_attempts",
    )
    .bind(LEASE_SECONDS as f64)
    .bind(&blocked)
    .fetch_optional(pool)
    .await?)
}

pub async fn complete(pool: &Pool, id: i64) -> Result<()> {
    sqlx::query("UPDATE jobs SET status = 'done', finished_at = now(), updated_at = now(), locked_until = NULL, last_error = NULL WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Record a failure: retry later with backoff, or give up after `max_attempts`.
pub async fn fail(pool: &Pool, job: &Job, error: &str) -> Result<()> {
    let msg: String = error.chars().take(2000).collect();
    if job.attempts >= job.max_attempts {
        sqlx::query("UPDATE jobs SET status = 'failed', finished_at = now(), updated_at = now(), locked_until = NULL, last_error = $2 WHERE id = $1")
            .bind(job.id)
            .bind(msg)
            .execute(pool)
            .await?;
    } else {
        let backoff = 30.0 * 4f64.powi(job.attempts.max(1) - 1);
        sqlx::query("UPDATE jobs SET status = 'queued', run_at = now() + make_interval(secs => $3), updated_at = now(), locked_until = NULL, last_error = $2 WHERE id = $1")
            .bind(job.id)
            .bind(msg)
            .bind(backoff)
            .execute(pool)
            .await?;
    }
    Ok(())
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct JobCount {
    pub kind: String,
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct FailedJob {
    pub id: i64,
    pub kind: String,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn stats(pool: &Pool) -> Result<Vec<JobCount>> {
    Ok(sqlx::query_as("SELECT kind, status, count(*)::bigint AS count FROM jobs GROUP BY kind, status ORDER BY kind, status")
        .fetch_all(pool)
        .await?)
}

pub async fn recent_failures(pool: &Pool) -> Result<Vec<FailedJob>> {
    Ok(sqlx::query_as(
        "SELECT id, kind, attempts, last_error, updated_at FROM jobs WHERE status = 'failed' ORDER BY updated_at DESC LIMIT 20",
    )
    .fetch_all(pool)
    .await?)
}

/// Put every failed job back in the queue (admin action).
pub async fn retry_failed(pool: &Pool) -> Result<u64> {
    let r = sqlx::query("UPDATE jobs SET status = 'queued', attempts = 0, run_at = now(), updated_at = now(), finished_at = NULL WHERE status = 'failed'")
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

/// Delete finished jobs older than `days` days.
pub async fn prune(pool: &Pool, days: i32) -> Result<u64> {
    let r = sqlx::query("DELETE FROM jobs WHERE status = 'done' AND finished_at < now() - make_interval(days => $1)")
        .bind(days)
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

/// Enqueue a maintenance job unless one is already pending or finished within
/// the last `min_interval_minutes` (keeps hourly tasks hourly, whatever the
/// number of workers or restarts).
pub async fn enqueue_periodic(pool: &Pool, kind: &str, min_interval_minutes: i32) -> Result<()> {
    sqlx::query(
        "INSERT INTO jobs (kind) SELECT $1::text WHERE NOT EXISTS ( \
             SELECT 1 FROM jobs WHERE kind = $1::text AND (status IN ('queued','running') \
             OR finished_at > now() - make_interval(mins => $2)))",
    )
    .bind(kind)
    .bind(min_interval_minutes)
    .execute(pool)
    .await?;
    Ok(())
}

/// After a crash or an upgrade: queue work for blobs still waiting for a
/// thumbnail, checksum or text index.
pub async fn requeue_pending(pool: &Pool) -> Result<u64> {
    let r = sqlx::query(
        "INSERT INTO jobs (kind, payload, dedupe_key) \
         SELECT 'thumbnail', jsonb_build_object('blob_id', id), 'thumbnail:' || id FROM blobs WHERE thumb_state = 'pending' \
         UNION ALL \
         SELECT 'checksum', jsonb_build_object('blob_id', id), 'checksum:' || id FROM blobs WHERE sha256 IS NULL \
         ON CONFLICT (dedupe_key) WHERE dedupe_key IS NOT NULL AND status IN ('queued','running') DO NOTHING",
    )
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
}
