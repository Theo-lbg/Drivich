// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Drivich core: data model, permissions, storage, search and job queue.
//!
//! The HTTP API (`drivich-server`) and the background worker (`drivich-worker`)
//! are thin binaries on top of this crate. File bytes live on disk
//! (see [`storage`]), everything else lives in PostgreSQL.

pub mod archive;
pub mod auth;
pub mod avatars;
pub mod backups;
pub mod comments;
pub mod config;
pub mod db;
pub mod error;
pub mod jobs;
pub mod models;
pub mod nodes;
pub mod office;
pub mod perms;
pub mod search;
pub mod settings;
pub mod shares;
pub mod storage;
pub mod thumbs;
pub mod uploads;
pub mod users;
pub mod versions;

pub use error::{CoreError, Result};

/// Version of the running build (shown in the sidebar and the admin panel).
/// Release version: injected by the release build from the git tag (`DRIVICH_VERSION=1.2.3`),
/// the workspace version otherwise (local and CI builds).
pub const VERSION: &str = match option_env!("DRIVICH_VERSION") {
    Some(v) if !v.is_empty() => v,
    _ => env!("CARGO_PKG_VERSION"),
};

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_a_plain_number_without_a_leading_v() {
        assert!(!super::VERSION.is_empty());
        assert!(!super::VERSION.starts_with('v'));
        assert!(super::VERSION.chars().next().unwrap().is_ascii_digit());
    }
}
