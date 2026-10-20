// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! Drivich core: data model, permissions, storage, search and job queue.
//!
//! The HTTP API (`drivich-server`) and the background worker (`drivich-worker`)
//! are thin binaries on top of this crate. File bytes live on disk
//! (see [`storage`]), everything else lives in PostgreSQL.

pub mod auth;
pub mod avatars;
pub mod backups;
pub mod config;
pub mod db;
pub mod error;
pub mod jobs;
pub mod models;
pub mod nodes;
pub mod perms;
pub mod search;
pub mod settings;
pub mod shares;
pub mod storage;
pub mod thumbs;
pub mod uploads;
pub mod users;

pub use error::{CoreError, Result};

/// Version of the running build (shown in the admin panel).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
