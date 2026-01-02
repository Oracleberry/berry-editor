//! Authentication with database backend

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::{Deserialize, Serialize};

use super::database::Database;

/// Authentication state with database
#[derive(Clone)]
pub struct AuthDbState {
    pub db: Database,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
}

/// Login handler with database
pub async fn login_db(
    State(state): State<AuthDbState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<(CookieJar, Json<LoginResponse>), StatusCode> {
    // Verify credentials
    let valid = state
        .db
        .verify_user(&payload.username, &payload.password)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Create session token
    let token = state
        .db
        .create_session_token(payload.username.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let cookie = Cookie::build(("session_token", token.clone()))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::days(7));

    let jar = jar.add(cookie);

    Ok((
        jar,
        Json(LoginResponse {
            token,
            username: payload.username,
        }),
    ))
}

/// Logout handler with database
pub async fn logout_db(
    State(state): State<AuthDbState>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), StatusCode> {
    if let Some(cookie) = jar.get("session_token") {
        let _ = state.db.destroy_session_token(cookie.value()).await;
    }

    let jar = jar.remove(Cookie::from("session_token"));

    Ok((jar, StatusCode::OK))
}

/// Verify authentication middleware with database
pub async fn verify_auth_db(
    State(state): State<AuthDbState>,
    jar: CookieJar,
) -> Result<Json<String>, Response> {
    if let Some(cookie) = jar.get("session_token") {
        if let Ok(Some(username)) = state.db.verify_session_token(cookie.value()).await {
            return Ok(Json(username));
        }
    }

    Err((
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Bearer")],
    )
        .into_response())
}

/// Register new user
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

/// Register handler
pub async fn register(
    State(state): State<AuthDbState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<StatusCode, StatusCode> {
    // Validate username and password
    if payload.username.len() < 3 || payload.password.len() < 6 {
        return Err(StatusCode::BAD_REQUEST);
    }

    state
        .db
        .create_user(&payload.username, &payload.password, "user")
        .await
        .map_err(|_| StatusCode::CONFLICT)?;

    Ok(StatusCode::CREATED)
}
