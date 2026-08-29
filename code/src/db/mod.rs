//! Database module
//!
//! PostgreSQL database access layer

pub mod error;
pub mod migrations;
pub mod repositories;
pub mod traits;

pub use error::{DbError, DbResult};
