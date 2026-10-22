// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Worker code that shells out (ffmpeg, LibreOffice, pg_dump) is exercised with
//! tiny stand-in programs placed first on `PATH`, so no real tool is needed.
//! Needs a PostgreSQL: `DATABASE_URL=postgres://… cargo test -p drivich-core --test workers`

#![cfg(unix)]

use bytes::Bytes;
use drivich_core::config::Config;
use drivich_core::jobs::{self, JobControls};
use drivich_core::models::Node;
use drivich_core::storage::Storage;
use drivich_core::uploads::{self, Limits};
use drivich_core::{backups, db, office, thumbs, users};
use futures_util::stream;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::OnceLock;
use uuid::Uuid;

const FFMPEG: &str = r#"#!/bin/sh
src=""; prev=""; seek=""
for a in "$@"; do
  [ "$prev" = "-i" ] && src="$a"
  [ "$prev" = "-ss" ] && seek="$a"
  prev="$a"; last="$a"
done
grep -q FAILME "$src" && exit 1
grep -q FIRSTFAIL "$src" && [ "$seek" = "1" ] && exit 1
grep -q EMPTYFRAME "$src" && { : > "$last"; exit 0; }
printf 'jpegdata' > "$last"
"#;

const SOFFICE: &str = r#"#!/bin/sh
[ "$1" = "--version" ] && exit 0
out=""; prev=""
for a in "$@"; do
  [ "$prev" = "--outdir" ] && out="$a"
  prev="$a"; input="$a"
done
grep -q FAILEXIT "$input" && exit 1
grep -q NOOUTPUT "$input" && exit 0
grep -q EMPTYPDF "$input" && { : > "$out/document.pdf"; exit 0; }
grep -q NOTPDF "$input" && { printf 'hello world' > "$out/document.pdf"; exit 0; }
printf '%%PDF-1.4 fake' > "$out/document.pdf"
"#;

const PG_DUMP: &str = r#"#!/bin/sh
[ "$PGDATABASE" = "failing" ] && { echo "boom" >&2; exit 1; }
prev=""
for a in "$@"; do
  [ "$prev" = "--file" ] && printf 'dump' > "$a"
  prev="$a"
done
"#;

/// Fake tools first on PATH, and a configuration in the environment. Done once,
/// before any test touches the environment.
fn setup() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("drivich-fakebin-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        for (name, body) in [
            ("ffmpeg", FFMPEG),
            ("soffice", SOFFICE),
            ("pg_dump", PG_DUMP),
        ] {
            let p = dir.join(name);
            std::fs::write(&p, body).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let path = format!(
            "{}:{}",
            dir.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        // SAFETY: runs once, before any other test in this binary reads the environment.
        unsafe {
            std::env::set_var("PATH", path);
            std::env::set_var("DRIVICH_DATA_DIR", "/srv/drivich");
            std::env::set_var("DRIVICH_MAX_FILE_BYTES", "1234");
            std::env::set_var("DRIVICH_COOKIE_SECURE", "true");
            std::env::set_var("DRIVICH_THUMB_MAX_PX", "99999");
            std::env::set_var("DRIVICH_MAX_VERSIONS", "0");
            std::env::set_var("DRIVICH_WORKER_CONCURRENCY", "0");
            std::env::set_var("DRIVICH_ONLYOFFICE_JWT_SECRET", "  0123456789abcdef0123  ");
            std::env::set_var("DRIVICH_ONLYOFFICE_URL", "http://office.test/");
            std::env::set_var("DRIVICH_ONLYOFFICE_APP_URL", "   ");
        }
        dir
    })
}

struct Ctx {
    pool: db::Pool,
    storage: Storage,
    limits: Limits,
    user: Uuid,
}

async fn ctx() -> Ctx {
    setup();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must point to a test PostgreSQL");
    let pool = db::connect(&url, 5).await.unwrap();
    db::migrate(&pool).await.unwrap();
    let storage = Storage::new(std::env::temp_dir().join(format!("drivich-w-{}", Uuid::new_v4())));
    storage.init().await.unwrap();
    let email = format!("w-{}@example.test", Uuid::new_v4().simple());
    let user = users::create(&pool, &email, "W", "a-long-enough-password", false, None)
        .await
        .unwrap()
        .id;
    Ctx {
        pool,
        storage,
        limits: Limits {
            max_file_bytes: 0,
            ttl_hours: 1,
            max_versions: 3,
        },
        user,
    }
}

async fn put(c: &Ctx, name: &str, data: &[u8]) -> (Node, Uuid) {
    let info = uploads::create(
        &c.pool,
        &c.storage,
        &c.limits,
        c.user,
        None,
        name,
        data.len() as i64,
    )
    .await
    .unwrap();
    let body = stream::iter(vec![Ok::<Bytes, std::io::Error>(Bytes::copy_from_slice(
        data,
    ))]);
    let node = uploads::append(&c.pool, &c.storage, &c.limits, c.user, info.id, 0, body)
        .await
        .unwrap()
        .node
        .unwrap();
    let blob: Uuid = sqlx::query_scalar("SELECT blob_id FROM nodes WHERE id = $1")
        .bind(node.id)
        .fetch_one(&c.pool)
        .await
        .unwrap();
    (node, blob)
}

async fn state(c: &Ctx, col: &str, blob: Uuid) -> String {
    sqlx::query_scalar(&format!("SELECT {col} FROM blobs WHERE id = $1"))
        .bind(blob)
        .fetch_one(&c.pool)
        .await
        .unwrap()
}

/// A PNG header announcing a huge picture (no pixel data needed to be refused).
fn huge_png() -> Vec<u8> {
    fn crc(data: &[u8]) -> u32 {
        let mut c = !0u32;
        for b in data {
            c ^= *b as u32;
            for _ in 0..8 {
                c = if c & 1 == 1 {
                    (c >> 1) ^ 0xEDB8_8320
                } else {
                    c >> 1
                };
            }
        }
        !c
    }
    let mut ihdr = b"IHDR".to_vec();
    ihdr.extend(20_000u32.to_be_bytes());
    ihdr.extend(20_000u32.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    out.extend(13u32.to_be_bytes());
    out.extend(&ihdr);
    out.extend(crc(&ihdr).to_be_bytes());
    out
}

const PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

fn png() -> Vec<u8> {
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, PNG_B64).unwrap()
}

#[tokio::test]
async fn thumbnails_cover_images_videos_and_failures() {
    let c = ctx().await;
    let (_, img) = put(&c, "pic.png", &png()).await;
    thumbs::make_thumbnail(&c.pool, &c.storage, img, 64)
        .await
        .unwrap();
    assert_eq!(state(&c, "thumb_state", img).await, "ready");

    let (_, txt) = put(&c, "n.txt", b"hello").await;
    thumbs::make_thumbnail(&c.pool, &c.storage, txt, 64)
        .await
        .unwrap();
    assert_eq!(state(&c, "thumb_state", txt).await, "unsupported");

    let (_, broken) = put(&c, "broken.png", b"\x89PNG not really").await;
    thumbs::make_thumbnail(&c.pool, &c.storage, broken, 64)
        .await
        .unwrap();
    assert_eq!(state(&c, "thumb_state", broken).await, "failed");

    let (_, huge) = put(&c, "huge.png", &huge_png()).await;
    thumbs::make_thumbnail(&c.pool, &c.storage, huge, 64)
        .await
        .unwrap();
    assert_eq!(state(&c, "thumb_state", huge).await, "failed");

    for (name, body, want) in [
        ("ok.mp4", &b"video"[..], "ready"),
        ("retry.mp4", &b"FIRSTFAIL"[..], "ready"),
        ("bad.mp4", &b"FAILME"[..], "failed"),
        ("empty.mp4", &b"EMPTYFRAME"[..], "failed"),
    ] {
        let (_, blob) = put(&c, name, body).await;
        thumbs::make_thumbnail(&c.pool, &c.storage, blob, 64)
            .await
            .unwrap();
        assert_eq!(state(&c, "thumb_state", blob).await, want, "{name}");
    }

    // A blob that vanished meanwhile is not an error.
    thumbs::make_thumbnail(&c.pool, &c.storage, Uuid::new_v4(), 64)
        .await
        .unwrap();
}

#[tokio::test]
async fn checksums_and_text_indexing() {
    let c = ctx().await;
    let (_, a) = put(&c, "a.txt", b"alpha beta").await;
    thumbs::compute_checksum(&c.pool, &c.storage, a)
        .await
        .unwrap();
    let sum: Option<String> = sqlx::query_scalar("SELECT sha256 FROM blobs WHERE id = $1")
        .bind(a)
        .fetch_one(&c.pool)
        .await
        .unwrap();
    assert_eq!(sum.unwrap().len(), 64);
    thumbs::compute_checksum(&c.pool, &c.storage, Uuid::new_v4())
        .await
        .unwrap();

    thumbs::index_text(&c.pool, &c.storage, a).await.unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT indexed FROM blobs WHERE id = $1")
            .bind(a)
            .fetch_one(&c.pool)
            .await
            .unwrap()
    );
    let (_, bin) = put(&c, "bin.txt", b"a\0b").await;
    thumbs::index_text(&c.pool, &c.storage, bin).await.unwrap();
    thumbs::index_text(&c.pool, &c.storage, Uuid::new_v4())
        .await
        .unwrap();
}

#[tokio::test]
async fn orphan_blobs_are_collected() {
    let c = ctx().await;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO blobs (id, size, mime, created_at) VALUES ($1, 1, 'text/plain', now() - interval '3 hours')")
        .bind(id)
        .execute(&c.pool)
        .await
        .unwrap();
    let removed = thumbs::gc_orphan_blobs(&c.pool, &c.storage).await.unwrap();
    assert!(removed >= 1);
}

#[tokio::test]
async fn office_previews_are_generated_or_marked() {
    let c = ctx().await;
    let docx = "a.docx";
    for (body, want) in [
        (&b"fine"[..], "ready"),
        (&b"FAILEXIT"[..], "failed"),
        (&b"NOOUTPUT"[..], "failed"),
        (&b"EMPTYPDF"[..], "failed"),
        (&b"NOTPDF"[..], "failed"),
    ] {
        let (_, blob) = put(&c, docx, body).await;
        office::generate(&c.pool, &c.storage, blob).await.unwrap();
        assert_eq!(office::state_of(&c.pool, blob).await.unwrap(), want);
    }
    // Other formats get the right input extension and a text file is unsupported.
    for name in [
        "a.doc", "a.xlsx", "a.xls", "a.pptx", "a.ppt", "a.odt", "a.ods", "a.odp", "a.rtf",
    ] {
        let (_, blob) = put(&c, name, b"fine").await;
        office::generate(&c.pool, &c.storage, blob).await.unwrap();
        assert_eq!(
            office::state_of(&c.pool, blob).await.unwrap(),
            "ready",
            "{name}"
        );
    }
    let (_, txt) = put(&c, "t.txt", b"fine").await;
    office::generate(&c.pool, &c.storage, txt).await.unwrap();
    assert_eq!(office::state_of(&c.pool, txt).await.unwrap(), "unsupported");
    office::generate(&c.pool, &c.storage, Uuid::new_v4())
        .await
        .unwrap();

    assert!(office::converter_available().await);
    sqlx::query("UPDATE blobs SET preview_state = 'unsupported' WHERE id = $1")
        .bind(txt)
        .execute(&c.pool)
        .await
        .unwrap();
    office::reset_unsupported(&c.pool).await.unwrap();
}

#[tokio::test]
async fn job_queue_controls_and_retries() {
    let c = ctx().await;
    // Validation.
    let mut limits = std::collections::BTreeMap::new();
    limits.insert("thumbnail".to_string(), 99);
    assert!(
        jobs::set_controls(
            &c.pool,
            &JobControls {
                paused: vec![],
                limits
            }
        )
        .await
        .is_err()
    );
    assert!(
        jobs::set_controls(
            &c.pool,
            &JobControls {
                paused: vec!["nonsense".into()],
                limits: Default::default()
            }
        )
        .await
        .is_err()
    );

    // A failing job is retried, then given up on, then retried by the admin.
    jobs::enqueue(
        &c.pool,
        jobs::kind::GC_BLOBS,
        serde_json::json!({}),
        Some("w-test-fail"),
    )
    .await
    .unwrap();
    let mut seen = 0;
    while let Some(job) = jobs::claim(&c.pool).await.unwrap() {
        if job.kind != jobs::kind::GC_BLOBS {
            jobs::complete(&c.pool, job.id).await.unwrap();
            continue;
        }
        seen += 1;
        let mut j = job.clone();
        j.attempts = j.max_attempts; // out of attempts
        jobs::fail(&c.pool, &j, "kaboom").await.unwrap();
        break;
    }
    assert_eq!(seen, 1);
    assert!(!jobs::recent_failures(&c.pool).await.unwrap().is_empty());
    assert!(!jobs::stats(&c.pool).await.unwrap().is_empty());
    assert!(jobs::retry_failed(&c.pool).await.unwrap() >= 1);

    // Concurrency limits: zero blocks the kind, one blocks while one runs.
    let mut limits = std::collections::BTreeMap::new();
    limits.insert(jobs::kind::GC_BLOBS.to_string(), 0);
    jobs::set_controls(
        &c.pool,
        &JobControls {
            paused: vec![],
            limits,
        },
    )
    .await
    .unwrap();
    let blocked = jobs::claim(&c.pool).await.unwrap();
    assert!(blocked.is_none_or(|j| j.kind != jobs::kind::GC_BLOBS));
    jobs::set_controls(&c.pool, &JobControls::default())
        .await
        .unwrap();

    let mut limits = std::collections::BTreeMap::new();
    limits.insert(jobs::kind::GC_BLOBS.to_string(), 1);
    jobs::set_controls(
        &c.pool,
        &JobControls {
            paused: vec![],
            limits,
        },
    )
    .await
    .unwrap();
    let _ = jobs::claim(&c.pool).await.unwrap();
    jobs::set_controls(&c.pool, &JobControls::default())
        .await
        .unwrap();

    // Housekeeping helpers.
    jobs::enqueue_periodic(&c.pool, jobs::kind::CLEANUP_SESSIONS, 60)
        .await
        .unwrap();
    jobs::enqueue_periodic(&c.pool, jobs::kind::CLEANUP_SESSIONS, 60)
        .await
        .unwrap();
    jobs::prune(&c.pool, 7).await.unwrap();
    let (_, _) = put(&c, "queued.png", &png()).await;
    jobs::requeue_pending(&c.pool).await.unwrap();
}

#[tokio::test]
async fn database_dumps_use_pg_dump() {
    let c = ctx().await;
    let name = backups::create(&c.storage, "postgres://u:p%40ss@db.test:5433/drivich")
        .await
        .unwrap();
    assert!(backups::is_valid_name(&name));
    let listed = backups::list(&c.storage).await.unwrap();
    assert!(listed.iter().any(|b| b.name == name && b.size == 4));
    assert!(backups::backup_path(&c.storage, &name).unwrap().exists());

    let err = backups::create(&c.storage, "postgres://u:p@db.test/failing")
        .await
        .unwrap_err();
    assert!(!err.to_string().is_empty());
    assert!(backups::create(&c.storage, "not a url").await.is_err());

    // Strangers in the folder are ignored; a missing folder lists nothing.
    tokio::fs::write(c.storage.backups_dir().join("notes.txt"), b"x")
        .await
        .unwrap();
    assert_eq!(backups::list(&c.storage).await.unwrap().len(), listed.len());
    let fresh = Storage::new(std::env::temp_dir().join(format!("drivich-none-{}", Uuid::new_v4())));
    assert!(backups::list(&fresh).await.unwrap().is_empty());
}

#[tokio::test]
async fn configuration_is_read_from_the_environment() {
    setup();
    let url = std::env::var("DATABASE_URL").unwrap();
    let cfg = Config::from_env().unwrap();
    assert_eq!(cfg.database_url, url);
    assert_eq!(cfg.data_dir, PathBuf::from("/srv/drivich"));
    assert_eq!(cfg.max_file_bytes, 1234);
    assert!(cfg.cookie_secure);
    assert_eq!(cfg.thumb_max_px, 2048);
    assert_eq!(cfg.max_versions, 1);
    assert_eq!(cfg.worker_concurrency, 1);
    assert_eq!(
        cfg.office_jwt_secret.as_deref(),
        Some("0123456789abcdef0123")
    );
    assert_eq!(cfg.office_server_url, "http://office.test");
    assert_eq!(cfg.office_app_url, "http://server:2283");
    assert_eq!(cfg.bind, "0.0.0.0:2283");
}

#[tokio::test]
async fn storage_refuses_an_unmounted_volume() {
    let dir = std::env::temp_dir().join(format!("drivich-mount-{}", Uuid::new_v4()));
    let storage = Storage::new(dir);
    storage.init().await.unwrap();
    // The database claims files exist but the folder is empty and unmarked.
    assert!(storage.verify_mounts(true, false).await.is_err());
    assert!(storage.verify_mounts(true, true).await.is_ok());
    assert!(storage.verify_mounts(false, false).await.is_ok());
    // Now marked: fine even with files claimed.
    assert!(storage.verify_mounts(true, false).await.is_ok());
    assert!(storage.office_tmp_dir().ends_with("tmp/office"));
    let (free, total) = storage.disk_space().unwrap();
    assert!(total >= free);
}

#[test]
fn mime_sniffing_knows_common_formats() {
    use drivich_core::storage::detect_mime;
    assert_eq!(detect_mime("x", &[0xFF, 0xD8, 0xFF, 0]), "image/jpeg");
    assert_eq!(detect_mime("x", b"GIF89a.."), "image/gif");
    assert_eq!(detect_mime("x", b"RIFF\0\0\0\0WEBPVP8 "), "image/webp");
    assert_eq!(detect_mime("x.txt", b"hello"), "text/plain");
}
