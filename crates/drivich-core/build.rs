// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

// `sqlx::migrate!` embeds the migrations at compile time; without this, adding a
// new migration file would not trigger a rebuild and it would silently be skipped.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
