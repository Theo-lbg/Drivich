// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Serving file bytes: download, inline preview and thumbnails, with HTTP
//! Range support (video seeking, resumable downloads) and ETag revalidation.
//! Bytes are streamed from disk, never buffered.

use crate::error::{ApiError, ApiResult};
use crate::extract::AuthUser;
use crate::state::AppState;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::Response;
use axum::routing::{get, post};
use drivich_core::models::Role;
use drivich_core::perms::{self, NodeRef};
use drivich_core::{CoreError, nodes};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/nodes/{id}/download", get(download))
        .route("/nodes/{id}/content", get(content))
        .route("/nodes/{id}/thumbnail", get(thumbnail))
        .route("/nodes/{id}/open", post(open))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Download,
    Inline,
}

async fn file_of(st: &AppState, user: Uuid, id: Uuid) -> Result<NodeRef, CoreError> {
    let (n, _) = perms::authorize(&st.pool, user, id, Role::Viewer).await?;
    if n.kind != "file" || n.blob_id.is_none() {
        return Err(CoreError::bad("this item is a folder"));
    }
    Ok(n)
}

async fn download(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let n = file_of(&st, u.id, id).await?;
    serve_file(&st, &n, &headers, Mode::Download).await
}

async fn content(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let n = file_of(&st, u.id, id).await?;
    serve_file(&st, &n, &headers, Mode::Inline).await
}

async fn thumbnail(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let n = file_of(&st, u.id, id).await?;
    serve_thumbnail(&st, &n, &headers).await
}

async fn open(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    nodes::mark_opened(&st.pool, u.id, id).await?;
    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .map_err(|e| CoreError::Internal(e.into()))?)
}

pub async fn serve_thumbnail(
    st: &AppState,
    n: &NodeRef,
    headers: &HeaderMap,
) -> ApiResult<Response> {
    let blob = n.blob_id.ok_or(CoreError::NotFound)?;
    let ready: Option<String> = sqlx::query_scalar("SELECT thumb_state FROM blobs WHERE id = $1")
        .bind(blob)
        .fetch_optional(&st.pool)
        .await
        .map_err(CoreError::from)?;
    if ready.as_deref() != Some("ready") {
        return Err(CoreError::NotFound.into());
    }
    let path = st.storage.thumb_path(blob);
    let etag = format!("\"t-{}\"", blob.simple());
    stream_path(
        path,
        "image/jpeg".into(),
        etag,
        headers,
        None,
        "private, max-age=86400",
    )
    .await
}

/// Serve the original bytes of a file node.
pub async fn serve_file(
    st: &AppState,
    n: &NodeRef,
    headers: &HeaderMap,
    mode: Mode,
) -> ApiResult<Response> {
    let blob = n.blob_id.ok_or(CoreError::NotFound)?;
    let mime = n
        .mime
        .clone()
        .unwrap_or_else(|| "application/octet-stream".into());
    let (ctype, inline_ok, csp) = inline_policy(&mime);
    let attachment = mode == Mode::Download || !inline_ok;
    let disp = if attachment { "attachment" } else { "inline" };
    let disposition = content_disposition(disp, &n.name);
    let etag = format!("\"{}\"", blob.simple());
    let ctype = if mode == Mode::Download { mime } else { ctype };
    let mut res = stream_path(
        st.storage.blob_path(blob),
        ctype,
        etag,
        headers,
        Some(disposition),
        "private, no-cache",
    )
    .await?;
    if mode == Mode::Inline {
        if let Some(csp) = csp {
            res.headers_mut()
                .insert("content-security-policy", HeaderValue::from_static(csp));
        }
    }
    Ok(res)
}

/// What may be shown inline, and with which content type. Anything that could
/// run script in our origin (HTML, XML...) is shown as plain text instead;
/// SVG is allowed only under a sandboxing CSP.
fn inline_policy(mime: &str) -> (String, bool, Option<&'static str>) {
    match mime {
        "image/svg+xml" => (
            mime.into(),
            true,
            Some("sandbox; default-src 'none'; style-src 'unsafe-inline'"),
        ),
        "text/html"
        | "application/xhtml+xml"
        | "text/xml"
        | "application/xml"
        | "text/javascript"
        | "application/javascript"
        | "text/css" => ("text/plain; charset=utf-8".into(), true, None),
        m if m.starts_with("text/") => (format!("{m}; charset=utf-8"), true, None),
        "application/json" => ("application/json".into(), true, None),
        "application/pdf" => (mime.into(), true, None),
        m if m.starts_with("image/") || m.starts_with("video/") || m.starts_with("audio/") => {
            (mime.into(), true, None)
        }
        _ => ("application/octet-stream".into(), false, None),
    }
}

pub fn content_disposition(kind: &str, name: &str) -> String {
    let fallback: String = name
        .chars()
        .map(|c| {
            if c.is_ascii() && !c.is_control() && c != '"' && c != '\\' && c != '%' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut enc = String::new();
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            enc.push(b as char);
        } else {
            enc.push_str(&format!("%{b:02X}"));
        }
    }
    format!("{kind}; filename=\"{fallback}\"; filename*=UTF-8''{enc}")
}

/// Parse a single `Range: bytes=...` header against a file of `len` bytes.
/// `Ok(None)`: no (usable) range, serve everything. `Err(())`: unsatisfiable.
pub fn parse_range(value: &str, len: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(spec) = value.trim().strip_prefix("bytes=") else {
        return Ok(None);
    };
    if spec.contains(',') {
        return Ok(None); // multi-range: ignore, serve the whole file
    }
    let Some((a, b)) = spec.split_once('-') else {
        return Ok(None);
    };
    let (start, end) = if a.is_empty() {
        let n: u64 = b.trim().parse().map_err(|_| ())?;
        if n == 0 {
            return Err(());
        }
        (len.saturating_sub(n), len.saturating_sub(1))
    } else {
        let s: u64 = a.trim().parse().map_err(|_| ())?;
        let e = if b.trim().is_empty() {
            len.saturating_sub(1)
        } else {
            b.trim()
                .parse::<u64>()
                .map_err(|_| ())?
                .min(len.saturating_sub(1))
        };
        (s, e)
    };
    if len == 0 || start >= len || start > end {
        return Err(());
    }
    Ok(Some((start, end)))
}

async fn stream_path(
    path: PathBuf,
    ctype: String,
    etag: String,
    headers: &HeaderMap,
    disposition: Option<String>,
    cache: &'static str,
) -> ApiResult<Response> {
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|t| t.trim() == etag))
    {
        return Ok(Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, &etag)
            .header(header::CACHE_CONTROL, cache)
            .body(Body::empty())
            .map_err(|e| CoreError::Internal(e.into()))?);
    }
    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(CoreError::from)?;
    let len = file.metadata().await.map_err(CoreError::from)?.len();

    // If-Range: only honour the range when the validator still matches.
    let range_header = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .filter(|_| {
            headers
                .get(header::IF_RANGE)
                .and_then(|v| v.to_str().ok())
                .is_none_or(|v| v == etag)
        });
    let range = match range_header {
        None => None,
        Some(r) => match parse_range(r, len) {
            Ok(x) => x,
            Err(()) => {
                return Ok(Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(header::CONTENT_RANGE, format!("bytes */{len}"))
                    .body(Body::empty())
                    .map_err(|e| CoreError::Internal(e.into()))?);
            }
        },
    };

    let (status, start, count) = match range {
        Some((s, e)) => (StatusCode::PARTIAL_CONTENT, s, e - s + 1),
        None => (StatusCode::OK, 0, len),
    };
    if start > 0 {
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(CoreError::from)?;
    }
    let stream = ReaderStream::with_capacity(file.take(count), 256 * 1024);
    let mut b = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, ctype)
        .header(header::CONTENT_LENGTH, count)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::ETAG, &etag)
        .header(header::CACHE_CONTROL, cache);
    if let Some(d) = disposition {
        b = b.header(header::CONTENT_DISPOSITION, d);
    }
    if status == StatusCode::PARTIAL_CONTENT {
        b = b.header(
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", start, start + count - 1, len),
        );
    }
    b.body(Body::from_stream(stream))
        .map_err(|e| ApiError(CoreError::Internal(e.into())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_parsed_like_http_says() {
        assert_eq!(parse_range("bytes=0-99", 1000), Ok(Some((0, 99))));
        assert_eq!(parse_range("bytes=900-", 1000), Ok(Some((900, 999))));
        assert_eq!(parse_range("bytes=-100", 1000), Ok(Some((900, 999))));
        assert_eq!(parse_range("bytes=0-5000", 1000), Ok(Some((0, 999))));
        assert_eq!(parse_range("bytes=1000-", 1000), Err(()));
        assert_eq!(parse_range("bytes=5-2", 1000), Err(()));
        assert_eq!(parse_range("bytes=0-1,5-9", 1000), Ok(None));
        assert_eq!(parse_range("items=0-1", 1000), Ok(None));
        assert_eq!(parse_range("bytes=0-0", 0), Err(()));
    }

    #[test]
    fn dangerous_types_are_never_served_as_html() {
        assert_eq!(inline_policy("text/html").0, "text/plain; charset=utf-8");
        assert!(inline_policy("image/svg+xml").2.is_some());
        assert!(!inline_policy("application/x-msdownload").1);
        assert!(inline_policy("video/mp4").1);
    }

    #[test]
    fn disposition_escapes_quotes_and_unicode() {
        let d = content_disposition("attachment", "é\"x.txt");
        assert!(d.contains("filename=\"_"));
        assert!(d.contains("%C3%A9%22x.txt"));
        assert!(!d.contains("\"x"));
    }
}
