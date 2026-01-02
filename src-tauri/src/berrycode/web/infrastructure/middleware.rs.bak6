//! Security and rate limiting middleware

use axum::{
    extract::Request,
    middleware::Next,
    response::{Response, Redirect, IntoResponse},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;

use super::database::Database;

/// Security headers middleware
pub async fn security_headers(request: Request, next: Next) -> Response {
    let uri_path = request.uri().path().to_string();
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Add security headers
    headers.insert(
        "X-Content-Type-Options",
        "nosniff".parse().unwrap(),
    );

    // Allow workflow editor to be embedded in iframe (same origin)
    if uri_path.starts_with("/workflow/editor") {
        tracing::info!("✅ Setting X-Frame-Options to SAMEORIGIN for path: {}", uri_path);
        headers.insert(
            "X-Frame-Options",
            "SAMEORIGIN".parse().unwrap(),
        );
    } else {
        tracing::info!("❌ Setting X-Frame-Options to DENY for path: {}", uri_path);
        headers.insert(
            "X-Frame-Options",
            "DENY".parse().unwrap(),
        );
    }

    headers.insert(
        "X-XSS-Protection",
        "1; mode=block".parse().unwrap(),
    );
    headers.insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    headers.insert(
        "Permissions-Policy",
        "geolocation=(), microphone=(), camera=()".parse().unwrap(),
    );

    // Temporarily disable CSP to allow code editor to work
    // TODO: Re-enable with proper configuration
    // headers.insert(
    //     "Content-Security-Policy",
    //     "default-src 'self'; script-src 'self' 'unsafe-eval' 'unsafe-inline' https://cdn.jsdelivr.net https://unpkg.com blob:; style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net; font-src 'self' data: https://cdn.jsdelivr.net; img-src 'self' data: https:; connect-src 'self' ws: wss:; worker-src 'self' blob: data:; child-src 'self' blob: data:;".parse().unwrap(),
    // );

    response
}

/// Request logging middleware
pub async fn request_logging(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = std::time::Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    tracing::info!(
        method = %method,
        uri = %uri,
        status = %status,
        duration_ms = %duration.as_millis(),
        "Request completed"
    );

    response
}

/// Error handling middleware
pub async fn error_handler(request: Request, next: Next) -> Response {
    let response = next.run(request).await;

    // Log errors
    if response.status().is_server_error() {
        tracing::error!(
            status = %response.status(),
            "Server error occurred"
        );
    }

    response
}

/// No-cache middleware for static files (development mode)
pub async fn nocache_static_files(request: Request, next: Next) -> Response {
    let uri_path = request.uri().path().to_string();
    let mut response = next.run(request).await;

    // Add no-cache headers for static files to prevent browser caching during development
    if uri_path.starts_with("/static/") {
        let headers = response.headers_mut();
        headers.insert(
            "Cache-Control",
            "no-cache, no-store, must-revalidate".parse().unwrap(),
        );
        headers.insert(
            "Pragma",
            "no-cache".parse().unwrap(),
        );
        headers.insert(
            "Expires",
            "0".parse().unwrap(),
        );
    }

    response
}

/// Authentication state for middleware
#[derive(Clone)]
pub struct AuthMiddlewareState {
    pub db: Database,
}

/// Authentication middleware - protects routes that require login
pub async fn require_auth(
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Response {
    let uri_path = request.uri().path();
    
    // Allow these paths without authentication
    let public_paths = [
        "/login",
        "/register",
        "/api/auth/login",
        "/api/auth/register",
        "/static/",
        "/favicon.ico",
    ];
    
    // Check if path is public
    for public_path in &public_paths {
        if uri_path.starts_with(public_path) {
            return next.run(request).await;
        }
    }
    
    // Check for session token in cookies
    if let Some(cookie) = jar.get("session_token") {
        let token = cookie.value();
        
        // Get database from extensions (will be set by the router)
        if let Some(db) = request.extensions().get::<Database>() {
            // Verify token
            match db.verify_session_token(token).await {
                Ok(Some(username)) => {
                    // Token is valid, add username to request extensions
                    request.extensions_mut().insert(username.clone());
                    tracing::debug!("Authenticated user: {}", username);
                    return next.run(request).await;
                }
                _ => {
                    tracing::warn!("Invalid or expired session token");
                }
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

/// Simpler auth middleware that only checks token validity
pub async fn check_auth(
    jar: CookieJar,
    request: Request,
    next: Next,
) -> Response {
    // Allow all requests through but add auth info if available
    if let Some(cookie) = jar.get("session_token") {
        tracing::debug!("Session token found: {}", &cookie.value()[..10.min(cookie.value().len())]);
    }
    
    next.run(request).await
}
