//! Authentication middleware for protecting routes

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;

use super::database::Database;

/// Authentication state for middleware
#[derive(Clone)]
pub struct AuthState {
    pub db: Database,
}

/// Authentication middleware - protects routes that require login
pub async fn require_auth(
    State(state): State<AuthState>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Response {
    let uri_path = request.uri().path();

    // Skip authentication in desktop mode
    if std::env::var("BERRYCODE_DESKTOP_MODE").unwrap_or_default() == "true" {
        tracing::debug!("Desktop mode: skipping authentication for {}", uri_path);
        return next.run(request).await;
    }

    // Allow these exact paths without authentication
    let public_exact_paths = [
        "/",
        "/landing",
        "/login",
        "/register",
        "/favicon.ico",
        "/health",
        "/ready",
        "/editor",
        "/editor/index.html",
        "/dashboard",
        "/app",
    ];

    // Allow these path prefixes without authentication
    let public_prefix_paths = [
        "/api/auth/",
        "/static/",
    ];

    // Check if path is public (exact match)
    for public_path in &public_exact_paths {
        if uri_path == *public_path {
            return next.run(request).await;
        }
    }

    // Check if path is public (prefix match)
    for public_path in &public_prefix_paths {
        if uri_path.starts_with(public_path) {
            return next.run(request).await;
        }
    }

    // Check for session token in cookies
    if let Some(cookie) = jar.get("session_token") {
        let token = cookie.value();

        // Verify token
        match state.db.verify_session_token(token).await {
            Ok(Some(username)) => {
                // Token is valid, add username to request extensions
                request.extensions_mut().insert(username.clone());
                tracing::debug!("Authenticated user: {}", username);
                return next.run(request).await;
            }
            Ok(None) => {
                tracing::warn!("Session token not found in database");
            }
            Err(e) => {
                tracing::error!("Error verifying session token: {}", e);
            }
        }
    }

    // No valid authentication - redirect to login
    tracing::info!("Unauthenticated access attempt to: {}", uri_path);

    // For API routes, return 401 Unauthorized
    if uri_path.starts_with("/api/") {
        return (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
    }

    // For web pages, redirect to login
    Redirect::to("/login").into_response()
}
