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
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"storage_mode\":\"memory\""));
        assert!(json.contains("\"llm_provider_name\":\"openai\""));
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
