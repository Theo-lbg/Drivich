// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Edge cases of uploads, trash and housekeeping against a real PostgreSQL.

use bytes::Bytes;
use drivich_core::error::CoreError;
use drivich_core::models::Role;
use drivich_core::storage::Storage;
use drivich_core::uploads::{self, Limits};
use drivich_core::{db, nodes, shares, users};
use futures_util::stream;
use uuid::Uuid;

struct Ctx {
    pool: db::Pool,
    storage: Storage,
    limits: Limits,
}

async fn ctx() -> Ctx {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must point to a test PostgreSQL");
    let pool = db::connect(&url, 5).await.unwrap();
    db::migrate(&pool).await.unwrap();
    let storage = Storage::new(std::env::temp_dir().join(format!("drivich-m-{}", Uuid::new_v4())));
    storage.init().await.unwrap();
    Ctx {
        pool,
        storage,
        limits: Limits {
            max_file_bytes: 0,
            ttl_hours: 1,
            max_versions: 3,
        },
    }
}

async fn user(c: &Ctx, tag: &str) -> Uuid {
    let email = format!("{tag}-{}@example.test", Uuid::new_v4().simple());
    users::create(&c.pool, &email, tag, "a-long-enough-password", false, None)
        .await
        .unwrap()
        .id
}

async fn put(c: &Ctx, user: Uuid, name: &str, data: &[u8]) -> drivich_core::models::Node {
    let info = uploads::create(
        &c.pool,
        &c.storage,
        &c.limits,
        user,
        None,
        name,
        data.len() as i64,
    )
    .await
    .unwrap();
    let body = stream::iter(vec![Ok::<Bytes, std::io::Error>(Bytes::copy_from_slice(
        data,
    ))]);
    uploads::append(&c.pool, &c.storage, &c.limits, user, info.id, 0, body)
        .await
        .unwrap()
        .node
        .unwrap()
}

const PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

fn png() -> Vec<u8> {
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, PNG_B64).unwrap()
}

#[tokio::test]
async fn blank_documents_and_their_names() {
    let c = ctx().await;
    let u = user(&c, "alice").await;
    for (kind, name, want) in [
        ("docx", "Report", "Report.docx"),
        ("xlsx", "Sheet.XLSX", "Sheet.XLSX"),
        ("pptx", "Deck", "Deck.pptx"),
        ("md", "Notes", "Notes.md"),
        ("txt", "Plain", "Plain.txt"),
    ] {
        let n = uploads::create_blank(&c.pool, &c.storage, &c.limits, u, None, kind, name)
            .await
            .unwrap();
        assert_eq!(n.name, want);
    }
    assert!(
        uploads::create_blank(&c.pool, &c.storage, &c.limits, u, None, "exe", "x")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn upload_session_edge_cases() {
    let c = ctx().await;
    let u = user(&c, "alice").await;

    // finish_empty refuses a session that expects bytes.
    let info = uploads::create(&c.pool, &c.storage, &c.limits, u, None, "x.bin", 5)
        .await
        .unwrap();
    assert!(
        uploads::finish_empty(&c.pool, &c.storage, &c.limits, u, info.id)
            .await
            .is_err()
    );

    // An expired session is gone for the client, and is cleaned up.
    sqlx::query("UPDATE upload_sessions SET expires_at = now() - interval '1 hour' WHERE id = $1")
        .bind(info.id)
        .execute(&c.pool)
        .await
        .unwrap();
    assert!(matches!(
        uploads::status(&c.pool, &c.storage, u, info.id).await,
        Err(CoreError::NotFound)
    ));

    // Leftover partial files: old orphans go, fresh ones and strangers stay.
    let dir = c.storage.root().join("tmp/uploads");
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let old = dir.join(format!("{}.part", Uuid::new_v4().simple()));
    tokio::fs::write(&old, b"old").await.unwrap();
    std::fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3 * 86_400))
        .unwrap();
    let fresh = dir.join(format!("{}.part", Uuid::new_v4().simple()));
    tokio::fs::write(&fresh, b"new").await.unwrap();
    tokio::fs::write(dir.join("README"), b"x").await.unwrap();
    tokio::fs::write(dir.join("not-a-uuid.part"), b"x")
        .await
        .unwrap();
    let removed = uploads::cleanup_expired(&c.pool, &c.storage).await.unwrap();
    assert!(removed >= 2);
    assert!(!old.exists());
    assert!(fresh.exists());

    // Too many sessions at once.
    let mut last = None;
    for i in 0..201 {
        match uploads::create(&c.pool, &c.storage, &c.limits, u, None, &format!("f{i}"), 1).await {
            Ok(_) => {}
            Err(e) => {
                last = Some(e);
                break;
            }
        }
    }
    assert!(matches!(last, Some(CoreError::TooManyRequests)));
}

#[tokio::test]
async fn replacing_an_image_queues_a_thumbnail() {
    let c = ctx().await;
    let u = user(&c, "alice").await;
    let n = put(&c, u, "pic.png", &png()).await;
    let info = uploads::create_version(&c.pool, &c.storage, &c.limits, u, n.id, png().len() as i64)
        .await
        .unwrap();
    let body = stream::iter(vec![Ok::<Bytes, std::io::Error>(Bytes::from(png()))]);
    uploads::append(&c.pool, &c.storage, &c.limits, u, info.id, 0, body)
        .await
        .unwrap();
    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM blobs WHERE thumb_state = 'pending'")
            .fetch_one(&c.pool)
            .await
            .unwrap();
    assert!(pending >= 1);

    // The file disappears into the trash during a long upload.
    let info = uploads::create_version(&c.pool, &c.storage, &c.limits, u, n.id, 3)
        .await
        .unwrap();
    nodes::trash(&c.pool, u, n.id).await.unwrap();
    let body = stream::iter(vec![Ok::<Bytes, std::io::Error>(Bytes::from_static(
        b"abc",
    ))]);
    assert!(
        uploads::append(&c.pool, &c.storage, &c.limits, u, info.id, 0, body)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn trash_rules_and_purging() {
    let c = ctx().await;
    let (a, b) = (user(&c, "alice").await, user(&c, "bob").await);
    let f = put(&c, a, "f.txt", b"x").await;

    // Trashing twice and restoring a live item are harmless.
    nodes::restore(&c.pool, a, f.id).await.unwrap();
    nodes::trash(&c.pool, a, f.id).await.unwrap();
    nodes::trash(&c.pool, a, f.id).await.unwrap();
    assert!(nodes::set_starred(&c.pool, a, f.id, true).await.is_err());

    // Old trash is purged for good.
    sqlx::query("UPDATE nodes SET trashed_at = now() - interval '90 days' WHERE id = $1")
        .bind(f.id)
        .execute(&c.pool)
        .await
        .unwrap();
    assert!(
        nodes::purge_old_trash(&c.pool, &c.storage, 30)
            .await
            .unwrap()
            >= 1
    );
    assert!(nodes::get(&c.pool, a, f.id).await.is_err());

    // Descriptions are bounded; items cannot cross into another person's Drive.
    let g = put(&c, a, "g.txt", b"x").await;
    let long = "x".repeat(2001);
    assert!(
        nodes::set_description(&c.pool, a, g.id, Some(&long))
            .await
            .is_err()
    );
    let folder = nodes::create_folder(&c.pool, a, None, "Shared")
        .await
        .unwrap();
    shares::add_user(
        &c.pool,
        a,
        folder.id,
        &users::get(&c.pool, b).await.unwrap().email,
        "editor",
    )
    .await
    .unwrap();
    let bobs = put(&c, b, "bobs.txt", b"x").await;
    assert!(
        nodes::move_to(&c.pool, b, bobs.id, Some(folder.id))
            .await
            .is_err()
    );
    assert_eq!(
        nodes::load_ref(&c.pool, b, folder.id, Role::Editor)
            .await
            .unwrap()
            .id,
        folder.id
    );

    // Public listing paginates.
    put(&c, a, "h.txt", b"x").await;
    let inside = nodes::create_folder(&c.pool, a, Some(folder.id), "Inside")
        .await
        .unwrap();
    nodes::create_folder(&c.pool, a, Some(folder.id), "Inside 2")
        .await
        .unwrap();
    let page = nodes::list_public_children(&c.pool, folder.id, 1, None)
        .await
        .unwrap();
    assert!(page.next_cursor.is_some());
    assert_eq!(page.items.len(), 1);
    let _ = inside;
}
