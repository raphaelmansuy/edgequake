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

    /// Operational signals for dashboards (SPEC-024 Phase 4.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operational: Option<OperationalHealth>,

    /// Discoverable API surface links (SPEC-027 REST-008).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ApiCapabilities>,

    /// Application attribution summary (SPEC-043).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<crate::attribution::HealthAttributionSummary>,
}

/// Operator-facing API discovery hints (additive JSON on `/health`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiCapabilities {
    /// OpenAPI document URL.
    pub openapi_url: String,
    /// Standalone AsyncAPI document URL (WebSocket channels).
    pub asyncapi_url: String,
    /// Swagger UI entry point.
    pub swagger_ui_url: String,
    /// Admin API path prefix.
    pub admin_api_prefix: String,
    /// Public shared-conversation path prefix.
    pub shared_conversations_prefix: String,
    /// v2 async jobs API path template (Level 4 — substitute `{workspace_id}`).
    pub jobs_v2_prefix: String,
    /// v2 job catalog path template.
    pub jobs_v2_catalog: String,
    /// Identity/auth SSOT backend label (`postgresql` or `in-memory`) — SPEC-027 phase 55.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_identity_ssot: Option<String>,
    /// Whether JWT/API-key auth is enforced on protected routes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_enabled: Option<bool>,
    /// Local dev opt-out (`EDGEQUAKE_DEV_MODE`) — auth disabled when true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_mode: Option<bool>,
    /// Whether `EDGEQUAKE_KV_IDENTITY_MIRROR` was set in env (may be ignored when PG pool exists).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kv_identity_mirror_configured: Option<bool>,
    /// Effective KV mirror after policy resolution (`false` when PostgreSQL pool is SSOT).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kv_identity_mirror_effective: Option<bool>,
    /// Built-in auth mechanisms (`jwt_password`, `api_key`) — SPEC-027 phase 49.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_mechanisms: Option<Vec<String>>,
    /// Whether OAuth2/OIDC login is implemented in-process (always `false` today).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth2_oidc_builtin: Option<bool>,
    /// Whether in-memory auth harness is active (no PG pool — not KV).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_kv_harness_active: Option<bool>,
    /// Documented external SSO integration pattern when `oauth2_oidc_builtin` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_sso_pattern: Option<String>,
    /// SPEC-091 LD-09: whether `EDGEQUAKE_SERVING_FENCE` is on (Ready vs Indexed badge).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_fence_enabled: Option<bool>,
    /// SPEC-091 IP0 / F-IP-21: provider in-flight budget is cluster-wide (Postgres slot ledger).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_budget_cluster: Option<bool>,
    /// SPEC-091 IP0 / F-IP-21: byte admission (`try_admit`) is process-local — not cluster SSOT.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_admission_process_local: Option<bool>,
    /// SPEC-146: document ABAC enabled (`EDGEQUAKE_DOC_ABAC`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_abac: Option<bool>,
}

/// Task queue + query engine operational snapshot (SPEC-024 Phase 4.3).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OperationalHealth {
    pub task_queue: TaskQueueHealthSnapshot,
    pub query_engine: QueryEngineHealthSnapshot,
    /// Structured logging / OTLP runtime config (SPEC-024 Phase 4.5).
    pub observability: ObservabilityHealthSnapshot,
    /// KV ↔ relational document read-model reconciliation (SPEC-024 Phase 4.6).
    pub read_model: ReadModelHealthSnapshot,
    /// Migration bootstrap summary (SPEC-024 pass 10 — operator visibility).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration: Option<MigrationHealthSnapshot>,
    /// Ingest execution model (SPEC-024 pass 12 — uniformity).
    pub ingestion: IngestionHealthSnapshot,
    /// Chunk storage layout (SPEC-024 pass 12 — storage efficiency).
    pub storage: StorageHealthSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestionHealthSnapshot {
    /// All API document uploads enqueue worker tasks (no sync HTTP persist).
    pub execution_model: String,
    /// Persist saga SSOT trait name for operators.
    pub persist_ssot: String,
    /// Duplicate uploads re-ingest when prior doc is not actively processing.
    pub duplicate_reingest_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StorageHealthSnapshot {
    /// Authoritative chunk text location.
    pub chunk_text_ssot: String,
    /// Vector row metadata references chunk id instead of inline body.
    pub vector_metadata_ref: String,
    /// Chunk KV writes happen inside IngestionPersister (not a second path).
    pub chunk_kv_in_persister: bool,
    /// SPEC-042-E: `full` or `halfvec` (`EDGEQUAKE_VECTOR_STORAGE`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_storage_mode: Option<String>,
    /// SPEC-042-E: `uuidv4` or `uuidv7` document ID generator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id_generator: Option<String>,
    /// SPEC-042-E: AGE 1.7 graph RLS active (`EDGEQUAKE_AGE_RLS=true` on PG17+).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_rls_enabled: Option<bool>,
    /// SPEC-042-E: AGE COPY bulk loader available (AGE >= 1.7.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_copy_loader_enabled: Option<bool>,
    /// SPEC-112: per-role pool max/size/idle (when PgPoolBundle is live).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_pools: Option<Vec<PoolRoleHealth>>,
    /// SPEC-112: boot-time shared-DB budget evaluation result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_budget_ok: Option<bool>,
}

/// SPEC-112 LAW-112-8: one role pool's configured max and live occupancy.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PoolRoleHealth {
    pub role: String,
    pub max: u32,
    pub size: u32,
    pub idle: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MigrationHealthSnapshot {
    pub latest_version: Option<i64>,
    pub source_ids_indexes_ready: bool,
    pub pgvector_extversion: Option<String>,
    pub pgvector_shipped_version: Option<String>,
    pub pgvector_iterative_scan_capable: bool,
    pub age_extversion: Option<String>,
    pub age_shipped_version: Option<String>,
    pub ready_for_traffic: bool,
    /// SPEC-045 SRE-M02: M080 halfvec conversion applied this bootstrap.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub halfvec_conversion_applied: bool,
    /// SPEC-045 SRE-M02: M081 AGE graph RLS applied this bootstrap.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub age_rls_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReadinessResponse {
    pub ready: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskQueueHealthSnapshot {
    pub pending: u64,
    pub processing: u64,
    pub failed: u64,
    /// Backpressure label: `normal`, `elevated`, or `critical`.
    pub pressure: String,
    pub pending_warn_threshold: u64,
    pub pending_critical_threshold: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueryEngineHealthSnapshot {
    /// Default query mode (e.g. `"mix"`, `"hybrid"`).
    pub default_mode: String,
    pub reranker_configured: bool,
    pub community_refresh_debounce_secs: u64,
    /// Hybrid mode chunk merge: `"round_robin"` (LightRAG) or `"rrf"`.
    pub hybrid_fusion: String,
    /// Mix mode chunk merge: `"rrf"` (default), `"max_after_minmax"` (legacy env `weighted`), or `"round_robin"`.
    pub mix_fusion: String,
    /// Workspaces with debounced Louvain refresh scheduled (scale coalescing signal).
    pub community_refresh_scheduled_workspaces: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ObservabilityHealthSnapshot {
    /// Active log format: `"plain"` or `"json"` (`EDGEQUAKE_LOG_FORMAT`).
    pub log_format: String,
    /// Whether OTLP gRPC export is enabled at runtime.
    pub otel_enabled: bool,
    /// SPEC-124: Langfuse keys present and not force-disabled.
    #[serde(default)]
    pub langfuse_enabled: bool,
    /// SPEC-124: non-secret Langfuse base URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub langfuse_base_url: Option<String>,
    /// SPEC-124: requested transport (`auto` | `otlp` | `ingestion`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub langfuse_api: Option<String>,
    /// SPEC-124: transport wired at init (`otlp` | `ingestion`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub langfuse_api_resolved: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReadModelHealthSnapshot {
    /// Merge rule for document counts and list backfill.
    pub merge_strategy: String,
    /// Relational `documents` table backfill when KV metadata is missing.
    pub relational_backfill_enabled: bool,
    /// Per-document entity_count reconciled against AGE graph on list.
    pub entity_count_graph_reconcile: bool,
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

    /// Migration 038 source_ids index readiness (PostgreSQL + AGE only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ids_indexes: Option<SourceIdsIndexHealth>,

    /// SPEC-045: M080 halfvec conversion applied at last bootstrap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halfvec_conversion_applied: Option<bool>,

    /// SPEC-083: AGE graphs with missing `eq_*` columns after M092 reconcile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eq_id_graphs_degraded: Option<Vec<String>>,

    /// SPEC-083: operator opted into property-path SQL (`EDGEQUAKE_EQ_ID_FALLBACK`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eq_id_fallback_enabled: Option<bool>,

    /// SPEC-091 Doc 17 (LAW-B3): embedded migrations not yet applied. Serves
    /// post-boot drift detection (e.g. a replica up while the fleet migrated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_count: Option<usize>,

    /// SPEC-091 Doc 17 (LAW-B3): true when the schema disagrees with this
    /// binary (pending or database-newer) — `edgequake migrate` is required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_required: Option<bool>,

    /// SPEC-091 IW4 (LAW-I6): PostgreSQL / extension runtime capability matrix.
    ///
    /// Derived from `edgequake-storage` `PostgresCapabilityProbe` SSOT — same
    /// fields as the live probe, not recomputed from version strings in handlers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postgres_capabilities: Option<PostgresCapabilityHealth>,
}

/// PostgreSQL runtime capabilities exposed on `/health.schema` (SPEC-091 IW4).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct PostgresCapabilityHealth {
    /// `current_setting('server_version_num') / 10000`.
    pub postgres_major: u32,
    /// Installed pgvector `extversion`, when the extension is present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pgvector_version: Option<String>,
    /// Installed Apache AGE `extversion`, when the extension is present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_version: Option<String>,
    /// Native `uuidv7()` available (PG18+ with function present).
    pub uuidv7_available: bool,
    /// pgvector ≥ 0.8.0 — iterative index scan GUCs (`hnsw.iterative_scan`, etc.).
    pub iterative_scan_available: bool,
    /// AGE ≥ 1.8 — agtype ↔ jsonb bidirectional casts (SPEC-091 RM3).
    pub age_jsonb_agtype_cast_available: bool,
}

#[cfg(feature = "postgres")]
impl From<edgequake_storage::adapters::postgres::PostgresCapabilityProbe>
    for PostgresCapabilityHealth
{
    fn from(probe: edgequake_storage::adapters::postgres::PostgresCapabilityProbe) -> Self {
        Self {
            postgres_major: probe.postgres_major,
            pgvector_version: probe.pgvector_version,
            age_version: probe.age_version,
            uuidv7_available: probe.uuidv7_available,
            iterative_scan_available: probe.iterative_scan_available,
            age_jsonb_agtype_cast_available: probe.age_jsonb_agtype_cast_available,
        }
    }
}

#[cfg(not(feature = "postgres"))]
impl PostgresCapabilityHealth {
    /// Placeholder when postgres feature is disabled (memory-only builds).
    pub fn unavailable() -> Self {
        Self {
            postgres_major: 0,
            pgvector_version: None,
            age_version: None,
            uuidv7_available: false,
            iterative_scan_available: false,
            age_jsonb_agtype_cast_available: false,
        }
    }
}

/// Migration 038 index health surfaced at bootstrap.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceIdsIndexHealth {
    pub ready: bool,
    pub graphs_checked: usize,
    pub indexes_repaired_at_bootstrap: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deferred_large_graphs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_indexes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_action: Option<String>,
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

    /// Apache AGE extension available (PostgreSQL only, SPEC-090 F-090-19).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_available: Option<bool>,

    /// LLM provider status.
    pub llm_provider: bool,

    /// SPEC-083 / P0: AGE `eq_*` denorm columns ready (or fallback env enabled).
    ///
    /// When false, `/ready` refuses traffic unless `EDGEQUAKE_EQ_ID_FALLBACK=1`.
    /// Liveness `/health` stays HTTP 200 with `status: degraded`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eq_id_schema: Option<bool>,
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
                graph_available: None,
                llm_provider: true,
                eq_id_schema: None,
            },
            llm_provider_name: Some("openai".to_string()),
            schema: None,
            providers: None,
            pdf_storage_enabled: None,
            operational: None,
            capabilities: None,
            attribution: None,
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
                graph_available: None,
                llm_provider: true,
                eq_id_schema: Some(true),
            },
            llm_provider_name: Some("ollama".to_string()),
            schema: Some(SchemaHealth {
                latest_version: Some(15),
                migrations_applied: 15,
                last_applied_at: Some("2025-01-26T10:00:00Z".to_string()),
                source_ids_indexes: None,
                halfvec_conversion_applied: None,
                eq_id_graphs_degraded: None,
                eq_id_fallback_enabled: None,
                pending_count: None,
                migration_required: None,
                postgres_capabilities: None,
            }),
            providers: None,
            pdf_storage_enabled: None,
            operational: None,
            capabilities: None,
            attribution: None,
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
            source_ids_indexes: None,
            halfvec_conversion_applied: None,
            eq_id_graphs_degraded: None,
            eq_id_fallback_enabled: None,
            pending_count: None,
            migration_required: None,
            postgres_capabilities: None,
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
            graph_available: None,
            llm_provider: false,
            eq_id_schema: Some(false),
        };
        let json = serde_json::to_string(&components).unwrap();
        assert!(json.contains("\"kv_storage\":false"));
        assert!(json.contains("\"vector_storage\":false"));
        assert!(json.contains("\"graph_storage\":false"));
        assert!(json.contains("\"llm_provider\":false"));
        assert!(json.contains("\"eq_id_schema\":false"));
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
                graph_available: None,
                llm_provider: false,
                eq_id_schema: Some(true),
            },
            llm_provider_name: None,
            schema: None,
            providers: None,
            pdf_storage_enabled: None,
            operational: None,
            capabilities: None,
            attribution: None,
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
            graph_available: None,
            llm_provider: true,
            eq_id_schema: Some(true),
        };
        let json = serde_json::to_string(&components).unwrap();
        assert!(json.contains("\"kv_storage\":true"));
        assert!(json.contains("\"graph_storage\":true"));
        assert!(json.contains("\"eq_id_schema\":true"));
    }

    /// OODA-11: Test providers health serialization.
    #[test]
    fn test_providers_health_serialization() {
        let providers = ProvidersHealth {
            llm: LlmProviderHealth {
                name: "ollama".to_string(),
                model: "gemma4:latest".to_string(),
            },
            embedding: EmbeddingProviderHealth {
                name: "ollama".to_string(),
                model: "nomic-embed-text".to_string(),
                dimension: 768,
            },
        };
        let json = serde_json::to_string(&providers).unwrap();
        assert!(json.contains("\"name\":\"ollama\""));
        assert!(json.contains("\"model\":\"gemma4:latest\""));
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
                graph_available: None,
                llm_provider: true,
                eq_id_schema: Some(true),
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
            operational: None,
            capabilities: None,
            attribution: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"providers\""));
        assert!(json.contains("\"model\":\"gpt-4.1-nano\""));
        assert!(json.contains("\"model\":\"text-embedding-3-small\""));
        assert!(json.contains("\"dimension\":1536"));
        assert!(json.contains("\"pdf_storage_enabled\":true"));
    }
}
