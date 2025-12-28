//! Common utilities and components for BerryEditor
//!
//! This module contains reusable functionality to ensure zero code duplication.

// Platform abstraction layer
pub mod platform;
pub mod storage;
pub mod events;
pub mod tauri_bridge;

// Existing modules
pub mod async_bridge;
pub mod context_menu;
pub mod dialogs;
pub mod event_handler;
pub mod keyboard;
pub mod splitter;
pub mod ui_components;
pub mod validation;
