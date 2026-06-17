//! SQLite-backed SessionStore implementation for data-studio-agent-lib.
//!
//! Provides `SqliteSessionStore` which implements the `SessionStore` trait
//! using a local SQLite database with the canonical agent schema.

pub mod db;
pub mod session_store;
