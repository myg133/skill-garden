//! Database module
//!
//! PostgreSQL database access layer

pub mod migrations;
pub mod repositories;
pub mod error;
pub mod traits;

pub use error::{DbError, DbResult};
