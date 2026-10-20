// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

use crate::error::ApiResult;
use crate::extract::{AuthUser, COOKIE, client_ip};
use crate::state::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use drivich_core::models::User;
use drivich_core::uploads::effective_quota;
use drivich_core::{CoreError, auth, settings, users};
use serde::Deserialize;
use serde_json::{Value, json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/status", get(status))
        .route("/auth/setup", post(setup))
        .route("/auth/login", post(login))
        .route("/auth/signup", post(signup))
        .route("/auth/logout", post(logout))
        .route("/me", get(me).patch(update_me))
        .route("/me/password", post(change_password))
}

fn session_cookie(st: &AppState, token: String) -> Cookie<'static> {
    Cookie::build((COOKIE, token))
        .http_only(true)
        .secure(st.cfg.cookie_secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time_days(st.cfg.session_days))
        .build()
}

fn time_days(days: i64) -> time::Duration {
    time::Duration::days(days)
}

async fn status(State(st): State<AppState>) -> ApiResult<Json<Value>> {
    let setup_required = users::count(&st.pool).await? == 0;
    let signup = settings::get_bool(&st.pool, settings::ALLOW_SIGNUP, false).await?;
    Ok(Json(json!({
        "setup_required": setup_required,
        "signup_allowed": signup && !setup_required,
        "version": drivich_core::VERSION,
    })))
}

#[derive(Deserialize)]
struct Registration {
    email: String,
    name: String,
    password: String,
}

async fn start_session(
    st: &AppState,
    jar: CookieJar,
    headers: &HeaderMap,
    user: &User,
) -> ApiResult<(CookieJar, Json<Value>)> {
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok());
    let token = auth::create_session(&st.pool, user.id, st.cfg.session_days, ua).await?;
    Ok((
        jar.add(session_cookie(st, token)),
        Json(json!({ "user": user })),
    ))
}

/// First-run setup: creates the administrator. Works exactly once.
async fn setup(
    State(st): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(b): Json<Registration>,
) -> ApiResult<(CookieJar, Json<Value>)> {
    let user = users::create_first_admin(&st.pool, &b.email, &b.name, &b.password).await?;
    start_session(&st, jar, &headers, &user).await
}

async fn signup(
    State(st): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(b): Json<Registration>,
) -> ApiResult<(CookieJar, Json<Value>)> {
    if !settings::get_bool(&st.pool, settings::ALLOW_SIGNUP, false).await? {
        return Err(CoreError::Forbidden.into());
    }
    let user = users::create(&st.pool, &b.email, &b.name, &b.password, false, None).await?;
    start_session(&st, jar, &headers, &user).await
}

#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
}

async fn login(
    State(st): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(b): Json<Login>,
) -> ApiResult<(CookieJar, Json<Value>)> {
    let key = format!(
        "{}|{}",
        b.email.trim().to_lowercase(),
        client_ip(&headers, st.cfg.trust_proxy)
    );
    if st.limiter.blocked(&key) {
        return Err(CoreError::TooManyRequests.into());
    }
    match users::authenticate(&st.pool, &b.email, &b.password).await {
        Ok(user) => {
            st.limiter.reset(&key);
            start_session(&st, jar, &headers, &user).await
        }
        Err(e) => {
            if matches!(e, CoreError::Unauthorized) {
                st.limiter.record_failure(&key);
            }
            Err(e.into())
        }
    }
}

async fn logout(State(st): State<AppState>, jar: CookieJar) -> ApiResult<(CookieJar, Json<Value>)> {
    if let Some(c) = jar.get(COOKIE) {
        auth::delete_session(&st.pool, c.value()).await?;
    }
    let removal = Cookie::build((COOKIE, "")).path("/").build();
    Ok((jar.remove(removal), Json(json!({ "ok": true }))))
}

async fn me(State(st): State<AppState>, AuthUser(u): AuthUser) -> ApiResult<Json<Value>> {
    let used = users::used_bytes(&st.pool, u.id).await?;
    let quota = effective_quota(&st.pool, u.id).await?;
    let disk = st
        .storage
        .disk_space()
        .map(|(free, total)| json!({ "free_bytes": free, "total_bytes": total }));
    // The address shown in the apps launcher: personal if set, else the server default.
    let immich = settings::effective_immich_url(&st.pool, u.immich_url.as_deref()).await?;
    let immich_default = settings::effective_immich_url(&st.pool, None).await?;
    Ok(Json(json!({
        "user": u,
        "used_bytes": used,
        "quota_bytes": quota,
        "disk": disk,
        "office": st.cfg.office_jwt_secret.is_some(),
        "immich_url": immich,
        "immich_default_url": immich_default,
    })))
}

/// Distinguishes "field absent" from "explicit null" (which clears the value).
fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(d)?))
}

#[derive(Deserialize)]
struct ProfileUpdate {
    name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    immich_url: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    locale: Option<Option<String>>,
}

async fn update_me(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    Json(b): Json<ProfileUpdate>,
) -> ApiResult<Json<Value>> {
    let mut user = u;
    if let Some(name) = &b.name {
        user = users::update_profile(&st.pool, user.id, name).await?;
    }
    if let Some(url) = &b.immich_url {
        user = users::set_immich_url(&st.pool, user.id, url.as_deref()).await?;
    }
    if let Some(locale) = &b.locale {
        user = users::set_locale(&st.pool, user.id, locale.as_deref()).await?;
    }
    Ok(Json(json!({ "user": user })))
}

#[derive(Deserialize)]
struct PasswordChange {
    current: String,
    new: String,
}

async fn change_password(
    State(st): State<AppState>,
    AuthUser(u): AuthUser,
    jar: CookieJar,
    headers: HeaderMap,
    Json(b): Json<PasswordChange>,
) -> ApiResult<(CookieJar, Json<Value>)> {
    if !auth::verify_password(&b.current, &u.password_hash) {
        return Err(CoreError::bad("current password is incorrect").into());
    }
    users::update_password(&st.pool, u.id, &b.new).await?;
    // Sign out everywhere else, keep this browser signed in with a fresh session.
    auth::delete_user_sessions(&st.pool, u.id).await?;
    start_session(&st, jar, &headers, &u).await
}
