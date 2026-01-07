//! Audit logging for EdgeQuake.
//!
//! This crate provides comprehensive audit logging capabilities for tracking
//! security-relevant events, user actions, and system operations in EdgeQuake.
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
