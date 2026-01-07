//! Audit event types and builders.
//!
//! This module defines the core audit event structure used to capture
//! security-relevant actions and system events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

/// Type of audit event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum AuditEventType {
    Authentication,
    Authorization,
    DocumentUpload,
    DocumentQuery,
    GraphTraversal,
    TenantAccess,
    WorkspaceAccess,
    RateLimitExceeded,
    SecurityViolation,
    DataExport,
    ConfigChange,
}

/// Result of the audited operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum AuditResult {
    Success,
    Failure,
    Blocked,
    Warning,
}

/// Severity level of the event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
pub enum AuditSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Complete audit event with all context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: Uuid,

    /// When the event occurred
    pub timestamp: DateTime<Utc>,

    /// Tenant context
    pub tenant_id: String,
    pub workspace_id: Option<String>,
    pub user_id: Option<String>,

    /// Event classification
    pub event_type: AuditEventType,
    pub event_category: String,
    pub event_action: String,

    /// Event details
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub result: AuditResult,
    pub severity: AuditSeverity,

    /// Request context
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub session_id: Option<String>,

    /// Additional metadata (flexible JSON)
    pub metadata: serde_json::Value,
    pub error_message: Option<String>,

    /// Compliance
    pub retention_days: i32,

    /// Performance
    pub duration_ms: Option<i32>,
}

impl AuditEvent {
    /// Create a new audit event with minimal required fields
    pub fn new(
        tenant_id: String,
        event_type: AuditEventType,
        event_action: String,
        result: AuditResult,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            tenant_id,
            workspace_id: None,
            user_id: None,
            event_type,
            event_category: "General".to_string(),
            event_action,
            resource_type: None,
            resource_id: None,
            result,
            severity: AuditSeverity::Medium,
            ip_address: None,
            user_agent: None,
            request_id: None,
            session_id: None,
            metadata: serde_json::json!({}),
            error_message: None,
            retention_days: 90,
            duration_ms: None,
        }
    }

    /// Builder pattern for fluent API
    pub fn with_workspace(mut self, workspace_id: String) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_severity(mut self, severity: AuditSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_resource(mut self, resource_type: String, resource_id: String) -> Self {
        self.resource_type = Some(resource_type);
        self.resource_id = Some(resource_id);
        self
    }

    pub fn with_request_context(
        mut self,
        ip: Option<IpAddr>,
        user_agent: Option<String>,
        request_id: Option<String>,
    ) -> Self {
        self.ip_address = ip;
        self.user_agent = user_agent;
        self.request_id = request_id;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_error(mut self, error_message: String) -> Self {
        self.error_message = Some(error_message);
        self.result = AuditResult::Failure;
        self
    }

    pub fn with_duration(mut self, duration_ms: i32) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
}

/// Builder for creating audit events with fluent API
pub struct AuditEventBuilder {
    event: AuditEvent,
}

impl AuditEventBuilder {
    pub fn new(tenant_id: String, event_type: AuditEventType, event_action: String) -> Self {
        Self {
            event: AuditEvent::new(tenant_id, event_type, event_action, AuditResult::Success),
        }
    }

    pub fn workspace(mut self, workspace_id: String) -> Self {
        self.event.workspace_id = Some(workspace_id);
        self
    }

    pub fn user(mut self, user_id: String) -> Self {
        self.event.user_id = Some(user_id);
        self
    }

    pub fn category(mut self, category: String) -> Self {
        self.event.event_category = category;
        self
    }

    pub fn resource(mut self, resource_type: String, resource_id: String) -> Self {
        self.event.resource_type = Some(resource_type);
        self.event.resource_id = Some(resource_id);
        self
    }

    pub fn result(mut self, result: AuditResult) -> Self {
        self.event.result = result;
        self
    }

    pub fn severity(mut self, severity: AuditSeverity) -> Self {
        self.event.severity = severity;
        self
    }

    pub fn ip_address(mut self, ip: IpAddr) -> Self {
        self.event.ip_address = Some(ip);
        self
    }

    pub fn user_agent(mut self, user_agent: String) -> Self {
        self.event.user_agent = Some(user_agent);
        self
    }

    pub fn request_id(mut self, request_id: String) -> Self {
        self.event.request_id = Some(request_id);
        self
    }

    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.event.metadata = metadata;
        self
    }

    pub fn error(mut self, error_message: String) -> Self {
        self.event.error_message = Some(error_message);
        self.event.result = AuditResult::Failure;
        self
    }

    pub fn duration_ms(mut self, duration_ms: i32) -> Self {
        self.event.duration_ms = Some(duration_ms);
        self
    }

    pub fn build(self) -> AuditEvent {
        self.event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(
            "tenant-123".to_string(),
            AuditEventType::DocumentUpload,
            "upload".to_string(),
            AuditResult::Success,
        );

        assert_eq!(event.tenant_id, "tenant-123");
        assert_eq!(event.event_type, AuditEventType::DocumentUpload);
        assert_eq!(event.result, AuditResult::Success);
    }

    #[test]
    fn test_audit_event_builder() {
        let event = AuditEventBuilder::new(
            "tenant-123".to_string(),
            AuditEventType::DocumentQuery,
            "query".to_string(),
        )
        .workspace("workspace-456".to_string())
        .user("user-789".to_string())
        .severity(AuditSeverity::High)
        .metadata(serde_json::json!({
            "query": "test query",
            "results": 10
        }))
        .build();

        assert_eq!(event.tenant_id, "tenant-123");
        assert_eq!(event.workspace_id, Some("workspace-456".to_string()));
        assert_eq!(event.user_id, Some("user-789".to_string()));
        assert_eq!(event.severity, AuditSeverity::High);
    }

    #[test]
    fn test_fluent_api() {
        let event = AuditEvent::new(
            "tenant-123".to_string(),
            AuditEventType::SecurityViolation,
            "blocked".to_string(),
            AuditResult::Blocked,
        )
        .with_severity(AuditSeverity::Critical)
        .with_workspace("workspace-456".to_string())
        .with_error("Invalid token".to_string());

        assert_eq!(event.severity, AuditSeverity::Critical);
        assert_eq!(event.result, AuditResult::Failure);
        assert_eq!(event.error_message, Some("Invalid token".to_string()));
    }
}
