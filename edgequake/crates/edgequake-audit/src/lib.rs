//! Audit logging for EdgeQuake.
//!
//! This crate provides comprehensive audit logging capabilities for tracking
//! security-relevant events, user actions, and system operations in EdgeQuake.
//!
//! ## Implements
//!
//! - [`FEAT0701`]: Comprehensive audit event logging
//! - [`FEAT0702`]: PostgreSQL-backed audit storage
//! - [`FEAT0703`]: Async background audit processing
//! - [`FEAT0704`]: Audit log query interface
//!
//! ## Enforces
//!
//! - [`BR0701`]: Security events logged with timestamps
//! - [`BR0702`]: Audit logs immutable after creation
//! - [`BR0703`]: Failed operations logged with error context
//!
//! ## Use Cases
//!
//! - [`UC0701`]: Admin reviews authentication audit trail
//! - [`UC0702`]: System logs document ingestion events
//! - [`UC0703`]: Security team queries access patterns
//!
//! # Features
//!
//! - Async audit event processing with background workers
//! - PostgreSQL-backed persistent storage
//! - Structured event types with severity levels
//! - Query interface for audit log analysis
//!
//! # Example
//!
//! ```ignore
//! use edgequake_audit::{AuditLogger, AuditEvent, AuditEventType};
//!
//! let logger = AuditLogger::new(pool);
//! logger.log(AuditEvent::new(AuditEventType::Authentication, "user.login"));
//! ```

pub mod event;
pub mod logger;

pub use event::{AuditEvent, AuditEventBuilder, AuditEventType, AuditResult, AuditSeverity};
pub use logger::{query_audit_logs, AuditLogger, AuditQuery};
