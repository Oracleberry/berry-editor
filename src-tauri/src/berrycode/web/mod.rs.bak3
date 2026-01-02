//! Web version of BerryCode
//!
//! Provides a web interface for BerryCode with:
//! - Real-time chat with AI
//! - File browser and editor
//! - Live code editing with preview
//! - Project sharing

#[cfg(feature = "web")]
pub mod server;
#[cfg(feature = "web")]
pub mod handlers;

// API modules
#[cfg(feature = "web")]
pub mod api;

// Infrastructure modules
#[cfg(feature = "web")]
pub mod infrastructure;

// Re-export commonly used items
#[cfg(feature = "web")]
pub use api::*;
#[cfg(feature = "web")]
pub use infrastructure::*;

#[cfg(all(test, feature = "web"))]
mod tests;

#[cfg(feature = "web")]
pub use server::run_server;
