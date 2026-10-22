// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Cucumber acceptance tests: the real router, a real PostgreSQL, no network.
//! Run: `DATABASE_URL=postgres://… cargo test -p drivich-server --test features`

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request};
use cucumber::{World, gherkin::Step, given, then, when};
use drivich_core::{config::Config, db, storage::Storage, uploads::Limits, users};
use drivich_server::routes;
use drivich_server::state::{AppState, Limiter};
use http_body_util::BodyExt;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

const PASSWORD: &str = "a-long-enough-password";
const OFFICE_SECRET: &str = "cucumber-office-secret-0123456789";

#[derive(World)]
#[world(init = Self::new)]
struct Api {
    app: Router,
    /// Session cookie per actor name.
    cookies: HashMap<String, String>,
    /// Values saved from responses, substituted for `<name>` in later steps.
    vars: HashMap<String, String>,
    /// Unique per scenario so e-mail addresses never collide in the shared database.
    run: String,
    pool: db::Pool,
    status: u16,
    headers: HeaderMap,
    body: Vec<u8>,
    skip_csrf: bool,
    storage: Storage,
    office_secret: Option<String>,
    office_url: String,
    state_cfg: Config,
}

impl std::fmt::Debug for Api {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Api({} {})", self.run, self.status)
    }
}

impl Api {
    async fn new() -> Self {
        let url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must point to a test PostgreSQL");
        let pool = db::connect(&url, 5).await.unwrap();
        db::migrate(&pool).await.unwrap();
        let dir = std::env::temp_dir().join(format!("drivich-cucumber-{}", Uuid::new_v4()));
        let storage = Storage::new(dir.clone());
        storage.init().await.unwrap();
        let cfg = Config {
            database_url: url,
            data_dir: dir,
            bind: "127.0.0.1:0".into(),
            max_file_bytes: 0,
            session_days: 1,
            cookie_secure: false,
            db_max_connections: 5,
            worker_concurrency: 1,
            thumb_max_px: 480,
            upload_ttl_hours: 1,
            trash_days: 30,
            backups_enabled: false,
            backup_keep: 1,
            max_versions: 3,
            max_zip_streams: 2,
            trust_proxy: false,
            ignore_mount_check: true,
            office_jwt_secret: None,
            office_server_url: "http://onlyoffice.invalid".into(),
            office_app_url: "http://server.invalid".into(),
        };
        let mut w = Self {
            app: Router::new(),
            storage,
            office_secret: None,
            office_url: String::new(),
            state_cfg: cfg,
            cookies: HashMap::new(),
            vars: HashMap::new(),
            run: Uuid::new_v4().simple().to_string(),
            pool,
            status: 0,
            headers: HeaderMap::new(),
            body: Vec::new(),
            skip_csrf: false,
        };
        w.office_url = mock_document_server().await;
        w.office_secret = Some(OFFICE_SECRET.to_string());
        w.rebuild();
        w
    }

    /// (Re)create the router from the current office settings.
    fn rebuild(&mut self) {
        let mut cfg = self.state_cfg.clone();
        cfg.office_jwt_secret = self.office_secret.clone();
        cfg.office_server_url = self.office_url.clone();
        let state = AppState {
            pool: self.pool.clone(),
            storage: self.storage.clone(),
            limits: Limits {
                max_file_bytes: self.state_cfg.max_file_bytes,
                ttl_hours: 1,
                max_versions: 3,
            },
            limiter: Arc::new(Limiter::new(8, Duration::from_secs(600))),
            zip_slots: Arc::new(tokio::sync::Semaphore::new(2)),
            cfg: Arc::new(cfg),
        };
        self.app = routes::router(state);
    }

    fn email(&self, who: &str) -> String {
        format!("{}-{}@example.test", who.to_lowercase(), self.run)
    }

    /// Replace `<name>` with saved values (and `<run>` with this scenario's id).
    fn fill(&self, text: &str) -> String {
        let mut out = text.replace("<run>", &self.run);
        for (k, v) in &self.vars {
            out = out.replace(&format!("<{k}>"), v);
        }
        out
    }

    async fn send(
        &mut self,
        who: &str,
        method: &str,
        path: &str,
        headers: &[(&str, String)],
        body: Vec<u8>,
        json: bool,
    ) {
        let uri = format!("/api/v1{}", self.fill(path));
        let mut req = Request::builder()
            .method(Method::from_bytes(method.as_bytes()).unwrap())
            .uri(uri);
        if !std::mem::take(&mut self.skip_csrf) {
            req = req.header("x-drivich-csrf", "1");
        }
        if let Some(c) = self.cookies.get(who) {
            req = req.header("cookie", format!("drivich_session={c}"));
        }
        if json {
            req = req.header("content-type", "application/json");
        }
        for (k, v) in headers {
            req = req.header(*k, v.as_str());
        }
        let res = self
            .app
            .clone()
            .oneshot(req.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        self.status = res.status().as_u16();
        self.headers = res.headers().clone();
        if let Some(sc) = self.headers.get("set-cookie").and_then(|v| v.to_str().ok()) {
            if let Some(v) = sc.strip_prefix("drivich_session=") {
                let v = v.split(';').next().unwrap_or("").to_string();
                if v.is_empty() {
                    self.cookies.remove(who);
                } else {
                    self.cookies.insert(who.to_string(), v);
                }
            }
        }
        self.body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    }

    fn json(&self) -> Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|_| panic!("not JSON: {}", String::from_utf8_lossy(&self.body)))
    }

    fn at(&self, pointer: &str) -> Value {
        let v = self.json();
        v.pointer(pointer)
            .cloned()
            .unwrap_or_else(|| panic!("no {pointer} in {v}"))
    }
}

/// `true` / `12` / `"text"` / bare text → JSON value.
fn literal(s: &str) -> Value {
    serde_json::from_str(s).unwrap_or_else(|_| Value::String(s.to_string()))
}

// ---- actors -------------------------------------------------------------------

async fn register(w: &mut Api, who: &str, admin: bool) {
    let email = w.email(who);
    users::create(&w.pool, &email, who, PASSWORD, admin, None)
        .await
        .unwrap();
    login(w, who).await;
    assert_eq!(w.status, 200, "login of {who} failed");
}

async fn login(w: &mut Api, who: &str) {
    let body = serde_json::json!({ "email": w.email(who), "password": PASSWORD });
    w.send(
        who,
        "POST",
        "/auth/login",
        &[],
        body.to_string().into_bytes(),
        true,
    )
    .await;
}

#[given(expr = "a user {word}")]
async fn a_user(w: &mut Api, who: String) {
    register(w, &who, false).await;
}

#[given(expr = "an administrator {word}")]
async fn an_admin(w: &mut Api, who: String) {
    register(w, &who, true).await;
}

#[given(expr = "{word} owns a folder {string} saved as {string}")]
async fn owns_folder(w: &mut Api, who: String, name: String, var: String) {
    let body = serde_json::json!({ "parent_id": null, "name": name });
    w.send(
        &who,
        "POST",
        "/nodes/folder",
        &[],
        body.to_string().into_bytes(),
        true,
    )
    .await;
    assert_eq!(w.status, 200, "{}", String::from_utf8_lossy(&w.body));
    let id = w.at("/id").as_str().unwrap().to_string();
    w.vars.insert(var, id);
}

#[given(expr = "{word} owns a file {string} containing {string} saved as {string}")]
async fn owns_file(w: &mut Api, who: String, name: String, content: String, var: String) {
    upload(w, &who, &name, &content, None).await;
    let id = w.at("/node/id").as_str().unwrap().to_string();
    w.vars.insert(var, id);
}

#[given(
    expr = "{word} owns a file {string} containing {string} in folder {string} saved as {string}"
)]
async fn owns_file_in(
    w: &mut Api,
    who: String,
    name: String,
    content: String,
    folder: String,
    var: String,
) {
    let parent = w.vars.get(&folder).cloned().expect("unknown folder alias");
    upload(w, &who, &name, &content, Some(parent)).await;
    let id = w.at("/node/id").as_str().unwrap().to_string();
    w.vars.insert(var, id);
}

async fn upload(w: &mut Api, who: &str, name: &str, content: &str, parent: Option<String>) {
    let body = serde_json::json!({ "parent_id": parent, "name": name, "size": content.len() });
    send_upload(w, who, body, content.as_bytes()).await;
}

async fn send_upload(w: &mut Api, who: &str, body: Value, content: &[u8]) {
    w.send(
        who,
        "POST",
        "/uploads",
        &[],
        body.to_string().into_bytes(),
        true,
    )
    .await;
    assert_eq!(w.status, 200, "{}", String::from_utf8_lossy(&w.body));
    let id = w.at("/upload/id").as_str().unwrap().to_string();
    w.send(
        who,
        "PATCH",
        &format!("/uploads/{id}"),
        &[("upload-offset", "0".into())],
        content.to_vec(),
        false,
    )
    .await;
    assert_eq!(w.status, 200, "{}", String::from_utf8_lossy(&w.body));
}

// ---- requests -------------------------------------------------------------------

#[when(expr = "{word} logs in with the password {string}")]
async fn logs_in_with(w: &mut Api, who: String, password: String) {
    let body = serde_json::json!({ "email": w.email(&who), "password": password });
    w.send(
        &who,
        "POST",
        "/auth/login",
        &[],
        body.to_string().into_bytes(),
        true,
    )
    .await;
}

#[when(expr = "{word} sends {word} {string}")]
async fn sends(w: &mut Api, who: String, method: String, path: String) {
    w.send(&who, &method, &path, &[], Vec::new(), false).await;
}

#[when(expr = "{word} sends {word} {string} with header {string} set to {string}")]
async fn sends_with_header(
    w: &mut Api,
    who: String,
    method: String,
    path: String,
    k: String,
    v: String,
) {
    let v = w.fill(&v);
    w.send(&who, &method, &path, &[(k.as_str(), v)], Vec::new(), false)
        .await;
}

#[when(expr = "{word} sends {word} {string} with JSON:")]
async fn sends_json(w: &mut Api, who: String, method: String, path: String, step: &Step) {
    let body = w.fill(step.docstring().expect("a JSON docstring"));
    w.send(&who, &method, &path, &[], body.into_bytes(), true)
        .await;
}

#[when(expr = "{word} sends {word} {string} with the raw body {string}")]
async fn sends_raw(w: &mut Api, who: String, method: String, path: String, body: String) {
    w.send(&who, &method, &path, &[], body.into_bytes(), false)
        .await;
}

#[when(expr = "{word} sends {word} {string} without the CSRF header")]
async fn sends_no_csrf(w: &mut Api, who: String, method: String, path: String) {
    w.skip_csrf = true;
    w.send(&who, &method, &path, &[], Vec::new(), false).await;
}

#[when(expr = "{word} fails to log in {int} times")]
async fn fails_login(w: &mut Api, who: String, n: u32) {
    for i in 0..n {
        let body = serde_json::json!({ "email": w.email(&who), "password": format!("wrong-password-{i}") });
        w.send(
            &who,
            "POST",
            "/auth/login",
            &[],
            body.to_string().into_bytes(),
            true,
        )
        .await;
    }
}

#[when(expr = "{word} replaces file {string} with {string}")]
async fn replaces_file(w: &mut Api, who: String, var: String, content: String) {
    let id = w.vars.get(&var).cloned().expect("unknown file alias");
    let body = serde_json::json!({ "replace_node_id": id, "size": content.len() });
    send_upload(w, &who, body, content.as_bytes()).await;
}

#[when(expr = "{word} uploads a profile picture")]
async fn uploads_avatar(w: &mut Api, who: String) {
    let png = png();
    w.send(
        &who,
        "PUT",
        "/me/avatar",
        &[("content-type", "image/png".into())],
        png,
        false,
    )
    .await;
}

#[then(expr = "the user {word} id is saved as {string}")]
async fn user_id_saved(w: &mut Api, who: String, var: String) {
    w.send(&who, "GET", "/me", &[], Vec::new(), false).await;
    let id = w.at("/user/id").as_str().unwrap().to_string();
    w.vars.insert(var, id);
}

const PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

fn png() -> Vec<u8> {
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, PNG_B64).unwrap()
}

#[given(expr = "{word} owns an image {string} saved as {string}")]
async fn owns_image(w: &mut Api, who: String, name: String, var: String) {
    let png = png();
    let body = serde_json::json!({ "parent_id": null, "name": name, "size": png.len() });
    send_upload(w, &who, body, &png).await;
    let id = w.at("/node/id").as_str().unwrap().to_string();
    w.vars.insert(var, id);
}

async fn blob_of(w: &Api, var: &str) -> Uuid {
    let id: Uuid = w.vars.get(var).expect("unknown alias").parse().unwrap();
    sqlx::query_scalar("SELECT blob_id FROM nodes WHERE id = $1")
        .bind(id)
        .fetch_one(&w.pool)
        .await
        .unwrap()
}

#[given(expr = "the worker has made the thumbnail of {string}")]
async fn thumbnail_made(w: &mut Api, var: String) {
    let blob = blob_of(w, &var).await;
    drivich_core::thumbs::make_thumbnail(&w.pool, &w.storage, blob, 64)
        .await
        .unwrap();
}

#[given(expr = "the worker has made the PDF preview of {string}")]
async fn preview_made(w: &mut Api, var: String) {
    let blob = blob_of(w, &var).await;
    let path = w.storage.preview_path(blob);
    tokio::fs::create_dir_all(path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&path, b"%PDF-1.4 fake").await.unwrap();
    sqlx::query("UPDATE blobs SET preview_state = 'ready' WHERE id = $1")
        .bind(blob)
        .execute(&w.pool)
        .await
        .unwrap();
}

#[then(expr = "the response header {string} is saved as {string}")]
async fn header_saved(w: &mut Api, name: String, var: String) {
    let v = w
        .headers
        .get(&name)
        .and_then(|v| v.to_str().ok())
        .expect("header")
        .to_string();
    w.vars.insert(var, v);
}

#[when(expr = "{word} signs out")]
async fn signs_out(w: &mut Api, who: String) {
    w.send(&who, "POST", "/auth/logout", &[], Vec::new(), false)
        .await;
    // The browser would drop the cookie; keep the old token to prove the session is dead.
}

// ---- assertions -----------------------------------------------------------------

#[then(expr = "the status is {int}")]
async fn status_is(w: &mut Api, code: u16) {
    assert_eq!(w.status, code, "body: {}", String::from_utf8_lossy(&w.body));
}

#[then(expr = "the JSON at {string} is {string}")]
async fn json_is(w: &mut Api, pointer: String, expected: String) {
    assert_eq!(w.at(&pointer), literal(&w.fill(&expected)));
}

#[then(expr = "the JSON at {string} has {int} items")]
async fn json_len(w: &mut Api, pointer: String, n: usize) {
    assert_eq!(w.at(&pointer).as_array().expect("an array").len(), n);
}

#[then(expr = "the JSON at {string} has at least {int} items")]
async fn json_len_min(w: &mut Api, pointer: String, n: usize) {
    assert!(w.at(&pointer).as_array().expect("an array").len() >= n);
}

#[then(expr = "the JSON at {string} is saved as {string}")]
async fn json_saved(w: &mut Api, pointer: String, var: String) {
    let v = match w.at(&pointer) {
        Value::String(s) => s,
        other => other.to_string(),
    };
    w.vars.insert(var, v);
}

#[then(expr = "the JSON at {string} is not null")]
async fn json_not_null(w: &mut Api, pointer: String) {
    assert!(!w.at(&pointer).is_null());
}

#[then(expr = "the response header {string} starts with {string}")]
async fn header_starts(w: &mut Api, name: String, prefix: String) {
    let v = w
        .headers
        .get(&name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(v.starts_with(&prefix), "{name}: {v:?}");
}

#[then(expr = "the response body is {string}")]
async fn body_is(w: &mut Api, expected: String) {
    assert_eq!(String::from_utf8_lossy(&w.body), expected);
}

// ---- ONLYOFFICE ---------------------------------------------------------------

/// A stand-in Document Server: accepts `/command`, serves a few documents.
async fn mock_document_server() -> String {
    use axum::routing::{get, post};
    let app = Router::new()
        .route("/command", post(|| async { "{\"error\":0}" }))
        .route(
            "/cache/doc",
            get(|| async { "edited by the Document Server" }),
        )
        .route("/cache/empty", get(|| async { "" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut k = [0u8; 64];
    if key.len() > 64 {
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
    outer.finalize().to_vec()
}

fn jwt(secret: &str, claims: &Value) -> String {
    use base64::Engine;
    let b64 = &base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let signing = format!(
        "{}.{}",
        b64.encode(br#"{"alg":"HS256","typ":"JWT"}"#),
        b64.encode(claims.to_string())
    );
    let sig = b64.encode(hmac_sha256(secret.as_bytes(), signing.as_bytes()));
    format!("{signing}.{sig}")
}

#[given("the server trusts proxy headers")]
async fn trusts_proxy(w: &mut Api) {
    w.state_cfg.trust_proxy = true;
    w.rebuild();
}

#[given(expr = "the server accepts files up to {int} bytes")]
async fn max_file(w: &mut Api, n: u64) {
    w.state_cfg.max_file_bytes = n;
    w.rebuild();
}

#[given("a database backup exists")]
async fn backup_exists(w: &mut Api) {
    let dir = w.storage.backups_dir();
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("drivich-20260101-000000.dump"), b"dump")
        .await
        .unwrap();
}

#[when(expr = "{word} logs in with the password {string} using header {string} set to {string}")]
async fn logs_in_with_header(w: &mut Api, who: String, password: String, k: String, v: String) {
    let body = serde_json::json!({ "email": w.email(&who), "password": password });
    w.send(
        &who,
        "POST",
        "/auth/login",
        &[(k.as_str(), v)],
        body.to_string().into_bytes(),
        true,
    )
    .await;
}

#[given("document editing is disabled")]
async fn office_disabled(w: &mut Api) {
    w.office_secret = None;
    w.rebuild();
}

#[given("the Document Server is unreachable")]
async fn office_down(w: &mut Api) {
    w.office_url = "http://127.0.0.1:1".into();
    w.rebuild();
}

#[then(expr = "the last path segment of the JSON at {string} is saved as {string}")]
async fn last_segment_saved(w: &mut Api, pointer: String, var: String) {
    let v = w.at(&pointer);
    let url = v.as_str().expect("a string");
    w.vars
        .insert(var, url.rsplit('/').next().unwrap().to_string());
}

#[when(expr = "{word} sends {word} {string} with JSON signed by the Document Server:")]
async fn sends_signed(w: &mut Api, who: String, method: String, path: String, step: &Step) {
    let body = w.fill(step.docstring().expect("a JSON docstring"));
    let claims: Value = serde_json::from_str(&body).unwrap();
    let token = jwt(OFFICE_SECRET, &serde_json::json!({ "payload": claims }));
    w.send(
        &who,
        &method,
        &path,
        &[("authorization", format!("Bearer {token}"))],
        body.into_bytes(),
        true,
    )
    .await;
}

#[when(expr = "{word} sends {word} {string} with JSON carrying its own signature:")]
async fn sends_body_signed(w: &mut Api, who: String, method: String, path: String, step: &Step) {
    let body = w.fill(step.docstring().expect("a JSON docstring"));
    let mut claims: Value = serde_json::from_str(&body).unwrap();
    claims["token"] = Value::String(jwt(OFFICE_SECRET, &claims.clone()));
    w.send(
        &who,
        &method,
        &path,
        &[],
        claims.to_string().into_bytes(),
        true,
    )
    .await;
}

#[when(expr = "{word} sends {word} {string} with JSON signed with the wrong secret:")]
async fn sends_badly_signed(w: &mut Api, who: String, method: String, path: String, step: &Step) {
    let body = w.fill(step.docstring().expect("a JSON docstring"));
    let claims: Value = serde_json::from_str(&body).unwrap();
    let token = jwt("some-other-secret-value", &claims);
    w.send(
        &who,
        &method,
        &path,
        &[("authorization", format!("Bearer {token}"))],
        body.into_bytes(),
        true,
    )
    .await;
}

#[then(expr = "the document {string} contains {string}")]
async fn document_contains(w: &mut Api, var: String, text: String) {
    let id = w.vars.get(&var).cloned().expect("unknown alias");
    w.send(
        "Alice",
        "GET",
        &format!("/nodes/{id}/download"),
        &[],
        Vec::new(),
        false,
    )
    .await;
    assert_eq!(String::from_utf8_lossy(&w.body), text);
}

#[tokio::main]
async fn main() {
    // One scenario at a time: they share a database and run migrations on start.
    Api::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/features"))
        .await;
}
