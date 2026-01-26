//! DTOs for health check handlers.
//!
//! This module contains the request and response types for health operations.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Response Types
// ============================================================================

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Service status.
    pub status: String,

    /// Service version.
    pub version: String,

    /// Storage mode: "memory" or "postgresql".
    pub storage_mode: String,

    /// Workspace ID.
    pub workspace_id: String,

    /// Component health.
    pub components: ComponentHealth,

    /// LLM provider name (e.g., "openai", "mock", "ollama").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider_name: Option<String>,

    /// Database schema health (PostgreSQL only).
    ///
    /// WHY: Mission requirement - "verify the integrity of schema against
    /// the version of edgequake running."
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaHealth>,
}

/// Database schema health information.
///
/// WHY: OODA-14 - Provides visibility into database migration state.
/// Operators can verify schema is up-to-date before deployment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SchemaHealth {
    /// Latest migration version applied (e.g., 15 for 015_add_fulltext_search.sql).
    pub latest_version: Option<i64>,

    /// Number of successful migrations applied.
    pub migrations_applied: usize,

    /// When the last migration was applied (ISO 8601 timestamp).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_applied_at: Option<String>,
}

/// Component health status.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComponentHealth {
    /// KV storage status.
    pub kv_storage: bool,

    /// Vector storage status.
    pub vector_storage: bool,

    /// Graph storage status.
    pub graph_storage: bool,

    /// LLM provider status.
    pub llm_provider: bool,
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
            storage_mode: "memory".to_string(),
            workspace_id: "default".to_string(),
            components: ComponentHealth {
                kv_storage: true,
                vector_storage: true,
                graph_storage: true,
                llm_provider: true,
            },
            llm_provider_name: Some("openai".to_string()),
            schema: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"storage_mode\":\"memory\""));
        assert!(json.contains("\"llm_provider_name\":\"openai\""));
        // schema should be skipped when None
        assert!(!json.contains("\"schema\""));
    }

    #[test]
    fn test_health_response_with_schema() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
            storage_mode: "postgresql".to_string(),
            workspace_id: "ws-123".to_string(),
            components: ComponentHealth {
                kv_storage: true,
                vector_storage: true,
                graph_storage: true,
                llm_provider: true,
            },
            llm_provider_name: Some("ollama".to_string()),
            schema: Some(SchemaHealth {
                latest_version: Some(15),
                migrations_applied: 15,
                last_applied_at: Some("2025-01-26T10:00:00Z".to_string()),
            }),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"schema\""));
        assert!(json.contains("\"latest_version\":15"));
        assert!(json.contains("\"migrations_applied\":15"));
    }

    #[test]
    fn test_schema_health_serialization() {
        let schema = SchemaHealth {
            latest_version: Some(14),
            migrations_applied: 14,
            last_applied_at: None,
        };
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("\"latest_version\":14"));
        assert!(json.contains("\"migrations_applied\":14"));
        // last_applied_at should be skipped when None
        assert!(!json.contains("last_applied_at"));
    }

    #[test]
    fn test_component_health_all_false() {
        let components = ComponentHealth {
            kv_storage: false,
            vector_storage: false,
            graph_storage: false,
            llm_provider: false,
        };
        let json = serde_json::to_string(&components).unwrap();
        assert!(json.contains("\"kv_storage\":false"));
        assert!(json.contains("\"vector_storage\":false"));
        assert!(json.contains("\"graph_storage\":false"));
        assert!(json.contains("\"llm_provider\":false"));
    }

    #[test]
    fn test_health_response_skip_none_llm() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
            storage_mode: "postgresql".to_string(),
            workspace_id: "ws-123".to_string(),
            components: ComponentHealth {
                kv_storage: true,
                vector_storage: true,
                graph_storage: true,
                llm_provider: false,
            },
            llm_provider_name: None,
            schema: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        // llm_provider_name should be skipped when None
        assert!(!json.contains("llm_provider_name"));
        assert!(json.contains("\"storage_mode\":\"postgresql\""));
    }

    #[test]
    fn test_component_health_all_true() {
        let components = ComponentHealth {
            kv_storage: true,
            vector_storage: true,
            graph_storage: true,
            llm_provider: true,
        };
        let json = serde_json::to_string(&components).unwrap();
        assert!(json.contains("\"kv_storage\":true"));
        assert!(json.contains("\"graph_storage\":true"));
    }
}
