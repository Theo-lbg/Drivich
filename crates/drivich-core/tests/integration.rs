// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Integration tests against a real PostgreSQL (set `DATABASE_URL`).
//! Run: `DATABASE_URL=postgres://… cargo test -p drivich-core --test integration`

use bytes::Bytes;
use drivich_core::error::CoreError;
use drivich_core::models::Role;
use drivich_core::nodes::{self, ListParams, SortBy, View};
use drivich_core::storage::Storage;
use drivich_core::uploads::{self, Limits};
use drivich_core::{db, jobs, perms, search, shares, thumbs, users};
use futures_util::stream;
use uuid::Uuid;

/// The job-queue tests share one global queue: run them one at a time.
static JOBS_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct Ctx {
    pool: db::Pool,
    storage: Storage,
    limits: Limits,
}

async fn ctx() -> Ctx {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must point to a test PostgreSQL");
    let pool = db::connect(&url, 5).await.unwrap();
    db::migrate(&pool).await.unwrap();
    let dir = std::env::temp_dir().join(format!("drivich-test-{}", Uuid::new_v4()));
    let storage = Storage::new(dir);
    storage.init().await.unwrap();
    Ctx {
        pool,
        storage,
        limits: Limits {
            max_file_bytes: 0,
            ttl_hours: 1,
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

fn params(parent: Option<Uuid>, limit: i64, cursor: Option<String>) -> ListParams {
    ListParams {
        view: View::Drive,
        parent,
        sort: SortBy::Name,
        desc: false,
        type_filter: None,
        modified_from: None,
        owner: None,
        location: None,
        limit,
        cursor,
    }
}

async fn upload(
    c: &Ctx,
    user: Uuid,
    parent: Option<Uuid>,
    name: &str,
    data: &[u8],
) -> drivich_core::models::Node {
    let info = uploads::create(
        &c.pool,
        &c.storage,
        &c.limits,
        user,
        parent,
        name,
        data.len() as i64,
    )
    .await
    .unwrap();
    if data.is_empty() {
        return uploads::finish_empty(&c.pool, &c.storage, user, info.id)
            .await
            .unwrap();
    }
    let body = stream::iter(vec![Ok::<Bytes, std::io::Error>(Bytes::copy_from_slice(
        data,
    ))]);
    uploads::append(&c.pool, &c.storage, &c.limits, user, info.id, 0, body)
        .await
        .unwrap()
        .node
        .unwrap()
}

#[tokio::test]
async fn private_items_are_invisible_and_shares_grant_access() {
    let c = ctx().await;
    let (a, b) = (user(&c, "alice").await, user(&c, "bob").await);
    let folder = nodes::create_folder(&c.pool, a, None, "Projet")
        .await
        .unwrap();

    // Bob cannot even learn that the folder exists.
    assert!(matches!(
        nodes::get(&c.pool, b, folder.id).await,
        Err(CoreError::NotFound)
    ));
    assert!(matches!(
        nodes::rename(&c.pool, b, folder.id, "x").await,
        Err(CoreError::NotFound)
    ));

    shares::add_user(
        &c.pool,
        a,
        folder.id,
        &users::get(&c.pool, b).await.unwrap().email,
        "viewer",
    )
    .await
    .unwrap();
    assert_eq!(
        nodes::get(&c.pool, b, folder.id).await.unwrap().role,
        "viewer"
    );
    assert!(matches!(
        nodes::rename(&c.pool, b, folder.id, "x").await,
        Err(CoreError::Forbidden)
    ));
    assert!(matches!(
        nodes::create_folder(&c.pool, b, Some(folder.id), "nope").await,
        Err(CoreError::Forbidden)
    ));

    // Shares apply to the whole subtree.
    let child = nodes::create_folder(&c.pool, a, Some(folder.id), "Sous-dossier")
        .await
        .unwrap();
    assert!(nodes::get(&c.pool, b, child.id).await.is_ok());

    // Upgrade to editor: Bob can now write inside, new items belong to Alice.
    let entry = shares::add_user(
        &c.pool,
        a,
        folder.id,
        &users::get(&c.pool, b).await.unwrap().email,
        "editor",
    )
    .await
    .unwrap();
    let made = nodes::create_folder(&c.pool, b, Some(child.id), "Par Bob")
        .await
        .unwrap();
    assert_eq!(made.owner_id, a);
    // ...but only the owner manages sharing and trash.
    assert!(matches!(
        shares::list(&c.pool, b, folder.id).await,
        Err(CoreError::NotFound | CoreError::Forbidden)
    ));
    assert!(nodes::trash(&c.pool, b, child.id).await.is_err());

    // Revoking removes access immediately.
    shares::remove(&c.pool, a, entry.id).await.unwrap();
    assert!(matches!(
        nodes::get(&c.pool, b, child.id).await,
        Err(CoreError::NotFound)
    ));
}

#[tokio::test]
async fn trash_restore_and_permanent_delete() {
    let c = ctx().await;
    let a = user(&c, "alice").await;
    let folder = nodes::create_folder(&c.pool, a, None, "Dossier")
        .await
        .unwrap();
    let file = upload(&c, a, Some(folder.id), "note.txt", b"bonjour").await;
    let blob = perms::node_ref(&c.pool, file.id)
        .await
        .unwrap()
        .blob_id
        .unwrap();
    assert!(c.storage.blob_path(blob).exists());

    nodes::trash(&c.pool, a, folder.id).await.unwrap();
    let trash = nodes::list(
        &c.pool,
        a,
        ListParams {
            view: View::Trash,
            ..params(None, 50, None)
        },
    )
    .await
    .unwrap();
    assert_eq!(trash.items.len(), 1, "only the trash root is listed");
    assert!(
        nodes::get(&c.pool, a, file.id)
            .await
            .unwrap()
            .trashed_at
            .is_some()
    );
    assert!(
        nodes::list(&c.pool, a, params(None, 50, None))
            .await
            .unwrap()
            .items
            .is_empty()
    );

    nodes::restore(&c.pool, a, folder.id).await.unwrap();
    assert!(
        nodes::get(&c.pool, a, file.id)
            .await
            .unwrap()
            .trashed_at
            .is_none()
    );

    // Permanent delete needs the trash first, then frees disk space.
    assert!(
        nodes::delete_permanently(&c.pool, &c.storage, a, folder.id)
            .await
            .is_err()
    );
    nodes::trash(&c.pool, a, folder.id).await.unwrap();
    nodes::delete_permanently(&c.pool, &c.storage, a, folder.id)
        .await
        .unwrap();
    assert!(matches!(
        nodes::get(&c.pool, a, file.id).await,
        Err(CoreError::NotFound)
    ));
    assert!(!c.storage.blob_path(blob).exists());
}

#[tokio::test]
async fn folders_cannot_be_moved_into_themselves() {
    let c = ctx().await;
    let a = user(&c, "alice").await;
    let top = nodes::create_folder(&c.pool, a, None, "A").await.unwrap();
    let mid = nodes::create_folder(&c.pool, a, Some(top.id), "B")
        .await
        .unwrap();
    let low = nodes::create_folder(&c.pool, a, Some(mid.id), "C")
        .await
        .unwrap();
    assert!(
        nodes::move_to(&c.pool, a, top.id, Some(low.id))
            .await
            .is_err()
    );
    assert!(
        nodes::move_to(&c.pool, a, top.id, Some(top.id))
            .await
            .is_err()
    );
    let moved = nodes::move_to(&c.pool, a, low.id, None).await.unwrap();
    assert_eq!(moved.parent_id, None);
}

#[tokio::test]
async fn resumable_upload_survives_interruption_and_checks_offsets() {
    let c = ctx().await;
    let a = user(&c, "alice").await;
    let data: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
    let info = uploads::create(
        &c.pool,
        &c.storage,
        &c.limits,
        a,
        None,
        "gros.bin",
        data.len() as i64,
    )
    .await
    .unwrap();

    // First chunk of 4000 bytes.
    let first = stream::iter(vec![Ok::<Bytes, std::io::Error>(Bytes::copy_from_slice(
        &data[..4000],
    ))]);
    let out = uploads::append(&c.pool, &c.storage, &c.limits, a, info.id, 0, first)
        .await
        .unwrap();
    assert!(out.node.is_none());
    assert_eq!(out.upload.received, 4000);

    // A chunk at the wrong offset is refused and tells the client where to resume.
    let bad = stream::iter(vec![Ok::<Bytes, std::io::Error>(Bytes::from_static(b"zz"))]);
    match uploads::append(&c.pool, &c.storage, &c.limits, a, info.id, 123, bad).await {
        Err(CoreError::OffsetMismatch { current }) => assert_eq!(current, 4000),
        other => panic!("expected offset mismatch, got {other:?}"),
    }

    // A connection that breaks mid-chunk keeps the bytes that did arrive.
    let broken = stream::iter(vec![
        Ok::<Bytes, std::io::Error>(Bytes::copy_from_slice(&data[4000..5000])),
        Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "reset",
        )),
    ]);
    assert!(
        uploads::append(&c.pool, &c.storage, &c.limits, a, info.id, 4000, broken)
            .await
            .is_err()
    );
    let st = uploads::status(&c.pool, &c.storage, a, info.id)
        .await
        .unwrap();
    assert_eq!(st.received, 5000);

    // Another user cannot touch this upload.
    let b = user(&c, "bob").await;
    assert!(
        uploads::status(&c.pool, &c.storage, b, info.id)
            .await
            .is_err()
    );

    // Resume from 5000 and finish.
    let rest = stream::iter(vec![Ok::<Bytes, std::io::Error>(Bytes::copy_from_slice(
        &data[5000..],
    ))]);
    let done = uploads::append(&c.pool, &c.storage, &c.limits, a, info.id, 5000, rest)
        .await
        .unwrap();
    let node = done.node.expect("upload finalised");
    assert_eq!(node.size, 10_000);
    let blob = perms::node_ref(&c.pool, node.id)
        .await
        .unwrap()
        .blob_id
        .unwrap();
    assert_eq!(std::fs::read(c.storage.blob_path(blob)).unwrap(), data);
    assert!(
        !c.storage.upload_path(info.id).exists(),
        "temporary part file is gone"
    );

    // Sending more than announced is rejected.
    let info2 = uploads::create(&c.pool, &c.storage, &c.limits, a, None, "petit.txt", 3)
        .await
        .unwrap();
    let over = stream::iter(vec![Ok::<Bytes, std::io::Error>(Bytes::from_static(
        b"abcdef",
    ))]);
    assert!(
        uploads::append(&c.pool, &c.storage, &c.limits, a, info2.id, 0, over)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn quota_and_size_limits_are_enforced() {
    let c = ctx().await;
    let email = format!("quota-{}@example.test", Uuid::new_v4().simple());
    let q = users::create(
        &c.pool,
        &email,
        "quota",
        "a-long-enough-password",
        false,
        Some(1000),
    )
    .await
    .unwrap()
    .id;
    assert!(matches!(
        uploads::create(&c.pool, &c.storage, &c.limits, q, None, "trop.bin", 2000).await,
        Err(CoreError::QuotaExceeded)
    ));
    upload(&c, q, None, "ok.bin", &[7u8; 600]).await;
    assert!(matches!(
        uploads::create(&c.pool, &c.storage, &c.limits, q, None, "encore.bin", 600).await,
        Err(CoreError::QuotaExceeded)
    ));
    let strict = Limits {
        max_file_bytes: 10,
        ttl_hours: 1,
    };
    let a = user(&c, "alice").await;
    assert!(matches!(
        uploads::create(&c.pool, &c.storage, &strict, a, None, "x.bin", 11).await,
        Err(CoreError::TooLarge)
    ));
}

#[tokio::test]
async fn pagination_is_stable_and_complete() {
    let c = ctx().await;
    let a = user(&c, "alice").await;
    let parent = nodes::create_folder(&c.pool, a, None, "Liste")
        .await
        .unwrap();
    for i in 0..7 {
        nodes::create_folder(&c.pool, a, Some(parent.id), &format!("dossier-{i}"))
            .await
            .unwrap();
    }
    for i in 0..6 {
        upload(&c, a, Some(parent.id), &format!("fichier-{i}.txt"), b"x").await;
    }
    for desc in [false, true] {
        for sort in [SortBy::Name, SortBy::Modified, SortBy::Size] {
            let mut seen = Vec::new();
            let mut cursor = None;
            loop {
                let page = nodes::list(
                    &c.pool,
                    a,
                    ListParams {
                        sort,
                        desc,
                        ..params(Some(parent.id), 4, cursor)
                    },
                )
                .await
                .unwrap();
                seen.extend(page.items.iter().map(|n| n.id));
                match page.next_cursor {
                    Some(n) => cursor = Some(n),
                    None => break,
                }
            }
            let mut dedup = seen.clone();
            dedup.sort();
            dedup.dedup();
            assert_eq!(seen.len(), 13, "{sort:?} desc={desc}");
            assert_eq!(dedup.len(), 13, "no duplicates for {sort:?} desc={desc}");
        }
    }
}

#[tokio::test]
async fn search_respects_visibility_and_finds_content() {
    let c = ctx().await;
    let (a, b) = (user(&c, "alice").await, user(&c, "bob").await);
    let tag = Uuid::new_v4().simple().to_string();
    let secret = upload(
        &c,
        a,
        None,
        &format!("budget-{tag}.txt"),
        format!("le mot rarissime zorglub{tag} est ici").as_bytes(),
    )
    .await;
    let blob = perms::node_ref(&c.pool, secret.id)
        .await
        .unwrap()
        .blob_id
        .unwrap();
    thumbs::index_text(&c.pool, &c.storage, blob).await.unwrap();

    let sp = |q: &str| search::SearchParams {
        q: Some(q.to_string()),
        limit: 20,
        ..Default::default()
    };
    // Name search and content search both find it for the owner.
    assert_eq!(
        search::search(&c.pool, a, sp(&format!("budget-{tag}")))
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(
        search::search(&c.pool, a, sp(&format!("zorglub{tag}")))
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    // Bob sees nothing, neither in results nor in suggestions.
    assert!(
        search::search(&c.pool, b, sp(&format!("budget-{tag}")))
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        search::search(&c.pool, b, sp(&format!("zorglub{tag}")))
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        search::suggest(&c.pool, b, &format!("budget-{tag}"))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        search::suggest(&c.pool, a, &format!("budget-{tag}"))
            .await
            .unwrap()
            .len(),
        1
    );

    // After sharing, Bob finds it; after trashing, nobody but the owner does.
    shares::add_user(
        &c.pool,
        a,
        secret.id,
        &users::get(&c.pool, b).await.unwrap().email,
        "viewer",
    )
    .await
    .unwrap();
    assert_eq!(
        search::search(&c.pool, b, sp(&format!("budget-{tag}")))
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    nodes::trash(&c.pool, a, secret.id).await.unwrap();
    assert!(
        search::search(&c.pool, b, sp(&format!("budget-{tag}")))
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        search::search(&c.pool, a, sp(&format!("budget-{tag}")))
            .await
            .unwrap()
            .items
            .is_empty()
    );

    // Wildcards typed by the user are literal.
    assert!(
        search::search(&c.pool, a, sp("%"))
            .await
            .unwrap()
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn public_links_can_be_revoked_and_expire() {
    let c = ctx().await;
    let a = user(&c, "alice").await;
    let f = upload(&c, a, None, "public.txt", b"hello").await;
    let (link, token) = shares::create_link(&c.pool, a, f.id, None).await.unwrap();
    assert_eq!(
        shares::resolve_link(&c.pool, &token).await.unwrap().id,
        f.id
    );
    assert!(matches!(
        shares::resolve_link(&c.pool, "not-a-token").await,
        Err(CoreError::NotFound)
    ));
    assert!(
        shares::create_link(
            &c.pool,
            a,
            f.id,
            Some(chrono::Utc::now() - chrono::Duration::hours(1))
        )
        .await
        .is_err()
    );
    shares::revoke_link(&c.pool, a, link.id).await.unwrap();
    assert!(matches!(
        shares::resolve_link(&c.pool, &token).await,
        Err(CoreError::NotFound)
    ));

    // A link to a trashed item stops working.
    let (_, t2) = shares::create_link(&c.pool, a, f.id, None).await.unwrap();
    nodes::trash(&c.pool, a, f.id).await.unwrap();
    assert!(matches!(
        shares::resolve_link(&c.pool, &t2).await,
        Err(CoreError::NotFound)
    ));

    // Only the owner can create links.
    let b = user(&c, "bob").await;
    let g = upload(&c, a, None, "other.txt", b"x").await;
    assert!(shares::create_link(&c.pool, b, g.id, None).await.is_err());
}

#[tokio::test]
async fn job_queue_dedupes_claims_and_retries() {
    let _guard = JOBS_LOCK.lock().await;
    let c = ctx().await;
    let blob = Uuid::new_v4();
    jobs::enqueue_blob(&c.pool, jobs::kind::THUMBNAIL, blob)
        .await
        .unwrap();
    jobs::enqueue_blob(&c.pool, jobs::kind::THUMBNAIL, blob)
        .await
        .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs WHERE dedupe_key = $1")
        .bind(format!("thumbnail:{blob}"))
        .fetch_one(&c.pool)
        .await
        .unwrap();
    assert_eq!(n, 1, "identical pending jobs are merged");

    // Drain until we get ours (other tests may have queued jobs too).
    let mut mine = None;
    while let Some(j) = jobs::claim(&c.pool).await.unwrap() {
        if j.payload["blob_id"] == blob.to_string() {
            mine = Some(j);
            break;
        }
        jobs::complete(&c.pool, j.id).await.unwrap();
    }
    let job = mine.expect("job claimed");
    assert_eq!(job.attempts, 1);
    jobs::fail(&c.pool, &job, "boom").await.unwrap();
    let (status, err): (String, Option<String>) =
        sqlx::query_as("SELECT status, last_error FROM jobs WHERE id = $1")
            .bind(job.id)
            .fetch_one(&c.pool)
            .await
            .unwrap();
    assert_eq!(status, "queued", "failed job is rescheduled with backoff");
    assert_eq!(err.as_deref(), Some("boom"));
    sqlx::query("DELETE FROM jobs WHERE id = $1")
        .bind(job.id)
        .execute(&c.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn roles_enum_orders_correctly() {
    assert!(Role::Owner > Role::Editor && Role::Editor > Role::Viewer);
}

#[tokio::test]
async fn thumbnails_are_generated_for_images() {
    let c = ctx().await;
    let a = user(&c, "alice").await;
    // Build a 64x64 PNG in memory.
    let mut png = Vec::new();
    let img = image::RgbImage::from_pixel(64, 64, image::Rgb([200, 30, 30]));
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let n = upload(&c, a, None, "rouge.png", &png).await;
    assert_eq!(n.mime.as_deref(), Some("image/png"));
    let blob = perms::node_ref(&c.pool, n.id)
        .await
        .unwrap()
        .blob_id
        .unwrap();
    thumbs::make_thumbnail(&c.pool, &c.storage, blob, 32)
        .await
        .unwrap();
    let state: String = sqlx::query_scalar("SELECT thumb_state FROM blobs WHERE id = $1")
        .bind(blob)
        .fetch_one(&c.pool)
        .await
        .unwrap();
    assert_eq!(state, "ready");
    let t = image::open(c.storage.thumb_path(blob)).unwrap();
    assert!(t.width() <= 32 && t.height() <= 32);

    // Garbage with an image extension is marked failed, not retried forever.
    let bad = upload(&c, a, None, "faux.png", b"this is not a png").await;
    let bblob = perms::node_ref(&c.pool, bad.id)
        .await
        .unwrap()
        .blob_id
        .unwrap();
    thumbs::make_thumbnail(&c.pool, &c.storage, bblob, 32)
        .await
        .unwrap();
    let st: String = sqlx::query_scalar("SELECT thumb_state FROM blobs WHERE id = $1")
        .bind(bblob)
        .fetch_one(&c.pool)
        .await
        .unwrap();
    assert_eq!(st, "failed");
}

#[tokio::test]
async fn recent_view_supports_type_and_date_filters() {
    let c = ctx().await;
    let a = user(&c, "alice").await;
    let png = {
        let mut b = Vec::new();
        let img = image::RgbImage::from_pixel(8, 8, image::Rgb([1, 2, 3]));
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut b), image::ImageFormat::Png)
            .unwrap();
        b
    };
    upload(&c, a, None, "photo.png", &png).await;
    upload(&c, a, None, "notes.txt", b"hello").await;
    let recent = |type_filter: Option<&str>,
                  from: Option<chrono::DateTime<chrono::Utc>>,
                  owner: Option<&str>| ListParams {
        view: View::Recent,
        type_filter: type_filter.map(String::from),
        modified_from: from,
        owner: owner.map(String::from),
        ..params(None, 50, None)
    };
    let all = nodes::list(&c.pool, a, recent(None, None, None))
        .await
        .unwrap();
    assert_eq!(all.items.len(), 2);
    let images = nodes::list(&c.pool, a, recent(Some("image"), None, None))
        .await
        .unwrap();
    assert_eq!(images.items.len(), 1);
    assert_eq!(images.items[0].name, "photo.png");
    let future = chrono::Utc::now() + chrono::Duration::hours(1);
    assert!(
        nodes::list(&c.pool, a, recent(None, Some(future), None))
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let hour_ago = chrono::Utc::now() - chrono::Duration::hours(1);
    assert_eq!(
        nodes::list(&c.pool, a, recent(None, Some(hour_ago), None))
            .await
            .unwrap()
            .items
            .len(),
        2
    );
    assert!(
        nodes::list(&c.pool, a, recent(None, None, Some("others")))
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        nodes::list(&c.pool, a, recent(Some("nope"), None, None))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn recent_view_includes_shared_files_with_source_filter() {
    let c = ctx().await;
    let (a, b) = (user(&c, "alice").await, user(&c, "bob").await);
    let mine = upload(&c, b, None, "mine.txt", b"b").await;
    let theirs = upload(&c, a, None, "theirs.txt", b"a").await;
    shares::add_user(
        &c.pool,
        a,
        theirs.id,
        &users::get(&c.pool, b).await.unwrap().email,
        "viewer",
    )
    .await
    .unwrap();
    let list = |location: Option<&str>, owner: Option<&str>| ListParams {
        view: View::Recent,
        location: location.map(String::from),
        owner: owner.map(String::from),
        ..params(None, 50, None)
    };
    let names = |p: drivich_core::nodes::Page| {
        let mut v: Vec<String> = p.items.into_iter().map(|n| n.name).collect();
        v.sort();
        v
    };
    assert_eq!(
        names(nodes::list(&c.pool, b, list(None, None)).await.unwrap()),
        ["mine.txt", "theirs.txt"]
    );
    assert_eq!(
        names(
            nodes::list(&c.pool, b, list(Some("drive"), None))
                .await
                .unwrap()
        ),
        ["mine.txt"]
    );
    assert_eq!(
        names(
            nodes::list(&c.pool, b, list(Some("shared"), None))
                .await
                .unwrap()
        ),
        ["theirs.txt"]
    );
    assert_eq!(
        names(
            nodes::list(&c.pool, b, list(None, Some("others")))
                .await
                .unwrap()
        ),
        ["theirs.txt"]
    );
    // Alice never sees Bob's private file in her recents.
    assert!(
        !names(nodes::list(&c.pool, a, list(None, None)).await.unwrap())
            .contains(&"mine.txt".to_string())
    );
    let _ = mine;
}

#[tokio::test]
async fn immich_address_is_personal_with_server_default() {
    let c = ctx().await;
    let a = user(&c, "alice").await;
    // Nothing set (use the personal value only: the server default is shared state).
    let u = users::set_immich_url(&c.pool, a, Some(" https://photos.example.test/ "))
        .await
        .unwrap();
    assert_eq!(u.immich_url.as_deref(), Some("https://photos.example.test"));
    assert_eq!(
        drivich_core::settings::effective_immich_url(&c.pool, u.immich_url.as_deref())
            .await
            .unwrap()
            .as_deref(),
        Some("https://photos.example.test")
    );
    assert!(
        users::set_immich_url(&c.pool, a, Some("javascript:alert(1)"))
            .await
            .is_err()
    );
    assert!(
        users::set_immich_url(&c.pool, a, Some("https://u:p@host.test"))
            .await
            .is_err()
    );
    // Clearing removes it.
    let cleared = users::set_immich_url(&c.pool, a, Some("  ")).await.unwrap();
    assert!(cleared.immich_url.is_none());
}

#[tokio::test]
async fn avatar_can_be_set_replaced_and_removed() {
    let c = ctx().await;
    let a = user(&c, "alice").await;
    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        400,
        200,
        image::Rgb([10, 120, 200]),
    ))
    .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
    .unwrap();

    assert!(
        drivich_core::avatars::read(&c.pool, &c.storage, a)
            .await
            .is_err(),
        "no picture yet"
    );
    let v1 = drivich_core::avatars::set(&c.pool, &c.storage, a, png.clone())
        .await
        .unwrap();
    let v2 = drivich_core::avatars::set(&c.pool, &c.storage, a, png)
        .await
        .unwrap();
    assert_eq!(
        (v1, v2),
        (1, 2),
        "version increases on every change (cache busting)"
    );
    assert!(c.storage.avatar_path(a).exists());
    let (bytes, version) = drivich_core::avatars::read(&c.pool, &c.storage, a)
        .await
        .unwrap();
    assert_eq!(version, 2);
    assert!(bytes.starts_with(&[0xFF, 0xD8]), "stored as JPEG");

    // A bad upload changes nothing.
    assert!(
        drivich_core::avatars::set(&c.pool, &c.storage, a, b"not an image".to_vec())
            .await
            .is_err()
    );
    assert_eq!(users::get(&c.pool, a).await.unwrap().avatar_version, 2);

    drivich_core::avatars::remove(&c.pool, &c.storage, a)
        .await
        .unwrap();
    assert_eq!(users::get(&c.pool, a).await.unwrap().avatar_version, 0);
    assert!(!c.storage.avatar_path(a).exists());
    assert!(
        drivich_core::avatars::read(&c.pool, &c.storage, a)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn paused_kinds_are_skipped_and_limits_are_respected() {
    let _guard = JOBS_LOCK.lock().await;
    let c = ctx().await;
    // Use a kind no other test enqueues, so parallel tests cannot interfere.
    let kind = jobs::kind::CLEANUP_SESSIONS;
    sqlx::query("DELETE FROM jobs WHERE kind = $1")
        .bind(kind)
        .execute(&c.pool)
        .await
        .unwrap();
    for _ in 0..3 {
        sqlx::query("INSERT INTO jobs (kind) VALUES ($1)")
            .bind(kind)
            .execute(&c.pool)
            .await
            .unwrap();
    }
    let claim_mine = || async {
        // Claim until we get one of ours, requeueing foreign jobs untouched.
        let mut skipped = Vec::new();
        let mut mine = None;
        while let Some(j) = jobs::claim(&c.pool).await.unwrap() {
            if j.kind == kind {
                mine = Some(j);
                break;
            }
            skipped.push(j.id);
        }
        for id in skipped {
            sqlx::query("UPDATE jobs SET status = 'queued', attempts = attempts - 1, locked_until = NULL WHERE id = $1").bind(id).execute(&c.pool).await.unwrap();
        }
        mine
    };

    let mut controls = jobs::JobControls::default();
    controls.paused.push(kind.to_string());
    jobs::set_controls(&c.pool, &controls).await.unwrap();
    assert!(
        claim_mine().await.is_none(),
        "a paused kind is never started"
    );

    controls.paused.clear();
    controls.limits.insert(kind.to_string(), 1);
    jobs::set_controls(&c.pool, &controls).await.unwrap();
    let first = claim_mine().await.expect("one job may run");
    assert!(
        claim_mine().await.is_none(),
        "limit of 1 reached while one is running"
    );
    jobs::complete(&c.pool, first.id).await.unwrap();
    assert!(claim_mine().await.is_some(), "a slot is free again");

    assert!(
        jobs::set_controls(
            &c.pool,
            &jobs::JobControls {
                paused: vec!["nope".into()],
                ..Default::default()
            }
        )
        .await
        .is_err()
    );
    // Leave the instance as we found it.
    jobs::set_controls(&c.pool, &jobs::JobControls::default())
        .await
        .unwrap();
    sqlx::query("DELETE FROM jobs WHERE kind = $1")
        .bind(kind)
        .execute(&c.pool)
        .await
        .unwrap();
}
