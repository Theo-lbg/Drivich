// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Drivich HTTP API.

use drivich_core::config::Config;
use drivich_core::db;
use drivich_core::storage::Storage;
use drivich_core::uploads::Limits;
use drivich_server::routes;
use drivich_server::state::{AppState, Limiter};
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // `drivich-server --healthcheck`: used by the container HEALTHCHECK (no curl in the image).
    if std::env::args().nth(1).as_deref() == Some("--healthcheck") {
        return healthcheck().await;
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,sqlx=warn")),
        )
        .init();

    let cfg = Config::from_env()?;
    let pool = db::connect(&cfg.database_url, cfg.db_max_connections).await?;
    db::migrate(&pool).await?;
    let storage = Storage::new(cfg.data_dir.clone());
    storage.init().await?;
    let has_files: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM blobs)")
        .fetch_one(&pool)
        .await?;
    storage
        .verify_mounts(has_files, cfg.ignore_mount_check)
        .await?;

    let state = AppState {
        pool,
        storage,
        limits: Limits {
            max_file_bytes: cfg.max_file_bytes,
            ttl_hours: cfg.upload_ttl_hours,
            max_versions: cfg.max_versions,
        },
        limiter: Arc::new(Limiter::new(8, Duration::from_secs(600))),
        zip_slots: Arc::new(tokio::sync::Semaphore::new(cfg.max_zip_streams)),
        cfg: Arc::new(cfg.clone()),
    };

    let app = routes::router(state);
    let listener = tokio::net::TcpListener::bind(&cfg.bind).await?;
    tracing::info!(bind = %cfg.bind, version = drivich_core::VERSION, "drivich-server listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

async fn shutdown() {
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
    tracing::info!("shutting down");
}

/// Minimal HTTP/1.0 probe of our own `/api/v1/health`; exits non-zero on failure.
async fn healthcheck() -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let bind = std::env::var("DRIVICH_BIND").unwrap_or_else(|_| "0.0.0.0:2283".into());
    let port = bind.rsplit(':').next().unwrap_or("2283");
    let probe = async {
        let mut s = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}")).await?;
        s.write_all(b"GET /api/v1/health HTTP/1.0\r\nHost: localhost\r\n\r\n")
            .await?;
        let mut buf = Vec::new();
        s.read_to_end(&mut buf).await?;
        anyhow::Ok(buf)
    };
    let buf = tokio::time::timeout(Duration::from_secs(4), probe).await??;
    let head = String::from_utf8_lossy(&buf[..buf.len().min(16)]).to_string();
    if head.contains(" 200") {
        Ok(())
    } else {
        eprintln!("unhealthy: {head}");
        std::process::exit(1);
    }
}
