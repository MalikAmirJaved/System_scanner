//! SQLite persistence adapters.
//!
//! Only this module may open a database. Domain code depends on narrow traits
//! when a use case needs stored data; nothing in `domain/` imports `rusqlite`.

pub mod migrations;
mod sessions;
mod sqlite;

pub use sqlite::{DATABASE_FILE_NAME, Database, StoreError};
