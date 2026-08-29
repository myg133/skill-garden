//! Audit log repository (DEPRECATED)
//!
//! This module now re-exports types from `audit_compat` to provide
//! backward compatibility for handler code that still references
//! `crate::db::repositories::audit::NewAuditLog`.
//!
//! Old `AuditRepository` (audit_logs table) has been replaced by
//! `AuditRepositoryCompat` which writes to the new `audit_log_entries` table.
//!
//! TODO: Remove this file once all call sites are migrated to use
//!       `AuditService::write_entry()` directly.

pub use super::audit_compat::{AuditLog, AuditRepositoryCompat as AuditRepository, NewAuditLog};
