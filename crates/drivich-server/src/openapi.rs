// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

//! OpenAPI description of the HTTP API, served at `/api/v1/openapi.json` and
//! committed as `docs/openapi.json` (a test fails when they drift apart; run
//! `UPDATE_OPENAPI=1 cargo test -p drivich-server openapi` to refresh it).
//!
//! The handlers live in `routes/`; the request/response shapes below mirror
//! them for documentation, so the frontend types can be generated from the spec.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use drivich_core::backups::Backup;
use drivich_core::jobs::{FailedJob, JobControls, JobCount};
use drivich_core::models::{LinkEntry, Node, ShareEntry, User, UserBrief};
use drivich_core::nodes::{BatchResult, Page};
use drivich_core::search::Suggestion;
use drivich_core::shares::ShareInfo;
use drivich_core::uploads::UploadInfo;
use serde::Serialize;
use std::collections::BTreeMap;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{IntoParams, Modify, OpenApi, ToSchema};
use uuid::Uuid;

struct SessionAuth;

impl Modify for SessionAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(c) = openapi.components.as_mut() {
            c.add_security_scheme(
                "session",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
                    "drivich_session",
                    "HttpOnly session cookie set by /auth/login or /auth/setup. Every non-GET request must also send the header `X-Drivich-CSRF: 1`.",
                ))),
            );
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}
#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}
#[derive(Serialize, ToSchema)]
pub struct OkResponse {
    pub ok: bool,
}
#[derive(Serialize, ToSchema)]
pub struct StatusResponse {
    pub setup_required: bool,
    pub signup_allowed: bool,
    pub version: String,
}
#[derive(Serialize, ToSchema)]
pub struct UserEnvelope {
    pub user: User,
}
#[derive(Serialize, ToSchema)]
pub struct DiskInfo {
    pub free_bytes: u64,
    pub total_bytes: u64,
}
#[derive(Serialize, ToSchema)]
pub struct MeResponse {
    pub user: User,
    pub used_bytes: i64,
    pub quota_bytes: Option<i64>,
    pub disk: Option<DiskInfo>,
    /// Effective Immich address: personal, else the server default.
    pub immich_url: Option<String>,
    pub immich_default_url: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub struct BreadcrumbItem {
    pub id: Uuid,
    pub name: String,
}
#[derive(Serialize, ToSchema)]
pub struct PathResponse {
    pub items: Vec<BreadcrumbItem>,
}
#[derive(Serialize, ToSchema)]
pub struct UserList {
    pub items: Vec<UserBrief>,
}
#[derive(Serialize, ToSchema)]
pub struct SuggestResponse {
    pub items: Vec<Suggestion>,
}
#[derive(Serialize, ToSchema)]
pub struct UploadCreated {
    pub upload: UploadInfo,
}
#[derive(Serialize, ToSchema)]
pub struct AppendResponse {
    pub upload: UploadInfo,
    pub node: Option<Node>,
}
#[derive(Serialize, ToSchema)]
pub struct ShareCreated {
    pub share: ShareEntry,
}
#[derive(Serialize, ToSchema)]
pub struct LinkCreated {
    pub link: LinkEntry,
    /// The raw token, returned only once.
    pub token: String,
    pub path: String,
}
#[derive(Serialize, ToSchema)]
pub struct PublicRoot {
    pub root: Node,
}
#[derive(Serialize, ToSchema)]
pub struct PublicNode {
    pub node: Node,
}
#[derive(Serialize, ToSchema)]
pub struct TrashEmptied {
    pub deleted: u64,
}
#[derive(Serialize, ToSchema)]
pub struct AdminUserRow {
    pub user: User,
    pub used_bytes: i64,
}
#[derive(Serialize, ToSchema)]
pub struct AdminUsers {
    pub items: Vec<AdminUserRow>,
}
#[derive(Serialize, ToSchema)]
pub struct AdminSettings {
    pub allow_signup: bool,
    pub default_quota_bytes: Option<i64>,
    pub immich_url: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub struct DatabaseInfo {
    pub ok: bool,
    pub version: String,
}
#[derive(Serialize, ToSchema)]
pub struct Counts {
    pub users: i64,
    pub files: i64,
    pub folders: i64,
    pub stored_bytes: i64,
}
#[derive(Serialize, ToSchema)]
pub struct Limits {
    pub max_file_bytes: u64,
    pub trash_days: i64,
    pub upload_ttl_hours: i64,
}
#[derive(Serialize, ToSchema)]
pub struct JobsInfo {
    pub stats: Vec<JobCount>,
    pub recent_failures: Vec<FailedJob>,
}
#[derive(Serialize, ToSchema)]
pub struct AdminOverview {
    pub version: String,
    pub database: DatabaseInfo,
    pub counts: Counts,
    pub disk: Option<DiskInfo>,
    pub data_dir: String,
    pub limits: Limits,
    pub jobs: JobsInfo,
}
#[derive(Serialize, ToSchema)]
pub struct JobControlsResponse {
    pub paused: Vec<String>,
    pub limits: BTreeMap<String, i64>,
    pub kinds: Vec<String>,
}
#[derive(Serialize, ToSchema)]
pub struct BackupsResponse {
    pub items: Vec<Backup>,
    pub enabled: bool,
    pub keep: usize,
}
#[derive(Serialize, ToSchema)]
pub struct QueuedResponse {
    pub queued: bool,
}
#[derive(Serialize, ToSchema)]
pub struct RequeuedResponse {
    pub requeued: u64,
}
#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

// ---- request bodies ----
#[derive(Serialize, ToSchema)]
pub struct Registration {
    pub email: String,
    pub name: String,
    pub password: String,
}
#[derive(Serialize, ToSchema)]
pub struct Login {
    pub email: String,
    pub password: String,
}
#[derive(Serialize, ToSchema)]
pub struct ProfilePatch {
    pub name: Option<String>,
    /// `null` clears the personal address.
    pub immich_url: Option<String>,
    /// BCP 47 language tag; `null` follows the browser.
    pub locale: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub struct PasswordChange {
    pub current: String,
    pub new: String,
}
#[derive(Serialize, ToSchema)]
pub struct NewFolder {
    pub parent_id: Option<Uuid>,
    pub name: String,
}
#[derive(Serialize, ToSchema)]
pub struct NodePatch {
    pub name: Option<String>,
    /// `null` moves the item to the root of My Drive.
    pub parent_id: Option<Uuid>,
    pub starred: Option<bool>,
    pub description: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub struct BatchRequest {
    pub ids: Vec<Uuid>,
    /// trash, restore, star, unstar, move or delete
    pub action: String,
    pub parent_id: Option<Uuid>,
}
#[derive(Serialize, ToSchema)]
pub struct NewUpload {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub size: i64,
}
#[derive(Serialize, ToSchema)]
pub struct AddShare {
    pub email: String,
    /// viewer, commenter or editor
    pub role: String,
}
#[derive(Serialize, ToSchema)]
pub struct UpdateShare {
    pub role: String,
}
#[derive(Serialize, ToSchema)]
pub struct NewLink {
    pub expires_at: Option<DateTime<Utc>>,
}
#[derive(Serialize, ToSchema)]
pub struct AdminNewUser {
    pub email: String,
    pub name: String,
    pub password: String,
    pub is_admin: bool,
    pub quota_bytes: Option<i64>,
}
#[derive(Serialize, ToSchema)]
pub struct AdminUserPatch {
    pub is_admin: Option<bool>,
    pub disabled: Option<bool>,
    pub quota_bytes: Option<i64>,
}
#[derive(Serialize, ToSchema)]
pub struct ResetPassword {
    pub password: String,
}
#[derive(Serialize, ToSchema)]
pub struct SettingsBody {
    pub allow_signup: Option<bool>,
    pub default_quota_bytes: Option<i64>,
    pub immich_url: Option<String>,
}

// ---- query strings ----
#[derive(IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
    /// drive (default), shared, recent, starred or trash
    pub view: Option<String>,
    /// Folder id or `root`
    pub parent: Option<String>,
    /// name, modified, created or size
    pub sort: Option<String>,
    /// asc or desc
    pub order: Option<String>,
    /// folder, image, video, audio, pdf, text, document or archive
    #[param(rename = "type")]
    pub type_filter: Option<String>,
    pub modified_from: Option<DateTime<Utc>>,
    /// me or others
    pub owner: Option<String>,
    /// Recent view only: drive or shared
    pub location: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}
#[derive(IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub name: Option<String>,
    pub words: Option<String>,
    #[param(rename = "type")]
    pub type_filter: Option<String>,
    pub owner: Option<String>,
    pub modified_from: Option<DateTime<Utc>>,
    pub modified_to: Option<DateTime<Utc>>,
    /// drive, shared, starred or trash
    pub location: Option<String>,
    pub in_folder: Option<Uuid>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}
#[derive(IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TextQuery {
    pub q: String,
}
#[derive(IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PageQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[utoipa::path(get, path = "/api/v1/health", tag = "system", operation_id = "health", summary = "Liveness and database check", responses((status = 200, description = "Healthy", body = HealthResponse), (status = 503, description = "Database unreachable")))]
fn health() {}

#[utoipa::path(get, path = "/api/v1/auth/status", tag = "auth", operation_id = "auth_status", summary = "Setup state and version", responses((status = 200, description = "State", body = StatusResponse)))]
fn auth_status() {}

#[utoipa::path(post, path = "/api/v1/auth/setup", tag = "auth", operation_id = "auth_setup", summary = "Create the first administrator (works once)", request_body = Registration, responses((status = 200, description = "Signed in", body = UserEnvelope), (status = 409, description = "Setup already completed", body = ErrorBody)))]
fn auth_setup() {}

#[utoipa::path(post, path = "/api/v1/auth/login", tag = "auth", operation_id = "auth_login", summary = "Sign in", request_body = Login, responses((status = 200, description = "Signed in", body = UserEnvelope), (status = 401, description = "Wrong credentials", body = ErrorBody), (status = 429, description = "Too many attempts", body = ErrorBody)))]
fn auth_login() {}

#[utoipa::path(post, path = "/api/v1/auth/signup", tag = "auth", operation_id = "auth_signup", summary = "Create an account (when sign-ups are enabled)", request_body = Registration, responses((status = 200, description = "Signed in", body = UserEnvelope), (status = 403, description = "Sign-ups disabled", body = ErrorBody)))]
fn auth_signup() {}

#[utoipa::path(post, path = "/api/v1/auth/logout", tag = "auth", operation_id = "auth_logout", summary = "Sign out", responses((status = 200, description = "Done", body = OkResponse)), security(("session" = [])))]
fn auth_logout() {}

#[utoipa::path(get, path = "/api/v1/me", tag = "account", operation_id = "me", summary = "Current user, storage usage and Immich address", responses((status = 200, description = "Account", body = MeResponse)), security(("session" = [])))]
fn me() {}

#[utoipa::path(patch, path = "/api/v1/me", tag = "account", operation_id = "update_me", summary = "Update name, Immich address or interface language", request_body = ProfilePatch, responses((status = 200, description = "Updated", body = UserEnvelope), (status = 400, description = "Invalid value", body = ErrorBody)), security(("session" = [])))]
fn update_me() {}

#[utoipa::path(post, path = "/api/v1/me/password", tag = "account", operation_id = "change_password", summary = "Change password (signs out other devices)", request_body = PasswordChange, responses((status = 200, description = "Changed", body = UserEnvelope), (status = 400, description = "Wrong current password", body = ErrorBody)), security(("session" = [])))]
fn change_password() {}

#[utoipa::path(put, path = "/api/v1/me/avatar", tag = "account", operation_id = "upload_avatar", summary = "Set the profile picture (raw image body, 5 MB max)", request_body(content = Vec<u8>, content_type = "image/*", description = "The image (JPEG, PNG, GIF or WebP)"), responses((status = 200, description = "Updated", body = UserEnvelope), (status = 400, description = "Not a supported image", body = ErrorBody), (status = 413, description = "Too large", body = ErrorBody)), security(("session" = [])))]
fn upload_avatar() {}

#[utoipa::path(delete, path = "/api/v1/me/avatar", tag = "account", operation_id = "remove_avatar", summary = "Remove the profile picture", responses((status = 200, description = "Removed", body = UserEnvelope)), security(("session" = [])))]
fn remove_avatar() {}

#[utoipa::path(get, path = "/api/v1/users/{id}/avatar", tag = "account", operation_id = "show_avatar", summary = "A user's profile picture (JPEG)", params(("id" = Uuid, Path, description = "User id")), responses((status = 200, description = "JPEG image"), (status = 404, description = "No picture", body = ErrorBody)), security(("session" = [])))]
fn show_avatar() {}

#[utoipa::path(get, path = "/api/v1/users/lookup", tag = "account", operation_id = "lookup_users", summary = "Find users by name or email prefix (sharing dialog)", params(TextQuery), responses((status = 200, description = "Matches", body = UserList)), security(("session" = [])))]
fn lookup_users() {}

#[utoipa::path(get, path = "/api/v1/nodes", tag = "files", operation_id = "list_nodes", summary = "List files and folders (paginated)", params(ListQuery), responses((status = 200, description = "A page of items", body = Page)), security(("session" = [])))]
fn list_nodes() {}

#[utoipa::path(post, path = "/api/v1/nodes/folder", tag = "files", operation_id = "create_folder", summary = "Create a folder", request_body = NewFolder, responses((status = 200, description = "Created", body = Node), (status = 403, description = "Not allowed", body = ErrorBody)), security(("session" = [])))]
fn create_folder() {}

#[utoipa::path(post, path = "/api/v1/nodes/batch", tag = "files", operation_id = "batch", summary = "Apply an action to several items", request_body = BatchRequest, responses((status = 200, description = "Per-item results", body = BatchResult)), security(("session" = [])))]
fn batch() {}

#[utoipa::path(get, path = "/api/v1/nodes/{id}", tag = "files", operation_id = "get_node", summary = "One item", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "The item", body = Node), (status = 404, description = "Not found", body = ErrorBody)), security(("session" = [])))]
fn get_node() {}

#[utoipa::path(patch, path = "/api/v1/nodes/{id}", tag = "files", operation_id = "update_node", summary = "Rename, move, star or describe", params(("id" = Uuid, Path, description = "Item id")), request_body = NodePatch, responses((status = 200, description = "Updated", body = Node), (status = 403, description = "Not allowed", body = ErrorBody)), security(("session" = [])))]
fn update_node() {}

#[utoipa::path(delete, path = "/api/v1/nodes/{id}", tag = "files", operation_id = "delete_node", summary = "Delete an item that is in the trash, permanently", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "Deleted", body = OkResponse), (status = 400, description = "Not in the trash", body = ErrorBody)), security(("session" = [])))]
fn delete_node() {}

#[utoipa::path(get, path = "/api/v1/nodes/{id}/path", tag = "files", operation_id = "node_path", summary = "Breadcrumb of an item", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "Ancestors, top first", body = PathResponse)), security(("session" = [])))]
fn node_path() {}

#[utoipa::path(post, path = "/api/v1/nodes/{id}/trash", tag = "files", operation_id = "trash_node", summary = "Move to the trash", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "Done", body = OkResponse)), security(("session" = [])))]
fn trash_node() {}

#[utoipa::path(post, path = "/api/v1/nodes/{id}/restore", tag = "files", operation_id = "restore_node", summary = "Restore from the trash", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "Restored", body = Node)), security(("session" = [])))]
fn restore_node() {}

#[utoipa::path(post, path = "/api/v1/trash/empty", tag = "files", operation_id = "empty_trash", summary = "Permanently delete everything in the trash", responses((status = 200, description = "Count", body = TrashEmptied)), security(("session" = [])))]
fn empty_trash() {}

#[utoipa::path(get, path = "/api/v1/nodes/{id}/download", tag = "files", operation_id = "download", summary = "Download the file (HTTP Range and ETag supported)", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "File bytes"), (status = 206, description = "Partial content"), (status = 304, description = "Not modified"), (status = 416, description = "Range not satisfiable")), security(("session" = [])))]
fn download() {}

#[utoipa::path(get, path = "/api/v1/nodes/{id}/content", tag = "files", operation_id = "content", summary = "Inline content for the viewer (unsafe types are served as text)", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "File bytes"), (status = 206, description = "Partial content")), security(("session" = [])))]
fn content() {}

#[utoipa::path(get, path = "/api/v1/nodes/{id}/thumbnail", tag = "files", operation_id = "thumbnail", summary = "Thumbnail (JPEG), 404 until generated", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "JPEG image"), (status = 404, description = "Not available", body = ErrorBody)), security(("session" = [])))]
fn thumbnail() {}

#[utoipa::path(post, path = "/api/v1/nodes/{id}/open", tag = "files", operation_id = "mark_opened", summary = "Record that the file was opened (feeds Recent)", params(("id" = Uuid, Path, description = "Item id")), responses((status = 204, description = "Done")), security(("session" = [])))]
fn mark_opened() {}

#[utoipa::path(get, path = "/api/v1/search", tag = "search", operation_id = "search", summary = "Full search with filters", params(SearchQuery), responses((status = 200, description = "A page of results", body = Page)), security(("session" = [])))]
fn search() {}

#[utoipa::path(get, path = "/api/v1/search/suggest", tag = "search", operation_id = "suggest", summary = "Up to 8 name matches while typing", params(TextQuery), responses((status = 200, description = "Suggestions", body = SuggestResponse)), security(("session" = [])))]
fn suggest() {}

#[utoipa::path(post, path = "/api/v1/uploads", tag = "uploads", operation_id = "create_upload", summary = "Start a resumable upload", request_body = NewUpload, responses((status = 200, description = "Session", body = UploadCreated), (status = 413, description = "Too large", body = ErrorBody), (status = 507, description = "Quota exceeded", body = ErrorBody)), security(("session" = [])))]
fn create_upload() {}

#[utoipa::path(head, path = "/api/v1/uploads/{id}", tag = "uploads", operation_id = "upload_offset", summary = "Bytes received so far (header Upload-Offset)", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "Offset in headers")), security(("session" = [])))]
fn upload_offset() {}

#[utoipa::path(get, path = "/api/v1/uploads/{id}", tag = "uploads", operation_id = "upload_status", summary = "Upload state as JSON", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "State", body = UploadCreated)), security(("session" = [])))]
fn upload_status() {}

#[utoipa::path(patch, path = "/api/v1/uploads/{id}", tag = "uploads", operation_id = "append_upload", summary = "Send a chunk (header Upload-Offset, raw body); the last chunk returns the file", params(("id" = Uuid, Path, description = "Item id")), request_body(content = Vec<u8>, content_type = "application/offset+octet-stream", description = "Raw bytes of this chunk"), responses((status = 200, description = "Progress or finished file", body = AppendResponse), (status = 409, description = "Offset mismatch (current offset in Upload-Offset)", body = ErrorBody)), security(("session" = [])))]
fn append_upload() {}

#[utoipa::path(delete, path = "/api/v1/uploads/{id}", tag = "uploads", operation_id = "cancel_upload", summary = "Cancel an upload", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "Cancelled", body = OkResponse)), security(("session" = [])))]
fn cancel_upload() {}

#[utoipa::path(get, path = "/api/v1/nodes/{id}/shares", tag = "sharing", operation_id = "list_shares", summary = "Who has access, and active links", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "Sharing state", body = ShareInfo)), security(("session" = [])))]
fn list_shares() {}

#[utoipa::path(post, path = "/api/v1/nodes/{id}/shares", tag = "sharing", operation_id = "add_share", summary = "Share with a user", params(("id" = Uuid, Path, description = "Item id")), request_body = AddShare, responses((status = 200, description = "Shared", body = ShareCreated)), security(("session" = [])))]
fn add_share() {}

#[utoipa::path(patch, path = "/api/v1/shares/{id}", tag = "sharing", operation_id = "update_share", summary = "Change a share's role", params(("id" = Uuid, Path, description = "Item id")), request_body = UpdateShare, responses((status = 200, description = "Updated", body = OkResponse)), security(("session" = [])))]
fn update_share() {}

#[utoipa::path(delete, path = "/api/v1/shares/{id}", tag = "sharing", operation_id = "remove_share", summary = "Remove a share", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "Removed", body = OkResponse)), security(("session" = [])))]
fn remove_share() {}

#[utoipa::path(post, path = "/api/v1/nodes/{id}/links", tag = "sharing", operation_id = "create_link", summary = "Create a public read-only link (token returned once)", params(("id" = Uuid, Path, description = "Item id")), request_body = NewLink, responses((status = 200, description = "Created", body = LinkCreated)), security(("session" = [])))]
fn create_link() {}

#[utoipa::path(delete, path = "/api/v1/links/{id}", tag = "sharing", operation_id = "revoke_link", summary = "Revoke a public link", params(("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "Revoked", body = OkResponse)), security(("session" = [])))]
fn revoke_link() {}

#[utoipa::path(get, path = "/api/v1/public/{token}", tag = "public", operation_id = "public_root", summary = "The item a public link points to", params(("token" = String, Path, description = "Link token")), responses((status = 200, description = "Root item", body = PublicRoot), (status = 404, description = "Unknown, revoked or expired", body = ErrorBody)))]
fn public_root() {}

#[utoipa::path(get, path = "/api/v1/public/{token}/nodes/{id}", tag = "public", operation_id = "public_node", summary = "An item inside a public link", params(("token" = String, Path, description = "Link token"), ("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "The item", body = PublicNode)))]
fn public_node() {}

#[utoipa::path(get, path = "/api/v1/public/{token}/nodes/{id}/children", tag = "public", operation_id = "public_children", summary = "Children of a shared folder", params(("token" = String, Path, description = "Link token"), ("id" = Uuid, Path, description = "Item id"), PageQuery), responses((status = 200, description = "A page", body = Page)))]
fn public_children() {}

#[utoipa::path(get, path = "/api/v1/public/{token}/nodes/{id}/download", tag = "public", operation_id = "public_download", summary = "Download through a public link", params(("token" = String, Path, description = "Link token"), ("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "File bytes")))]
fn public_download() {}

#[utoipa::path(get, path = "/api/v1/public/{token}/nodes/{id}/content", tag = "public", operation_id = "public_content", summary = "Inline content through a public link", params(("token" = String, Path, description = "Link token"), ("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "File bytes")))]
fn public_content() {}

#[utoipa::path(get, path = "/api/v1/public/{token}/nodes/{id}/thumbnail", tag = "public", operation_id = "public_thumbnail", summary = "Thumbnail through a public link", params(("token" = String, Path, description = "Link token"), ("id" = Uuid, Path, description = "Item id")), responses((status = 200, description = "JPEG image")))]
fn public_thumbnail() {}

#[utoipa::path(get, path = "/api/v1/admin/overview", tag = "admin", operation_id = "admin_overview", summary = "Version, database, disk, counts and jobs", responses((status = 200, description = "Overview", body = AdminOverview), (status = 403, description = "Admins only", body = ErrorBody)), security(("session" = [])))]
fn admin_overview() {}

#[utoipa::path(get, path = "/api/v1/admin/users", tag = "admin", operation_id = "admin_users", summary = "All users with their usage", responses((status = 200, description = "Users", body = AdminUsers)), security(("session" = [])))]
fn admin_users() {}

#[utoipa::path(post, path = "/api/v1/admin/users", tag = "admin", operation_id = "admin_create_user", summary = "Create a user", request_body = AdminNewUser, responses((status = 200, description = "Created", body = UserEnvelope)), security(("session" = [])))]
fn admin_create_user() {}

#[utoipa::path(patch, path = "/api/v1/admin/users/{id}", tag = "admin", operation_id = "admin_update_user", summary = "Change role, status or quota", params(("id" = Uuid, Path, description = "Item id")), request_body = AdminUserPatch, responses((status = 200, description = "Updated", body = UserEnvelope)), security(("session" = [])))]
fn admin_update_user() {}

#[utoipa::path(post, path = "/api/v1/admin/users/{id}/password", tag = "admin", operation_id = "admin_reset_password", summary = "Reset a user's password (closes their sessions)", params(("id" = Uuid, Path, description = "Item id")), request_body = ResetPassword, responses((status = 200, description = "Done", body = OkResponse)), security(("session" = [])))]
fn admin_reset_password() {}

#[utoipa::path(get, path = "/api/v1/admin/settings", tag = "admin", operation_id = "admin_get_settings", summary = "Instance settings", responses((status = 200, description = "Settings", body = AdminSettings)), security(("session" = [])))]
fn admin_get_settings() {}

#[utoipa::path(put, path = "/api/v1/admin/settings", tag = "admin", operation_id = "admin_put_settings", summary = "Change instance settings", request_body = SettingsBody, responses((status = 200, description = "Settings", body = AdminSettings)), security(("session" = [])))]
fn admin_put_settings() {}

#[utoipa::path(get, path = "/api/v1/admin/jobs/config", tag = "admin", operation_id = "admin_get_job_controls", summary = "Paused kinds and concurrency limits", responses((status = 200, description = "Controls", body = JobControlsResponse)), security(("session" = [])))]
fn admin_get_job_controls() {}

#[utoipa::path(put, path = "/api/v1/admin/jobs/config", tag = "admin", operation_id = "admin_put_job_controls", summary = "Pause kinds and set concurrency limits", request_body = JobControls, responses((status = 200, description = "Controls", body = JobControlsResponse)), security(("session" = [])))]
fn admin_put_job_controls() {}

#[utoipa::path(post, path = "/api/v1/admin/jobs/retry", tag = "admin", operation_id = "admin_retry_jobs", summary = "Requeue every failed job", responses((status = 200, description = "Count", body = RequeuedResponse)), security(("session" = [])))]
fn admin_retry_jobs() {}

#[utoipa::path(get, path = "/api/v1/admin/backups", tag = "admin", operation_id = "admin_list_backups", summary = "Database dumps on disk", responses((status = 200, description = "Backups", body = BackupsResponse)), security(("session" = [])))]
fn admin_list_backups() {}

#[utoipa::path(post, path = "/api/v1/admin/backups", tag = "admin", operation_id = "admin_create_backup", summary = "Queue a database dump now", responses((status = 200, description = "Queued", body = QueuedResponse)), security(("session" = [])))]
fn admin_create_backup() {}

#[utoipa::path(get, path = "/api/v1/admin/backups/{name}/download", tag = "admin", operation_id = "admin_download_backup", summary = "Download a dump (contains password hashes)", params(("name" = String, Path, description = "Backup file name")), responses((status = 200, description = "pg_dump custom-format file"), (status = 404, description = "Unknown name", body = ErrorBody)), security(("session" = [])))]
fn admin_download_backup() {}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Drivich API",
        version = env!("CARGO_PKG_VERSION"),
        description = "Self-hosted file storage. Authentication uses a session cookie; every non-GET request must carry the header `X-Drivich-CSRF: 1`. Errors have the shape `{ \"error\": { \"code\", \"message\" } }`.",
        license(name = "AGPL-3.0-or-later", identifier = "AGPL-3.0-or-later")
    ),
    paths(
        health,
        auth_status,
        auth_setup,
        auth_login,
        auth_signup,
        auth_logout,
        me,
        update_me,
        change_password,
        upload_avatar,
        remove_avatar,
        show_avatar,
        lookup_users,
        list_nodes,
        create_folder,
        batch,
        get_node,
        update_node,
        delete_node,
        node_path,
        trash_node,
        restore_node,
        empty_trash,
        download,
        content,
        thumbnail,
        mark_opened,
        search,
        suggest,
        create_upload,
        upload_offset,
        upload_status,
        append_upload,
        cancel_upload,
        list_shares,
        add_share,
        update_share,
        remove_share,
        create_link,
        revoke_link,
        public_root,
        public_node,
        public_children,
        public_download,
        public_content,
        public_thumbnail,
        admin_overview,
        admin_users,
        admin_create_user,
        admin_update_user,
        admin_reset_password,
        admin_get_settings,
        admin_put_settings,
        admin_get_job_controls,
        admin_put_job_controls,
        admin_retry_jobs,
        admin_list_backups,
        admin_create_backup,
        admin_download_backup
    ),
    components(schemas(
        ErrorDetail,
        ErrorBody,
        OkResponse,
        StatusResponse,
        UserEnvelope,
        DiskInfo,
        MeResponse,
        BreadcrumbItem,
        PathResponse,
        UserList,
        SuggestResponse,
        UploadCreated,
        AppendResponse,
        ShareCreated,
        LinkCreated,
        PublicRoot,
        PublicNode,
        TrashEmptied,
        AdminUserRow,
        AdminUsers,
        AdminSettings,
        DatabaseInfo,
        Counts,
        Limits,
        JobsInfo,
        AdminOverview,
        JobControlsResponse,
        BackupsResponse,
        QueuedResponse,
        RequeuedResponse,
        HealthResponse,
        Registration,
        Login,
        ProfilePatch,
        PasswordChange,
        NewFolder,
        NodePatch,
        BatchRequest,
        NewUpload,
        AddShare,
        UpdateShare,
        NewLink,
        AdminNewUser,
        AdminUserPatch,
        ResetPassword,
        SettingsBody,
        User,
        UserBrief,
        Node,
        ShareEntry,
        LinkEntry,
        ShareInfo,
        Page,
        BatchResult,
        Suggestion,
        UploadInfo,
        Backup,
        JobControls,
        JobCount,
        FailedJob
    )),
    modifiers(&SessionAuth),
    tags(
        (name = "auth"), (name = "account"), (name = "files"), (name = "search"), (name = "uploads"),
        (name = "sharing"), (name = "public"), (name = "admin"), (name = "system")
    )
)]
pub struct ApiDoc;

pub fn spec() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pretty() -> String {
        let mut s = spec().to_pretty_json().expect("spec serialises");
        s.push('\n');
        s
    }

    /// `docs/openapi.json` must match what the code generates.
    /// Refresh with: UPDATE_OPENAPI=1 cargo test -p drivich-server openapi
    #[test]
    fn committed_spec_is_up_to_date() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/openapi.json");
        if std::env::var("UPDATE_OPENAPI").is_ok() {
            std::fs::write(path, pretty()).unwrap();
        }
        let committed = std::fs::read_to_string(path)
            .expect("docs/openapi.json exists (run with UPDATE_OPENAPI=1)");
        assert_eq!(
            committed,
            pretty(),
            "docs/openapi.json is stale: run `UPDATE_OPENAPI=1 cargo test -p drivich-server openapi`"
        );
    }

    #[test]
    fn every_documented_path_lives_under_the_api_prefix() {
        let v = serde_json::to_value(spec()).unwrap();
        let paths = v["paths"].as_object().unwrap();
        assert!(paths.len() > 40, "{} paths", paths.len());
        assert!(paths.keys().all(|p| p.starts_with("/api/v1/")));
    }
}
