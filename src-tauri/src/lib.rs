// Library exports for testing
pub mod app_database;  // Application database (sessions, settings, workflow logs)
pub mod database;      // External database connections (PostgreSQL, MySQL, etc.)
pub mod terminal;
pub mod workflow;
pub mod persistent_terminal;

// BerryCode CLI modules (integrated from parent)
pub mod berrycode;
