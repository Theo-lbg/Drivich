// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Drivich background worker: thumbnails, checksums, text indexing and
//! housekeeping, fed by the PostgreSQL-backed queue in `drivich_core::jobs`.

use drivich_core::config::Config;
use drivich_core::db::{self, Pool};
use drivich_core::jobs::{self, Job, kind};
use drivich_core::storage::Storage;
use drivich_core::{backups, nodes, thumbs, uploads};
use sqlx::postgres::PgListener;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, Semaphore};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Clone)]
struct Ctx {
    pool: Pool,
    storage: Storage,
    cfg: Arc<Config>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .init();

    let cfg = Config::from_env()?;
    let pool = db::connect(
        &cfg.database_url,
        (cfg.worker_concurrency as u32 + 3).max(5),
    )
    .await?;
    db::migrate(&pool).await?;
    let storage = Storage::new(cfg.data_dir.clone());
    storage.init().await?;
    let has_files: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM blobs)")
        .fetch_one(&pool)
        .await?;
    storage
        .verify_mounts(has_files, cfg.ignore_mount_check)
        .await?;
    let ctx = Ctx {
        pool: pool.clone(),
        storage,
        cfg: Arc::new(cfg.clone()),
    };

    let queued = jobs::requeue_pending(&pool).await?;
    tracing::info!(
        concurrency = cfg.worker_concurrency,
        recovered = queued,
        "drivich-worker started"
    );

    let wake = Arc::new(Notify::new());
    tokio::spawn(listen(cfg.database_url.clone(), wake.clone()));
    tokio::spawn(schedule(pool.clone(), cfg.backups_enabled));

    let slots = Arc::new(Semaphore::new(cfg.worker_concurrency));
    let stop = shutdown_signal();
    tokio::pin!(stop);

    loop {
        let permit = tokio::select! {
            p = slots.clone().acquire_owned() => p?,
            _ = &mut stop => break,
        };
        match jobs::claim(&pool).await {
            Ok(Some(job)) => {
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    run(&ctx, job).await;
                    drop(permit);
                });
            }
            Ok(None) => {
                drop(permit);
                tokio::select! {
                    _ = wake.notified() => {},
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {},
                    _ = &mut stop => break,
                }
            }
            Err(e) => {
                drop(permit);
                tracing::error!(error = %e, "could not claim a job");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    tracing::info!("stopping: waiting for running jobs");
    let _ = tokio::time::timeout(
        Duration::from_secs(30),
        slots.acquire_many(cfg.worker_concurrency as u32),
    )
    .await;
    Ok(())
}

/// Wake the loop as soon as a producer enqueues something.
async fn listen(url: String, wake: Arc<Notify>) {
    loop {
        match PgListener::connect(&url).await {
            Ok(mut l) => {
                if l.listen(jobs::CHANNEL).await.is_ok() {
                    while l.recv().await.is_ok() {
                        wake.notify_one();
                    }
                }
            }
            Err(e) => tracing::warn!(error = %e, "listener connection failed"),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Queue the periodic housekeeping jobs.
async fn schedule(pool: Pool, backups: bool) {
    loop {
        let mut periodic = vec![
            (kind::PURGE_TRASH, 55),
            (kind::CLEANUP_UPLOADS, 55),
            (kind::CLEANUP_SESSIONS, 55),
            (kind::GC_BLOBS, 23 * 60),
        ];
        if backups {
            // Roughly daily; "Create a backup" in the admin area queues one on demand.
            periodic.push((kind::DB_DUMP, 23 * 60));
        }
        for (k, every) in periodic {
            if let Err(e) = jobs::enqueue_periodic(&pool, k, every).await {
                tracing::warn!(error = %e, job = k, "could not schedule job");
            }
        }
        tokio::time::sleep(Duration::from_secs(10 * 60)).await;
    }
}

fn blob_id(job: &Job) -> anyhow::Result<Uuid> {
    job.payload
        .get("blob_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("job {} has no blob_id", job.id))
}

async fn run(ctx: &Ctx, job: Job) {
    let started = std::time::Instant::now();
    let result: anyhow::Result<()> = async {
        match job.kind.as_str() {
            kind::THUMBNAIL => {
                thumbs::make_thumbnail(
                    &ctx.pool,
                    &ctx.storage,
                    blob_id(&job)?,
                    ctx.cfg.thumb_max_px,
                )
                .await?
            }
            kind::CHECKSUM => {
                thumbs::compute_checksum(&ctx.pool, &ctx.storage, blob_id(&job)?).await?
            }
            kind::INDEX_TEXT => thumbs::index_text(&ctx.pool, &ctx.storage, blob_id(&job)?).await?,
            kind::PURGE_TRASH => {
                let n = nodes::purge_old_trash(&ctx.pool, &ctx.storage, ctx.cfg.trash_days).await?;
                tracing::info!(purged = n, "trash purged");
            }
            kind::CLEANUP_UPLOADS => {
                let n = uploads::cleanup_expired(&ctx.pool, &ctx.storage).await?;
                tracing::info!(removed = n, "stale uploads removed");
            }
            kind::CLEANUP_SESSIONS => {
                sqlx::query("DELETE FROM sessions WHERE expires_at < now()")
                    .execute(&ctx.pool)
                    .await?;
                jobs::prune(&ctx.pool, 7).await?;
            }
            kind::DB_DUMP => {
                let name = backups::create(&ctx.storage, &ctx.cfg.database_url).await?;
                let pruned = backups::prune(&ctx.storage, ctx.cfg.backup_keep).await?;
                tracing::info!(backup = %name, pruned, "database dump created");
            }
            kind::GC_BLOBS => {
                let n = thumbs::gc_orphan_blobs(&ctx.pool, &ctx.storage).await?;
                tracing::info!(removed = n, "orphan blobs removed");
            }
            other => anyhow::bail!("unknown job kind {other}"),
        }
        Ok(())
    }
    .await;

    match result {
        Ok(()) => {
            tracing::info!(id = job.id, kind = %job.kind, ms = started.elapsed().as_millis() as u64, "job done");
            if let Err(e) = jobs::complete(&ctx.pool, job.id).await {
                tracing::error!(error = %e, "could not mark job done");
            }
        }
        Err(e) => {
            tracing::warn!(id = job.id, kind = %job.kind, attempt = job.attempts, error = %e, "job failed");
            if let Err(e2) = jobs::fail(&ctx.pool, &job, &e.to_string()).await {
                tracing::error!(error = %e2, "could not record job failure");
            }
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}
