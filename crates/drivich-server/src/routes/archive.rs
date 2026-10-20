// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Folder download as a .zip, streamed: the archive is built on the fly by a
//! blocking thread and flows to the client through a small bounded channel,
//! so memory stays flat whatever the folder size and a slow client slows
//! the producer down. Files are compressed with a fast deflate level.
//!
//! * `GET /nodes/{id}/zip`            one folder
//! * `GET /nodes/zip?ids=a,b,c`       several items (the selection)
//! * `GET /public/{token}/nodes/{id}/zip`  a folder inside a public link
//!
//! Rights are checked on every requested root before anything is read, the
//! number of simultaneous archives is capped (`DRIVICH_MAX_ZIP_STREAMS`) and
//! the entry count is bounded (see `drivich_core::archive`).

use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::routes::files::content_disposition;
use crate::state::AppState;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::get;
use bytes::Bytes;
use drivich_core::CoreError;
use drivich_core::archive::{self, Entry};
use drivich_core::models::Role;
use drivich_core::perms::{self, NodeRef};
use drivich_core::storage::Storage;
use serde::Deserialize;
use std::io::{Read, Write};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

const MAX_IDS: usize = 200;
const CHUNK: usize = 128 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/nodes/zip", get(zip_selection))
        .route("/nodes/{id}/zip", get(zip_folder))
}

async fn zip_folder(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let (n, _) = perms::authorize(&st.pool, u.id, id, Role::Viewer).await?;
    if n.kind != "folder" {
        return Err(CoreError::bad("only folders can be downloaded as a .zip").into());
    }
    let name = format!("{}.zip", archive::safe_component(&n.name));
    stream_zip(&st, vec![n], &name).await
}

#[derive(Deserialize)]
struct Selection {
    ids: String,
}

async fn zip_selection(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Query(q): Query<Selection>,
) -> ApiResult<Response> {
    let mut ids: Vec<Uuid> = Vec::new();
    for part in q.ids.split(',').filter(|p| !p.is_empty()) {
        let id = Uuid::parse_str(part.trim()).map_err(|_| CoreError::bad("invalid id"))?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.is_empty() || ids.len() > MAX_IDS {
        return Err(CoreError::bad(format!("select between 1 and {MAX_IDS} items")).into());
    }
    let mut roots = Vec::new();
    for id in ids {
        let (n, _) = perms::authorize(&st.pool, u.id, id, Role::Viewer).await?;
        roots.push(n);
    }
    let name = if roots.len() == 1 {
        format!("{}.zip", archive::safe_component(&roots[0].name))
    } else {
        "drivich-download.zip".to_string()
    };
    stream_zip(&st, roots, &name).await
}

/// Archive already-authorised roots and stream the result.
pub async fn stream_zip(st: &AppState, roots: Vec<NodeRef>, filename: &str) -> ApiResult<Response> {
    let permit = st
        .zip_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| CoreError::TooManyRequests)?;
    let entries = archive::collect(&st.pool, &roots).await?;
    let (tx, rx) = mpsc::channel::<std::io::Result<Bytes>>(4);
    let storage = st.storage.clone();
    tokio::task::spawn_blocking(move || {
        let _permit = permit; // released when the archive is done or the client left
        let result = write_archive(&storage, &entries, ChannelWriter::new(tx.clone()));
        if let Err(e) = result {
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                tracing::warn!(error = %e, "zip stream aborted");
                // Surface the failure to the client: the body ends with an error.
                let _ = tx.blocking_send(Err(e));
            }
        }
    });
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            content_disposition("attachment", filename),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(ReceiverStream::new(rx)))
        .map_err(|e| CoreError::Internal(e.into()).into())
}

/// `Write` that hands full buffers to the async side, blocking when the client is slow.
struct ChannelWriter {
    tx: mpsc::Sender<std::io::Result<Bytes>>,
    buf: Vec<u8>,
}

impl ChannelWriter {
    fn new(tx: mpsc::Sender<std::io::Result<Bytes>>) -> Self {
        Self {
            tx,
            buf: Vec::with_capacity(CHUNK),
        }
    }

    fn send(&mut self) -> std::io::Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let chunk = Bytes::from(std::mem::replace(&mut self.buf, Vec::with_capacity(CHUNK)));
        self.tx
            .blocking_send(Ok(chunk))
            .map_err(|_| std::io::ErrorKind::BrokenPipe.into())
    }
}

impl Write for ChannelWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(data);
        if self.buf.len() >= CHUNK {
            self.send()?;
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.send()
    }
}

fn zip_err(e: zip::result::ZipError) -> std::io::Error {
    match e {
        zip::result::ZipError::Io(io) => io,
        other => std::io::Error::other(other),
    }
}

fn zip_time(t: chrono::DateTime<chrono::Utc>) -> Option<zip::DateTime> {
    zip::DateTime::try_from(t.naive_utc()).ok()
}

/// Build the archive into `out`. Public so the tests can read it back.
pub fn write_archive<W: Write>(
    storage: &Storage,
    entries: &[Entry],
    out: W,
) -> std::io::Result<()> {
    let mut zip = zip::ZipWriter::new_stream(out);
    for e in entries {
        let mut opts = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(1))
            .unix_permissions(if e.blob_id.is_some() { 0o644 } else { 0o755 });
        if let Some(t) = zip_time(e.modified) {
            opts = opts.last_modified_time(t);
        }
        match e.blob_id {
            None => zip.add_directory(&e.path, opts).map_err(zip_err)?,
            Some(blob) => {
                let file = std::fs::File::open(storage.blob_path(blob))?;
                let len = file.metadata()?.len();
                // Entries of 4 GiB and more need the zip64 extension.
                zip.start_file(&e.path, opts.large_file(len >= 0xFFFF_0000))
                    .map_err(zip_err)?;
                // `take` guards against a file that grows while we read it.
                std::io::copy(&mut file.take(len), &mut zip)?;
            }
        }
    }
    let mut out = zip.finish().map_err(zip_err)?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::io::Cursor;

    #[test]
    fn archive_round_trips_files_and_directories() {
        let dir = std::env::temp_dir().join(format!("drivich-zip-{}", Uuid::new_v4()));
        let storage = Storage::new(&dir);
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let text = "bonjour ".repeat(10_000);
        for (id, data) in [(a, text.as_bytes()), (b, b"" as &[u8])] {
            let p = storage.blob_path(id);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, data).unwrap();
        }
        let now = Utc::now();
        let entries = vec![
            Entry {
                path: "Dossier/".into(),
                blob_id: None,
                size: 0,
                modified: now,
            },
            Entry {
                path: "Dossier/é à.txt".into(),
                blob_id: Some(a),
                size: text.len() as i64,
                modified: now,
            },
            Entry {
                path: "Dossier/vide.bin".into(),
                blob_id: Some(b),
                size: 0,
                modified: now,
            },
            Entry {
                path: "Dossier/sous/".into(),
                blob_id: None,
                size: 0,
                modified: now,
            },
        ];
        let mut out = Vec::new();
        write_archive(&storage, &entries, &mut out).unwrap();

        let mut zip = zip::ZipArchive::new(Cursor::new(out)).unwrap();
        let names: Vec<String> = zip
            .file_names()
            .map(|n| n.expect("valid name").into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "Dossier/",
                "Dossier/é à.txt",
                "Dossier/vide.bin",
                "Dossier/sous/"
            ]
        );
        let mut got = String::new();
        zip.by_name("Dossier/é à.txt")
            .unwrap()
            .read_to_string(&mut got)
            .unwrap();
        assert_eq!(got, text);
        assert!(zip.by_name("Dossier/sous/").unwrap().is_dir());
        assert!(
            names
                .iter()
                .all(|n| !n.starts_with('/') && !n.contains(".."))
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_closed_client_stops_the_producer() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let mut w = ChannelWriter::new(tx);
        let big = vec![0u8; CHUNK];
        assert_eq!(
            w.write(&big).unwrap_err().kind(),
            std::io::ErrorKind::BrokenPipe
        );
    }
}
