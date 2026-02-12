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

    /// Service version (semver from Cargo.toml).
    pub version: String,

    /// Build metadata (git hash, timestamp, build number).
    ///
    /// WHY: Operators need to identify exactly which build is running.
    /// The semver alone is insufficient for debugging production issues.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_info: Option<BuildInfo>,

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

    /// Provider configuration details (LLM and embedding).
    ///
    /// WHY: OODA-11 - Mission requirement: "Ensure health API make it easy to know
    /// all parts of the applied configuration (llm provider, embedding provider,
    /// models used)".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<ProvidersHealth>,

    /// Whether PDF storage is enabled.
    ///
    /// WHY: OODA-11 - Operators need to verify PDF processing is available.
    /// When false, document uploads may fail silently.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_storage_enabled: Option<bool>,
}

/// Build metadata embedded at compile time.
///
/// WHY: Every build must be traceable to a specific git commit and time.
/// This enables fast debugging when issues arise in production.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BuildInfo {
    /// Git short hash (e.g., "a1b2c3d").
    pub git_hash: String,

    /// Git branch name (e.g., "main", "fix/improvement-fix").
    pub git_branch: String,

    /// Build timestamp in ISO 8601 UTC (e.g., "2026-02-12T10:30:00Z").
    pub build_timestamp: String,

    /// Build number in YYYYMMDD.HHMMSS format for monotonic ordering.
    pub build_number: String,
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
// Provider Health Types (OODA-11)
// ============================================================================

/// LLM provider health information.
///
/// WHY: OODA-11 - Operators need to verify which LLM model is active.
/// Model choice affects entity extraction quality and API costs.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LlmProviderHealth {
    /// Provider name (e.g., "openai", "ollama", "mock").
    pub name: String,

    /// Model being used (e.g., "gpt-4.1-nano", "gemma3:latest").
    pub model: String,
}

/// Embedding provider health information.
///
/// WHY: OODA-11 - Embedding dimension must match vector storage schema.
/// Dimension mismatch causes silent failures during semantic search.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmbeddingProviderHealth {
    /// Provider name (e.g., "openai", "ollama").
    pub name: String,

    /// Embedding model (e.g., "text-embedding-3-small", "nomic-embed-text").
    pub model: String,

    /// Embedding vector dimension (e.g., 768, 1536, 3072).
    /// Must match PostgreSQL vector column dimension.
    pub dimension: usize,
}

/// Combined provider health for LLM and embedding.
///
/// WHY: OODA-11 - Mission requirement: "know all parts of the applied
/// configuration (llm provider, embedding provider, models used)".
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProvidersHealth {
    /// LLM provider details.
    pub llm: LlmProviderHealth,

    /// Embedding provider details.
    pub embedding: EmbeddingProviderHealth,
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
            build_info: None,
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
            providers: None,
            pdf_storage_enabled: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"storage_mode\":\"memory\""));
        assert!(json.contains("\"llm_provider_name\":\"openai\""));
        // schema should be skipped when None
        assert!(!json.contains("\"schema\""));
        // providers should be skipped when None
        assert!(!json.contains("\"providers\""));
        // pdf_storage_enabled should be skipped when None
        assert!(!json.contains("\"pdf_storage_enabled\""));
    }

    #[test]
    fn test_health_response_with_schema() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
            build_info: None,
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
            providers: None,
            pdf_storage_enabled: None,
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
            build_info: None,
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
            providers: None,
            pdf_storage_enabled: None,
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

    /// OODA-11: Test providers health serialization.
    #[test]
    fn test_providers_health_serialization() {
        let providers = ProvidersHealth {
            llm: LlmProviderHealth {
                name: "ollama".to_string(),
                model: "gemma3:latest".to_string(),
            },
            embedding: EmbeddingProviderHealth {
                name: "ollama".to_string(),
                model: "nomic-embed-text".to_string(),
                dimension: 768,
            },
        };
        let json = serde_json::to_string(&providers).unwrap();
        assert!(json.contains("\"name\":\"ollama\""));
        assert!(json.contains("\"model\":\"gemma3:latest\""));
        assert!(json.contains("\"model\":\"nomic-embed-text\""));
        assert!(json.contains("\"dimension\":768"));
    }

    /// OODA-11: Test full health response with providers.
    #[test]
    fn test_health_response_with_providers() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
            build_info: None,
            storage_mode: "postgresql".to_string(),
            workspace_id: "default".to_string(),
            components: ComponentHealth {
                kv_storage: true,
                vector_storage: true,
                graph_storage: true,
                llm_provider: true,
            },
            llm_provider_name: Some("openai".to_string()),
            schema: None,
            providers: Some(ProvidersHealth {
                llm: LlmProviderHealth {
                    name: "openai".to_string(),
                    model: "gpt-4.1-nano".to_string(),
                },
                embedding: EmbeddingProviderHealth {
                    name: "openai".to_string(),
                    model: "text-embedding-3-small".to_string(),
                    dimension: 1536,
                },
            }),
            pdf_storage_enabled: Some(true),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"providers\""));
        assert!(json.contains("\"model\":\"gpt-4.1-nano\""));
        assert!(json.contains("\"model\":\"text-embedding-3-small\""));
        assert!(json.contains("\"dimension\":1536"));
        assert!(json.contains("\"pdf_storage_enabled\":true"));
    }
}
