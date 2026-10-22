// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Editing Word / Excel / PowerPoint / OpenDocument files in the browser with
//! an ONLYOFFICE Document Server (optional: enabled when
//! `DRIVICH_ONLYOFFICE_JWT_SECRET` is set).
//!
//! * `GET  /nodes/{id}/office`        editor configuration for the signed-in user
//! * `GET  /office/file/{token}`      the document bytes, fetched by the Document Server
//! * `POST /office/callback/{token}`  the Document Server announces a saved document
//!
//! The Document Server has no Drivich session, so both of its calls carry a
//! short-lived token signed with the shared secret, and the editor
//! configuration is itself a signed JWT (HS256), as the Document Server expects.
//! A saved document is stored as a new version of the file, so nothing is lost.

use crate::error::ApiResult;
use crate::extract::AuthUser;
use crate::routes::files::stream_path;
use crate::state::AppState;
use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, header};
use axum::response::Response;
use axum::routing::{get, post};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use drivich_core::CoreError;
use drivich_core::models::Role;
use drivich_core::perms;
use drivich_core::uploads;
use futures_util::StreamExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Prefix under which the web container exposes the Document Server.
const PUBLIC_PREFIX: &str = "/onlyoffice";
/// Lifetime of the tokens handed to the Document Server.
const TOKEN_HOURS: i64 = 12;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/nodes/{id}/office", get(editor_config))
        .route("/nodes/{id}/office/title", post(sync_title))
        .route("/office/file/{token}", get(file))
        .route("/office/callback/{token}", post(callback))
}

// --- JWT (HS256) ---------------------------------------------------------

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(k.map(|b| b ^ 0x36));
    inner.update(data);
    let mut outer = Sha256::new();
    outer.update(k.map(|b| b ^ 0x5c));
    outer.update(inner.finalize());
    outer.finalize().into()
}

fn sign(secret: &str, claims: &Value) -> String {
    let head = B64.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let body = B64.encode(claims.to_string());
    let signing = format!("{head}.{body}");
    let sig = B64.encode(hmac_sha256(secret.as_bytes(), signing.as_bytes()));
    format!("{signing}.{sig}")
}

/// The claims of a correctly signed, unexpired token.
fn verify(secret: &str, token: &str) -> Option<Value> {
    let (signing, sig) = token.rsplit_once('.')?;
    let (head, body) = signing.split_once('.')?;
    let header: Value = serde_json::from_slice(&B64.decode(head).ok()?).ok()?;
    if header.get("alg")?.as_str()? != "HS256" {
        return None;
    }
    let given = B64.decode(sig).ok()?;
    let expected = hmac_sha256(secret.as_bytes(), signing.as_bytes());
    // Constant-time comparison.
    let diff = given.len() ^ expected.len()
        | given
            .iter()
            .zip(expected.iter())
            .fold(0usize, |a, (x, y)| a | (x ^ y) as usize);
    if diff != 0 {
        return None;
    }
    let claims: Value = serde_json::from_slice(&B64.decode(body).ok()?).ok()?;
    if let Some(exp) = claims.get("exp").and_then(Value::as_i64) {
        if exp < chrono::Utc::now().timestamp() {
            return None;
        }
    }
    Some(claims)
}

fn secret(st: &AppState) -> Result<&str, CoreError> {
    st.cfg
        .office_jwt_secret
        .as_deref()
        .ok_or_else(|| CoreError::bad("document editing is not enabled on this server"))
}

fn link_token(secret: &str, kind: &str, node: Uuid, blob: Uuid) -> String {
    sign(
        secret,
        &json!({
            "typ": kind,
            "node": node,
            "blob": blob,
            "exp": chrono::Utc::now().timestamp() + TOKEN_HOURS * 3600,
        }),
    )
}

// --- editor configuration ------------------------------------------------

/// (document type, file type) of an extension the editor opens.
fn kind_of(name: &str) -> Option<(&'static str, String)> {
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    let doc = match ext.as_str() {
        "doc" | "docx" | "docm" | "odt" | "rtf" | "txt" | "dotx" => "word",
        "xls" | "xlsx" | "xlsm" | "ods" | "csv" | "xltx" => "cell",
        "ppt" | "pptx" | "pptm" | "odp" | "ppsx" => "slide",
        _ => return None,
    };
    Some((doc, ext))
}

async fn editor_config(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let secret = secret(&st)?;
    let (n, role) = perms::authorize(&st.pool, u.id, id, Role::Viewer).await?;
    let blob = match (n.kind.as_str(), n.blob_id) {
        ("file", Some(b)) => b,
        _ => return Err(CoreError::bad("this item is not a document").into()),
    };
    let (document_type, file_type) = kind_of(&n.name)
        .ok_or_else(|| CoreError::bad("this file type cannot be opened in the editor"))?;
    let edit = role >= Role::Editor && n.trashed_at.is_none();
    let app = &st.cfg.office_app_url;
    let mut config = json!({
        "documentType": document_type,
        "document": {
            "fileType": file_type,
            // A new key for each content: it changes whenever a save creates a new blob.
            "key": blob.simple().to_string(),
            "title": n.name,
            "url": format!("{app}/api/v1/office/file/{}", link_token(secret, "file", n.id, blob)),
            "permissions": { "edit": edit, "download": true, "print": true, "comment": edit },
        },
        "editorConfig": {
            "mode": if edit { "edit" } else { "view" },
            "lang": u.locale.clone().unwrap_or_else(|| "fr".into()),
            "callbackUrl": format!("{app}/api/v1/office/callback/{}", link_token(secret, "callback", n.id, blob)),
            "user": { "id": u.id, "name": u.name },
            "customization": { "autosave": true, "forcesave": true, "compactHeader": true, "hideRightMenu": true, "rename": edit },
        },
        "height": "100%",
        "width": "100%",
    });
    let token = sign(secret, &config);
    config["token"] = Value::String(token);
    Ok(Json(json!({ "server": PUBLIC_PREFIX, "config": config })))
}

/// After a rename, tells the Document Server the new title so the open editor
/// shows it (its `meta` command). Best effort: the rename itself already succeeded.
async fn sync_title(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let secret = secret(&st)?;
    let (n, _) = perms::authorize(&st.pool, u.id, id, Role::Editor).await?;
    let Some(blob) = n.blob_id else {
        return Err(CoreError::bad("this item is not a document").into());
    };
    let body = json!({
        "c": "meta",
        "key": blob.simple().to_string(),
        "meta": { "title": n.name },
    });
    let res = reqwest::Client::new()
        .post(format!("{}/command", st.cfg.office_server_url))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", sign(secret, &json!({ "payload": body }))),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(
            json!({ "c": "meta", "key": body["key"], "meta": body["meta"], "token": sign(secret, &body) })
                .to_string(),
        )
        .send()
        .await;
    if let Err(e) = res {
        tracing::warn!(error = %e, "could not push the new title to the Document Server");
    }
    Ok(Json(json!({ "ok": true })))
}

// --- calls from the Document Server --------------------------------------

async fn file(
    State(st): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let claims = verify(secret(&st)?, &token)
        .filter(|c| c["typ"] == "file")
        .ok_or(CoreError::Unauthorized)?;
    let blob: Uuid =
        serde_json::from_value(claims["blob"].clone()).map_err(|_| CoreError::NotFound)?;
    let mime: Option<String> = sqlx::query_scalar("SELECT mime FROM blobs WHERE id = $1")
        .bind(blob)
        .fetch_optional(&st.pool)
        .await
        .map_err(CoreError::from)?;
    let mime = mime.ok_or(CoreError::NotFound)?;
    stream_path(
        st.storage.blob_path(blob),
        mime,
        format!("\"{}\"", blob.simple()),
        &headers,
        None,
        "private, no-store",
    )
    .await
}

/// The address to download a saved document from. The Document Server
/// announces its public address; we reach it on the internal one instead.
fn internal_download_url(base: &str, announced: &str) -> Option<String> {
    let u = url::Url::parse(announced).ok()?;
    let path = u.path();
    let path = path.strip_prefix(PUBLIC_PREFIX).unwrap_or(path);
    let mut out = format!("{base}{path}");
    if let Some(q) = u.query() {
        out.push('?');
        out.push_str(q);
    }
    Some(out)
}

async fn callback(
    State(st): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    match handle_callback(&st, &token, &headers, &body).await {
        Ok(()) => Json(json!({ "error": 0 })),
        Err(e) => {
            tracing::warn!(error = %e, "document save failed");
            Json(json!({ "error": 1 }))
        }
    }
}

async fn handle_callback(
    st: &AppState,
    token: &str,
    headers: &HeaderMap,
    body: &Value,
) -> Result<(), CoreError> {
    let secret = secret(st)?;
    let link = verify(secret, token)
        .filter(|c| c["typ"] == "callback")
        .ok_or(CoreError::Unauthorized)?;
    let node: Uuid =
        serde_json::from_value(link["node"].clone()).map_err(|_| CoreError::NotFound)?;

    // The Document Server signs what it sends (header, or `token` in the body).
    let signed = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| body.get("token").and_then(Value::as_str))
        .ok_or(CoreError::Unauthorized)?;
    let claims = verify(secret, signed).ok_or(CoreError::Unauthorized)?;
    let payload = claims.get("payload").unwrap_or(&claims);

    // 2: closed after editing, 6: saved while still open. Other statuses carry nothing to store.
    let status = payload["status"].as_i64().unwrap_or(0);
    if status != 2 && status != 6 {
        return Ok(());
    }
    let announced = payload["url"]
        .as_str()
        .ok_or_else(|| CoreError::bad("no document url"))?;
    let url = internal_download_url(&st.cfg.office_server_url, announced)
        .ok_or_else(|| CoreError::bad("invalid document url"))?;

    // Saved on behalf of the last editor; their rights are checked again by the upload.
    let user = payload["users"]
        .as_array()
        .and_then(|a| a.last())
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok());
    let user = match user {
        Some(u) => u,
        None => perms::node_ref(&st.pool, node).await?.owner_id,
    };

    let res = reqwest::get(&url)
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| CoreError::Internal(e.into()))?;
    let len = res
        .content_length()
        .ok_or_else(|| CoreError::bad("the Document Server sent no length"))?;
    if len == 0 {
        return Err(CoreError::bad("empty document"));
    }
    let info =
        uploads::create_version(&st.pool, &st.storage, &st.limits, user, node, len as i64).await?;
    let stream = res.bytes_stream().map(|r| r.map_err(std::io::Error::other));
    uploads::append(
        &st.pool,
        &st.storage,
        &st.limits,
        user,
        info.id,
        0,
        Box::pin(stream),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_matches_rfc4231_case_2() {
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex::encode(mac),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn jwt_round_trip_and_tampering() {
        let t = sign("0123456789abcdef", &json!({ "a": 1 }));
        assert_eq!(verify("0123456789abcdef", &t).unwrap()["a"], 1);
        assert!(verify("another-secret-value", &t).is_none());
        let forged = format!("{}x", t);
        assert!(verify("0123456789abcdef", &forged).is_none());
        let expired = sign("0123456789abcdef", &json!({ "exp": 1 }));
        assert!(verify("0123456789abcdef", &expired).is_none());
    }

    #[test]
    fn download_url_is_rewritten_to_the_internal_address() {
        assert_eq!(
            internal_download_url(
                "http://onlyoffice:80",
                "https://drive.example/onlyoffice/cache/files/a/output.docx/o.docx?md5=x&expires=1"
            )
            .unwrap(),
            "http://onlyoffice:80/cache/files/a/output.docx/o.docx?md5=x&expires=1"
        );
    }
}
