//! Async audit logger implementation.
//!
//! This module provides the main `AuditLogger` which processes audit events
//! asynchronously via a background worker and persists them to PostgreSQL.
//!
//! ## WHY Async Background Worker?
//!
//! Audit logging must never block API request processing. By using an
//! unbounded channel with a background worker:
//! - API handlers return immediately after sending to channel
//! - Database latency doesn't affect user-facing response times
//! - Audit writes can be batched for better throughput (future optimization)
//!
//! We use `unbounded_channel` because:
//! - Audit events are small (< 1KB each)
//! - Backpressure would block API requests (unacceptable)
//! - Memory growth is bounded by request rate, which is already rate-limited

use anyhow::Result;
use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::event::AuditEvent;

/// Async audit logger that writes events to PostgreSQL
#[derive(Clone)]
pub struct AuditLogger {
    /// Channel sender for async event processing
    sender: mpsc::UnboundedSender<AuditEvent>,
}

impl AuditLogger {
    /// Create a new audit logger with background worker
    pub fn new(pool: Pool<Postgres>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Spawn background worker
        tokio::spawn(audit_worker(pool, receiver));

        Self { sender }
    }

    /// Log an audit event (non-blocking)
    pub fn log(&self, event: AuditEvent) {
        if let Err(e) = self.sender.send(event) {
            error!(error = %e, "Failed to send audit event to worker");
        }
    }

    /// Log an audit event and wait for confirmation (blocking)
    pub async fn log_sync(&self, pool: &Pool<Postgres>, event: AuditEvent) -> Result<()> {
        write_audit_event(pool, &event).await
    }
}

/// Background worker that processes audit events asynchronously
async fn audit_worker(pool: Pool<Postgres>, mut receiver: mpsc::UnboundedReceiver<AuditEvent>) {
    debug!("Audit worker started");

    while let Some(event) = receiver.recv().await {
        if let Err(e) = write_audit_event(&pool, &event).await {
            error!(
                error = %e,
                event_id = %event.id,
                event_type = ?event.event_type,
                "Failed to write audit event"
            );
        }
    }

    warn!("Audit worker stopped");
}

/// Write a single audit event to the database
async fn write_audit_event(pool: &Pool<Postgres>, event: &AuditEvent) -> Result<()> {
    let event_type = format!("{:?}", event.event_type).to_lowercase();
    let result = format!("{:?}", event.result).to_lowercase();
    let severity = format!("{:?}", event.severity).to_lowercase();

    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            id, timestamp,
            tenant_id, workspace_id, user_id,
            event_type, event_category, event_action,
            resource_type, resource_id,
            result, severity,
            ip_address, user_agent, request_id, session_id,
            metadata, error_message,
            retention_days, duration_ms
        ) VALUES (
            $1, $2,
            $3, $4, $5,
            $6::audit_event_type, $7, $8,
            $9, $10,
            $11::audit_result, $12::audit_severity,
            $13, $14, $15, $16,
            $17, $18,
            $19, $20
        )
        "#,
    )
    .bind(event.id)
    .bind(event.timestamp)
    .bind(&event.tenant_id)
    .bind(&event.workspace_id)
    .bind(&event.user_id)
    .bind(&event_type)
    .bind(&event.event_category)
    .bind(&event.event_action)
    .bind(&event.resource_type)
    .bind(&event.resource_id)
    .bind(&result)
    .bind(&severity)
    .bind(event.ip_address.map(|ip| ip.to_string()))
    .bind(&event.user_agent)
    .bind(&event.request_id)
    .bind(&event.session_id)
    .bind(&event.metadata)
    .bind(&event.error_message)
    .bind(event.retention_days)
    .bind(event.duration_ms)
    .execute(pool)
    .await?;

    debug!(
        event_id = %event.id,
        tenant_id = %event.tenant_id,
        event_type = ?event.event_type,
        "Wrote audit event"
    );

    Ok(())
}

/// Query audit logs with filters
pub struct AuditQuery {
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub user_id: Option<String>,
    pub event_type: Option<String>,
    pub result: Option<String>,
    pub severity: Option<String>,
    pub limit: i64,
}

impl Default for AuditQuery {
    fn default() -> Self {
        Self {
            tenant_id: None,
            workspace_id: None,
            user_id: None,
            event_type: None,
            result: None,
            severity: None,
            limit: 100,
        }
    }
}

/// Query audit logs from the database
pub async fn query_audit_logs(
    _pool: &Pool<Postgres>,
    query: AuditQuery,
) -> Result<Vec<AuditEvent>> {
    let mut sql = String::from(
        r#"
        SELECT 
            id, timestamp,
            tenant_id, workspace_id, user_id,
            event_type, event_category, event_action,
            resource_type, resource_id,
            result, severity,
            ip_address, user_agent, request_id, session_id,
            metadata, error_message,
            retention_days, duration_ms
        FROM audit_logs
        WHERE 1=1
        "#,
    );

    if query.tenant_id.is_some() {
        sql.push_str(" AND tenant_id = $1");
    }
    if query.workspace_id.is_some() {
        sql.push_str(" AND workspace_id = $2");
    }
    if query.user_id.is_some() {
        sql.push_str(" AND user_id = $3");
    }
    if query.event_type.is_some() {
        sql.push_str(" AND event_type::text = $4");
    }
    if query.result.is_some() {
        sql.push_str(" AND result::text = $5");
    }
    if query.severity.is_some() {
        sql.push_str(" AND severity::text = $6");
    }

    sql.push_str(" ORDER BY timestamp DESC LIMIT $7");

    // TODO: Implement actual query execution with dynamic parameters
    // For now, return empty vec as placeholder
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_logger_creation() {
        // This test requires a real database connection
        // Skip for now - will be tested in integration tests
    }

    #[test]
    fn test_audit_query_default() {
        let query = AuditQuery::default();
        assert_eq!(query.limit, 100);
        assert!(query.tenant_id.is_none());
    }
}
