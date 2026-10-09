//! Pipeline configuration and environment-variable defaults (SPEC-017 SRP).

use serde::{Deserialize, Serialize};

use crate::chunker::{ChunkStrategy, ChunkerConfig};

/// Pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Chunking configuration.
    pub chunker: ChunkerConfig,

    /// Chunk strategy selector (SPEC-026 Phase 2).
    #[serde(default)]
    pub chunk_strategy: ChunkStrategy,

    /// Batch size for LLM extraction.
    pub extraction_batch_size: usize,

    /// Batch size for embedding generation.
    pub embedding_batch_size: usize,

    /// Whether to enable entity extraction.
    pub enable_entity_extraction: bool,

    /// Whether to enable relationship extraction.
    pub enable_relationship_extraction: bool,

    /// Whether to generate chunk embeddings.
    pub enable_chunk_embeddings: bool,

    /// Whether to generate entity embeddings.
    pub enable_entity_embeddings: bool,

    /// Whether to generate relationship embeddings.
    pub enable_relationship_embeddings: bool,

    /// Maximum concurrent extraction tasks.
    pub max_concurrent_extractions: usize,

    /// Whether to track document lineage.
    pub enable_lineage_tracking: bool,

    /// Timeout per chunk extraction in seconds.
    #[serde(default = "default_chunk_timeout")]
    pub chunk_extraction_timeout_secs: u64,

    /// Maximum retry attempts per chunk.
    #[serde(default = "default_max_retries")]
    pub chunk_max_retries: u32,

    /// Initial retry delay in milliseconds (for exponential backoff).
    #[serde(default = "default_initial_retry_delay")]
    pub initial_retry_delay_ms: u64,
}

/// Default per-chunk entity-extraction timeout (seconds) — cloud providers.
pub const DEFAULT_CHUNK_TIMEOUT_SECS: u64 = 180;

/// Per-chunk entity-extraction timeout for local providers (Ollama / LM Studio).
///
/// WHY 600: Local GPUs are capacity-bound; gemma4-class models with wide context
/// routinely exceed 180s under even modest concurrency. Matches vision local page budgets.
pub const LOCAL_CHUNK_TIMEOUT_SECS: u64 = 600;

/// Minimum acceptable per-chunk timeout (seconds).
pub const MIN_CHUNK_TIMEOUT_SECS: u64 = 10;

/// Default maximum retry attempts per chunk.
pub const DEFAULT_CHUNK_MAX_RETRIES: u32 = 3;

/// Maximum allowed retry count (safety cap).
pub const MAX_CHUNK_MAX_RETRIES: u32 = 20;

/// Default initial exponential-backoff delay (milliseconds).
pub const DEFAULT_INITIAL_RETRY_DELAY_MS: u64 = 1_000;

/// Minimum backoff for local LLM overload / connection errors (milliseconds).
///
/// WHY 5000: Ollama connection storms need cool-down; 1s retries immediately
/// re-flood the single-slot runner and look like stuck ingestion.
pub const LOCAL_OVERLOAD_RETRY_DELAY_MS: u64 = 5_000;

/// Cap on per-chunk retry backoff (milliseconds).
pub const MAX_RETRY_DELAY_MS: u64 = 60_000;

/// Default maximum concurrent LLM extraction tasks — cloud providers.
pub const DEFAULT_MAX_CONCURRENT_EXTRACTIONS: usize = 16;

/// Concurrent extractions for local providers (Ollama / LM Studio).
///
/// WHY 1: Local inference is typically single-slot (`-np 1`); fan-out >1
/// queues work until chunks time out and storms Ollama connections.
pub const LOCAL_MAX_CONCURRENT_EXTRACTIONS: usize = 1;

/// Hard cap on concurrent extractions (SPEC-046 OPS-P1.6 — OOM / LLM storm guard).
pub const MAX_CONCURRENT_EXTRACTIONS_CAP: usize = 32;

/// Returns true for capacity-bound local inference servers used for entity extraction.
///
/// Excludes `mock` (fast in-process) — only Ollama / LM Studio need the slow profile.
pub fn is_local_extraction_provider(provider_name: &str) -> bool {
    let name = provider_name.trim().to_ascii_lowercase();
    // SPEC-160: a decision backend is one model on one machine, like Ollama.
    name.starts_with(crate::progress::DECISION_PROVIDER_PREFIX)
        || matches!(
            name.as_str(),
            "ollama"
                | "lmstudio"
                | "lm-studio"
                | "lm_studio"
                | "omlx"
                | "mtplx"
                | "llamacpp"
                | "llama-server"
                | "vllm-mlx"
                | "vllm_mlx"
                | "mlx-lm"
                | "mlx_lm"
        )
}

/// Pure priority resolution for fairness clamp provider (SPEC-057 P2).
pub fn resolve_extract_provider_name_for_fairness_from(
    extract: Option<&str>,
    default_extract: Option<&str>,
    default_llm: Option<&str>,
    llm: Option<&str>,
) -> String {
    for candidate in [extract, default_extract, default_llm, llm] {
        if let Some(v) = candidate.map(str::trim).filter(|s| !s.is_empty()) {
            return v.to_string();
        }
    }
    String::new()
}

/// Resolve the extract provider name used for worker-pool fairness clamps (SPEC-057 P2).
///
/// Prefer explicit extract overrides so hybrid setups (e.g. OpenAI LLM + Ollama
/// extract via `EDGEQUAKE_EXTRACT_PROVIDER`) clamp local concurrency correctly.
pub fn resolve_extract_provider_name_for_fairness() -> String {
    let extract = std::env::var("EDGEQUAKE_EXTRACT_PROVIDER").ok();
    let default_extract = std::env::var("EDGEQUAKE_DEFAULT_EXTRACT_PROVIDER").ok();
    let default_llm = std::env::var("EDGEQUAKE_DEFAULT_LLM_PROVIDER").ok();
    let llm = std::env::var("EDGEQUAKE_LLM_PROVIDER").ok();
    resolve_extract_provider_name_for_fairness_from(
        extract.as_deref(),
        default_extract.as_deref(),
        default_llm.as_deref(),
        llm.as_deref(),
    )
}

/// Default per-chunk timeout for a provider when `EDGEQUAKE_CHUNK_TIMEOUT_SECS` is unset.
pub fn default_chunk_timeout_for_provider(provider_name: &str) -> u64 {
    if is_local_extraction_provider(provider_name) {
        LOCAL_CHUNK_TIMEOUT_SECS
    } else {
        DEFAULT_CHUNK_TIMEOUT_SECS
    }
}

/// Default concurrent extractions for a provider when env override is unset.
pub fn default_max_concurrent_for_provider(provider_name: &str) -> usize {
    if is_local_extraction_provider(provider_name) {
        LOCAL_MAX_CONCURRENT_EXTRACTIONS
    } else {
        DEFAULT_MAX_CONCURRENT_EXTRACTIONS
    }
}

/// Hard cap on gleaning passes (SPEC-046 OPS-P1.6 — cost/latency bound).
pub const MAX_GLEANING_CAP: usize = 2;

/// Clamp gleaning passes to `[0, MAX_GLEANING_CAP]` (pure — non-flaky tests).
pub fn clamp_max_gleaning(raw: usize) -> usize {
    raw.min(MAX_GLEANING_CAP)
}

/// Clamp concurrent extractions to `[1, MAX_CONCURRENT_EXTRACTIONS_CAP]`.
pub fn clamp_max_concurrent_extractions(raw: usize) -> usize {
    raw.clamp(1, MAX_CONCURRENT_EXTRACTIONS_CAP)
}

/// Env flag to opt out of the local-provider concurrency safety clamp.
///
/// When set to `1`/`true`/`yes`, operators may raise
/// `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` above [`LOCAL_MAX_CONCURRENT_EXTRACTIONS`]
/// for Ollama / LM Studio (e.g. multi-GPU benches).
pub const ALLOW_LOCAL_HIGH_CONCURRENCY_ENV: &str = "EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY";

/// Returns true when operators explicitly allow high concurrency on local LLMs.
pub fn allow_local_high_concurrency() -> bool {
    matches!(
        std::env::var(ALLOW_LOCAL_HIGH_CONCURRENCY_ENV)
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// Cap a fan-out figure for capacity-bound local providers (SSOT).
///
/// First principle: a concurrency limit is a property of the **upstream that
/// is actually called**, never of the process default. Local inference servers
/// (Ollama / LM Studio) are compute-bound, so extra fan-out only queues; cloud
/// APIs are latency-bound, so fan-out hides round-trip time. Callers pass the
/// provider name of the unit of work being scheduled (embedding provider for
/// embeds, LLM provider for merges, ...), so a Mistral workspace is never
/// throttled just because the process default happens to be Ollama.
///
/// `local_cap` applies to local providers unless
/// [`ALLOW_LOCAL_HIGH_CONCURRENCY_ENV`] is set. Always returns `>= 1`.
pub fn cap_for_local_provider(provider_name: &str, requested: usize, local_cap: usize) -> usize {
    let requested = requested.max(1);
    if is_local_extraction_provider(provider_name) && !allow_local_high_concurrency() {
        requested.min(local_cap.max(1))
    } else {
        requested
    }
}

/// Cap concurrent extractions for capacity-bound local providers.
///
/// Ollama defaults to ~1 parallel sequence (`OLLAMA_NUM_PARALLEL`). Cloud-scale
/// fan-out (e.g. Makefile `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=32`) causes
/// connection storms that look like stuck ingestion. Unless
/// [`ALLOW_LOCAL_HIGH_CONCURRENCY_ENV`] is set, local providers are capped at
/// [`LOCAL_MAX_CONCURRENT_EXTRACTIONS`].
///
/// Returns `(effective, clamped)` where `clamped` is true when the requested
/// value was reduced.
pub fn apply_local_concurrency_safety_clamp(
    provider_name: &str,
    requested: usize,
) -> (usize, bool) {
    let bounded = clamp_max_concurrent_extractions(requested);
    if !is_local_extraction_provider(provider_name) || allow_local_high_concurrency() {
        return (bounded, false);
    }
    if bounded > LOCAL_MAX_CONCURRENT_EXTRACTIONS {
        (LOCAL_MAX_CONCURRENT_EXTRACTIONS, true)
    } else {
        (bounded, false)
    }
}

/// Local Ollama/LM Studio worker-pool ceilings (unless high-concurrency opt-out).
pub const LOCAL_WORKER_THREADS_CAP: usize = 4;
/// Local ingest fairness lane cap (Pdf/Insert) — protects LLM/vision.
pub const LOCAL_MAX_INGEST_TASKS_PER_TENANT_CAP: usize = 1;
/// Local lifecycle fairness lane default (Deletion/Wipe) — DB/graph bound.
pub const LOCAL_DEFAULT_LIFECYCLE_TASKS_PER_TENANT: usize = 4;
pub const LOCAL_MAX_LIFECYCLE_TASKS_PER_TENANT_CAP: usize = 4;

/// Resolved worker pool + dual-lane tenant fairness limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerPoolLimits {
    pub num_workers: usize,
    /// Ingest lane max (`0` = unlimited).
    pub max_ingest_per_tenant: usize,
    /// Lifecycle lane max (`0` = unlimited).
    pub max_lifecycle_per_tenant: usize,
    /// True when local clamp reduced workers or ingest vs the requested values.
    pub local_clamped: bool,
}

/// Pure resolver for worker pool + ingest/lifecycle tenant caps (testable SSOT).
pub fn resolve_worker_pool_limits_from(
    provider: &str,
    allow_high_concurrency: bool,
    requested_workers: usize,
    requested_ingest_per_tenant: usize,
    lifecycle_override: Option<usize>,
) -> WorkerPoolLimits {
    if !is_local_extraction_provider(provider) || allow_high_concurrency {
        let lifecycle = lifecycle_override.unwrap_or(requested_ingest_per_tenant);
        return WorkerPoolLimits {
            num_workers: requested_workers,
            max_ingest_per_tenant: requested_ingest_per_tenant,
            max_lifecycle_per_tenant: lifecycle,
            local_clamped: false,
        };
    }

    let workers = requested_workers.clamp(1, LOCAL_WORKER_THREADS_CAP);
    let ingest = if requested_ingest_per_tenant == 0 {
        0
    } else {
        requested_ingest_per_tenant.clamp(1, LOCAL_MAX_INGEST_TASKS_PER_TENANT_CAP)
    };
    let lifecycle = match lifecycle_override {
        Some(0) => 0,
        Some(n) => n.clamp(1, LOCAL_MAX_LIFECYCLE_TASKS_PER_TENANT_CAP),
        None => LOCAL_DEFAULT_LIFECYCLE_TASKS_PER_TENANT
            .max(requested_ingest_per_tenant.min(LOCAL_MAX_LIFECYCLE_TASKS_PER_TENANT_CAP)),
    };
    WorkerPoolLimits {
        num_workers: workers,
        max_ingest_per_tenant: ingest,
        max_lifecycle_per_tenant: lifecycle,
        local_clamped: workers != requested_workers || ingest != requested_ingest_per_tenant,
    }
}

fn default_worker_threads() -> usize {
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    (cpus * 4).max(4)
}

/// Resolve worker pool size + ingest/lifecycle per-tenant limits from env + extract provider.
pub fn resolve_worker_pool_limits() -> WorkerPoolLimits {
    let requested_workers: usize = std::env::var("WORKER_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(default_worker_threads);

    let requested_ingest: usize = std::env::var("MAX_TASKS_PER_TENANT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| (requested_workers * 3 / 4).max(1));

    let lifecycle_override = std::env::var("MAX_LIFECYCLE_TASKS_PER_TENANT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok());

    let provider = resolve_extract_provider_name_for_fairness();
    resolve_worker_pool_limits_from(
        &provider,
        allow_local_high_concurrency(),
        requested_workers,
        requested_ingest,
        lifecycle_override,
    )
}

/// True when a chunk extraction error looks like local LLM overload / unreachable.
pub fn is_local_provider_overload_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("network error")
        || lower.contains("connection refused")
        || lower.contains("error sending request")
        || lower.contains("localhost:11434")
        || lower.contains("server busy")
        || lower.contains("503")
        || lower.contains("too many requests")
}

/// Taxonomy for extract retry budgets (transient vs parse vs permanent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractErrorClass {
    /// Network / overload / timeout — full retry budget.
    Transient,
    /// JSON/schema parse failure — limited retries (repair turn handles most).
    Parse,
    /// Non-retryable business / enforce failure.
    Permanent,
}

/// Classify a chunk extraction error for retry policy.
pub fn classify_extract_error(error: &str) -> ExtractErrorClass {
    let lower = error.to_ascii_lowercase();
    if is_local_provider_overload_error(error)
        || lower.contains("timeout")
        || lower.contains("provider_unavailable")
        || lower.contains("circuit")
    {
        return ExtractErrorClass::Transient;
    }
    if lower.contains("invalid json")
        || lower.contains("parse")
        || lower.contains("schema")
        || lower.contains("empty response")
    {
        return ExtractErrorClass::Parse;
    }
    if lower.contains("cancelled") || lower.contains("enforce") {
        return ExtractErrorClass::Permanent;
    }
    ExtractErrorClass::Transient
}

/// Max attempts for a classified error (includes the first try).
pub fn extract_retry_budget(class: ExtractErrorClass, configured_max: u32) -> u32 {
    match class {
        ExtractErrorClass::Transient => configured_max.max(1),
        ExtractErrorClass::Parse => configured_max.clamp(1, 2),
        ExtractErrorClass::Permanent => 1,
    }
}

/// Compute exponential backoff delay, stretching the base for local overload errors.
pub fn retry_delay_ms_for_chunk_error(initial_delay_ms: u64, attempt: u32, error: &str) -> u64 {
    let base = if is_local_provider_overload_error(error) {
        initial_delay_ms.max(LOCAL_OVERLOAD_RETRY_DELAY_MS)
    } else {
        initial_delay_ms
    };
    let exp = attempt.saturating_sub(1).min(6);
    base.saturating_mul(2_u64.pow(exp)).min(MAX_RETRY_DELAY_MS)
}

/// Env flag to keep gleaning enabled on local extract providers.
pub const LOCAL_ENABLE_GLEANING_ENV: &str = "EDGEQUAKE_LOCAL_ENABLE_GLEANING";

/// Returns true when operators opt in to gleaning on Ollama / LM Studio.
pub fn allow_local_gleaning() -> bool {
    matches!(
        std::env::var(LOCAL_ENABLE_GLEANING_ENV)
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// Resolve whether gleaning should run for a provider.
///
/// Local providers default to off (gleaning doubles Ollama load). Cloud keeps
/// the caller’s `enable_gleaning` flag. Opt in locally via
/// [`LOCAL_ENABLE_GLEANING_ENV`] or `allow_local_gleaning_flag`.
pub fn resolve_gleaning_for_provider(
    provider_name: &str,
    enable_gleaning: bool,
    max_gleaning: usize,
    allow_local_gleaning_flag: bool,
) -> (bool, usize) {
    if !enable_gleaning || max_gleaning == 0 {
        return (false, 0);
    }
    if is_local_extraction_provider(provider_name)
        && !(allow_local_gleaning_flag || allow_local_gleaning())
    {
        return (false, 0);
    }
    (true, clamp_max_gleaning(max_gleaning))
}

fn default_chunk_timeout() -> u64 {
    DEFAULT_CHUNK_TIMEOUT_SECS
}

fn default_max_retries() -> u32 {
    DEFAULT_CHUNK_MAX_RETRIES
}

fn default_initial_retry_delay() -> u64 {
    DEFAULT_INITIAL_RETRY_DELAY_MS
}

fn read_env_u64(name: &str, default: u64, min_val: u64, max_val: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(default)
        .clamp(min_val, max_val)
}

fn read_env_u32(name: &str, default: u32, min_val: u32, max_val: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default)
        .clamp(min_val, max_val)
}

fn read_env_usize(name: &str, default: usize, min_val: usize, max_val: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default)
        .clamp(min_val, max_val)
}

/// Ingest profile names for `EDGEQUAKE_INGEST_PROFILE` (SPEC-047 P4 / retrieve-only eval).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestProfile {
    /// Full KG + embeddings (default).
    Full,
    /// Chunk + chunk embeddings only — skip entity/relationship extract & embed.
    ChunkOnly,
}

impl IngestProfile {
    /// Parse profile from env string (`chunk_only` / `retrieve_only` → ChunkOnly).
    pub fn from_env_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "chunk_only" | "retrieve_only" | "p0_retrieve_only" => Self::ChunkOnly,
            _ => Self::Full,
        }
    }

    /// Read `EDGEQUAKE_INGEST_PROFILE` (default Full).
    pub fn from_env() -> Self {
        std::env::var("EDGEQUAKE_INGEST_PROFILE")
            .ok()
            .map(|s| Self::from_env_str(&s))
            .unwrap_or(Self::Full)
    }

    /// Apply profile flags onto a config (entity/rel extract + embed gates).
    pub fn apply_to(self, config: &mut PipelineConfig) {
        if self == Self::ChunkOnly {
            config.enable_entity_extraction = false;
            config.enable_relationship_extraction = false;
            config.enable_entity_embeddings = false;
            config.enable_relationship_embeddings = false;
            // Chunk embeddings stay on — required for retrieve-only RAG.
            config.enable_chunk_embeddings = true;
        }
    }
}

impl PipelineConfig {
    /// Create a `PipelineConfig` from environment variables, falling back to cloud defaults.
    pub fn from_env() -> Self {
        Self::from_env_for_provider("")
    }

    /// Create a `PipelineConfig` with provider-aware defaults for unset env vars.
    ///
    /// - Cloud / unknown: 180s chunk timeout, 16 concurrent
    /// - Ollama / LM Studio: 600s chunk timeout, 2 concurrent
    ///
    /// Explicit `EDGEQUAKE_CHUNK_TIMEOUT_SECS` wins over provider defaults.
    /// `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` is honored for cloud; for local
    /// providers it is capped at [`LOCAL_MAX_CONCURRENT_EXTRACTIONS`] unless
    /// [`ALLOW_LOCAL_HIGH_CONCURRENCY_ENV`] is set.
    pub fn from_env_for_provider(provider_name: &str) -> Self {
        let default_timeout = default_chunk_timeout_for_provider(provider_name);
        let default_concurrent = default_max_concurrent_for_provider(provider_name);

        let chunk_timeout = read_env_u64(
            "EDGEQUAKE_CHUNK_TIMEOUT_SECS",
            default_timeout,
            MIN_CHUNK_TIMEOUT_SECS,
            u64::MAX,
        );
        // C-18: min 1 attempt — `0` previously skipped extraction silently.
        let max_retries = read_env_u32(
            "EDGEQUAKE_CHUNK_MAX_RETRIES",
            DEFAULT_CHUNK_MAX_RETRIES,
            1,
            MAX_CHUNK_MAX_RETRIES,
        );
        let retry_delay = read_env_u64(
            "EDGEQUAKE_CHUNK_RETRY_DELAY_MS",
            DEFAULT_INITIAL_RETRY_DELAY_MS,
            0,
            60_000,
        );
        let requested_concurrent = clamp_max_concurrent_extractions(read_env_usize(
            "EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS",
            default_concurrent,
            1,
            MAX_CONCURRENT_EXTRACTIONS_CAP,
        ));
        let (max_concurrent, local_clamped) =
            apply_local_concurrency_safety_clamp(provider_name, requested_concurrent);
        if local_clamped {
            tracing::warn!(
                provider = provider_name,
                requested = requested_concurrent,
                effective = max_concurrent,
                allow_env = ALLOW_LOCAL_HIGH_CONCURRENCY_ENV,
                "Capped concurrent extractions for local LLM to avoid Ollama connection storms; \
                 set {}=1 to opt out",
                ALLOW_LOCAL_HIGH_CONCURRENCY_ENV
            );
        }

        let mut config = Self {
            chunk_extraction_timeout_secs: chunk_timeout,
            chunk_max_retries: max_retries,
            initial_retry_delay_ms: retry_delay,
            max_concurrent_extractions: max_concurrent,
            ..Self::default()
        };
        IngestProfile::from_env().apply_to(&mut config);
        config
    }

    /// Apply local/cloud extraction defaults in place when env overrides are absent.
    pub fn with_provider_defaults(mut self, provider_name: &str) -> Self {
        let tuned = Self::from_env_for_provider(provider_name);
        self.chunk_extraction_timeout_secs = tuned.chunk_extraction_timeout_secs;
        self.max_concurrent_extractions = tuned.max_concurrent_extractions;
        self.chunk_max_retries = tuned.chunk_max_retries;
        self.initial_retry_delay_ms = tuned.initial_retry_delay_ms;
        self.enable_entity_extraction = tuned.enable_entity_extraction;
        self.enable_relationship_extraction = tuned.enable_relationship_extraction;
        self.enable_entity_embeddings = tuned.enable_entity_embeddings;
        self.enable_relationship_embeddings = tuned.enable_relationship_embeddings;
        self.enable_chunk_embeddings = tuned.enable_chunk_embeddings;
        self
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunker: ChunkerConfig::default(),
            chunk_strategy: ChunkStrategy::default(),
            extraction_batch_size: 10,
            embedding_batch_size: 100,
            enable_entity_extraction: true,
            enable_relationship_extraction: true,
            enable_chunk_embeddings: true,
            enable_entity_embeddings: true,
            enable_relationship_embeddings: true,
            max_concurrent_extractions: DEFAULT_MAX_CONCURRENT_EXTRACTIONS,
            enable_lineage_tracking: true,
            chunk_extraction_timeout_secs: DEFAULT_CHUNK_TIMEOUT_SECS,
            chunk_max_retries: DEFAULT_CHUNK_MAX_RETRIES,
            initial_retry_delay_ms: DEFAULT_INITIAL_RETRY_DELAY_MS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_worker_pool_limits_raise_tenant_lanes() {
        let limits = resolve_worker_pool_limits_from("ollama", false, 16, 12, None);
        assert_eq!(limits.num_workers, LOCAL_WORKER_THREADS_CAP);
        assert_eq!(
            limits.max_ingest_per_tenant,
            LOCAL_MAX_INGEST_TASKS_PER_TENANT_CAP
        );
        assert_eq!(
            limits.max_lifecycle_per_tenant,
            LOCAL_DEFAULT_LIFECYCLE_TASKS_PER_TENANT
        );
        assert!(limits.local_clamped);
    }

    #[test]
    fn cloud_worker_pool_limits_pass_through() {
        let limits = resolve_worker_pool_limits_from("openai", false, 16, 12, None);
        assert_eq!(
            limits,
            WorkerPoolLimits {
                num_workers: 16,
                max_ingest_per_tenant: 12,
                max_lifecycle_per_tenant: 12,
                local_clamped: false,
            }
        );
    }

    #[test]
    fn local_lifecycle_override_and_ingest_unlimited() {
        let limits = resolve_worker_pool_limits_from("lmstudio", false, 8, 0, Some(2));
        assert_eq!(limits.max_ingest_per_tenant, 0);
        assert_eq!(limits.max_lifecycle_per_tenant, 2);
        assert_eq!(limits.num_workers, LOCAL_WORKER_THREADS_CAP);
    }

    #[test]
    fn allow_high_concurrency_skips_local_clamp() {
        let limits = resolve_worker_pool_limits_from("ollama", true, 16, 12, Some(3));
        assert_eq!(limits.num_workers, 16);
        assert_eq!(limits.max_ingest_per_tenant, 12);
        assert_eq!(limits.max_lifecycle_per_tenant, 3);
        assert!(!limits.local_clamped);
    }

    #[test]
    fn clamp_gleaning_caps_at_two() {
        assert_eq!(clamp_max_gleaning(0), 0);
        assert_eq!(clamp_max_gleaning(1), 1);
        assert_eq!(clamp_max_gleaning(2), 2);
        assert_eq!(clamp_max_gleaning(99), MAX_GLEANING_CAP);
    }

    #[test]
    fn clamp_concurrent_extractions_bounds() {
        assert_eq!(clamp_max_concurrent_extractions(0), 1);
        assert_eq!(clamp_max_concurrent_extractions(16), 16);
        assert_eq!(
            clamp_max_concurrent_extractions(256),
            MAX_CONCURRENT_EXTRACTIONS_CAP
        );
    }

    #[test]
    fn ingest_profile_chunk_only_disables_kg_extract() {
        assert_eq!(
            IngestProfile::from_env_str("chunk_only"),
            IngestProfile::ChunkOnly
        );
        assert_eq!(
            IngestProfile::from_env_str("retrieve_only"),
            IngestProfile::ChunkOnly
        );
        assert_eq!(IngestProfile::from_env_str("full"), IngestProfile::Full);

        let mut config = PipelineConfig::default();
        IngestProfile::ChunkOnly.apply_to(&mut config);
        assert!(!config.enable_entity_extraction);
        assert!(!config.enable_relationship_extraction);
        assert!(!config.enable_entity_embeddings);
        assert!(!config.enable_relationship_embeddings);
        assert!(config.enable_chunk_embeddings);
    }

    #[test]
    fn local_extraction_provider_detection() {
        assert!(is_local_extraction_provider("ollama"));
        assert!(is_local_extraction_provider("LMStudio"));
        assert!(!is_local_extraction_provider("openai"));
        assert!(!is_local_extraction_provider("mistral"));
        assert!(!is_local_extraction_provider("mock"));
    }

    #[test]
    fn hybrid_openai_llm_ollama_extract_fairness_uses_extract_provider() {
        // Cloud LLM alone → not local.
        let llm_only = resolve_extract_provider_name_for_fairness_from(
            None,
            None,
            Some("openai"),
            Some("openai"),
        );
        assert!(!is_local_extraction_provider(&llm_only));

        // Hybrid: extract override is local even when LLM is cloud.
        let hybrid = resolve_extract_provider_name_for_fairness_from(
            Some("ollama"),
            None,
            Some("openai"),
            Some("openai"),
        );
        assert_eq!(hybrid, "ollama");
        assert!(is_local_extraction_provider(&hybrid));
    }

    #[test]
    fn provider_defaults_local_vs_cloud() {
        assert_eq!(
            default_chunk_timeout_for_provider("ollama"),
            LOCAL_CHUNK_TIMEOUT_SECS
        );
        assert_eq!(
            default_chunk_timeout_for_provider("mistral"),
            DEFAULT_CHUNK_TIMEOUT_SECS
        );
        assert_eq!(
            default_max_concurrent_for_provider("lmstudio"),
            LOCAL_MAX_CONCURRENT_EXTRACTIONS
        );
        assert_eq!(
            default_max_concurrent_for_provider("openai"),
            DEFAULT_MAX_CONCURRENT_EXTRACTIONS
        );
    }

    #[test]
    fn local_concurrency_safety_clamp_caps_ollama() {
        // Serialise env — parallel tests may leave ALLOW_LOCAL_HIGH_CONCURRENCY set.
        let _lock = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev_allow = std::env::var(ALLOW_LOCAL_HIGH_CONCURRENCY_ENV).ok();
        unsafe {
            std::env::remove_var(ALLOW_LOCAL_HIGH_CONCURRENCY_ENV);
        }

        let (effective, clamped) = apply_local_concurrency_safety_clamp("ollama", 32);
        assert_eq!(effective, LOCAL_MAX_CONCURRENT_EXTRACTIONS);
        assert!(clamped);

        let (effective, clamped) = apply_local_concurrency_safety_clamp("ollama", 2);
        assert_eq!(effective, LOCAL_MAX_CONCURRENT_EXTRACTIONS);
        assert!(clamped);

        let (effective, clamped) = apply_local_concurrency_safety_clamp("openai", 32);
        assert_eq!(effective, 32);
        assert!(!clamped);

        unsafe {
            match prev_allow {
                Some(v) => std::env::set_var(ALLOW_LOCAL_HIGH_CONCURRENCY_ENV, v),
                None => std::env::remove_var(ALLOW_LOCAL_HIGH_CONCURRENCY_ENV),
            }
        }
    }

    #[test]
    fn local_concurrency_safety_clamp_respects_allow_flag() {
        // Serialise env mutations — cargo test runs this module's tests in parallel otherwise.
        let _lock = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev_allow = std::env::var(ALLOW_LOCAL_HIGH_CONCURRENCY_ENV).ok();
        let prev_conc = std::env::var("EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS").ok();
        let prev_profile = std::env::var("EDGEQUAKE_INGEST_PROFILE").ok();

        unsafe {
            std::env::remove_var(ALLOW_LOCAL_HIGH_CONCURRENCY_ENV);
            std::env::set_var("EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS", "32");
            std::env::remove_var("EDGEQUAKE_INGEST_PROFILE");
        }
        let capped = PipelineConfig::from_env_for_provider("ollama");
        assert_eq!(
            capped.max_concurrent_extractions,
            LOCAL_MAX_CONCURRENT_EXTRACTIONS
        );

        unsafe {
            std::env::set_var(ALLOW_LOCAL_HIGH_CONCURRENCY_ENV, "1");
        }
        let allowed = PipelineConfig::from_env_for_provider("ollama");
        assert_eq!(allowed.max_concurrent_extractions, 32);

        let cloud = PipelineConfig::from_env_for_provider("openai");
        assert_eq!(cloud.max_concurrent_extractions, 32);

        unsafe {
            std::env::remove_var("EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS");
            std::env::remove_var(ALLOW_LOCAL_HIGH_CONCURRENCY_ENV);
        }
        let unset_local = PipelineConfig::from_env_for_provider("ollama");
        assert_eq!(
            unset_local.max_concurrent_extractions,
            LOCAL_MAX_CONCURRENT_EXTRACTIONS
        );

        // Restore prior env so other tests are not polluted.
        unsafe {
            match prev_allow {
                Some(v) => std::env::set_var(ALLOW_LOCAL_HIGH_CONCURRENCY_ENV, v),
                None => std::env::remove_var(ALLOW_LOCAL_HIGH_CONCURRENCY_ENV),
            }
            match prev_conc {
                Some(v) => std::env::set_var("EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS", v),
                None => std::env::remove_var("EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS"),
            }
            match prev_profile {
                Some(v) => std::env::set_var("EDGEQUAKE_INGEST_PROFILE", v),
                None => std::env::remove_var("EDGEQUAKE_INGEST_PROFILE"),
            }
        }
    }

    #[test]
    fn overload_error_detection_and_backoff() {
        assert!(is_local_provider_overload_error(
            "Network error: error sending request for url (http://localhost:11434/api/chat)"
        ));
        assert!(is_local_provider_overload_error("HTTP 503 server busy"));
        assert!(!is_local_provider_overload_error("Timeout after 600s"));

        let delay = retry_delay_ms_for_chunk_error(1_000, 1, "Network error");
        assert_eq!(delay, LOCAL_OVERLOAD_RETRY_DELAY_MS);
        let delay2 = retry_delay_ms_for_chunk_error(1_000, 2, "Network error");
        assert_eq!(delay2, LOCAL_OVERLOAD_RETRY_DELAY_MS * 2);
        let normal = retry_delay_ms_for_chunk_error(1_000, 1, "parse error");
        assert_eq!(normal, 1_000);
    }

    #[test]
    fn classify_extract_error_taxonomy() {
        assert_eq!(
            classify_extract_error("Network error: connection refused"),
            ExtractErrorClass::Transient
        );
        assert_eq!(
            classify_extract_error("Invalid JSON: empty response"),
            ExtractErrorClass::Parse
        );
        assert_eq!(
            classify_extract_error("Task cancelled"),
            ExtractErrorClass::Permanent
        );
        assert_eq!(extract_retry_budget(ExtractErrorClass::Parse, 5), 2);
        assert_eq!(extract_retry_budget(ExtractErrorClass::Permanent, 5), 1);
        assert_eq!(extract_retry_budget(ExtractErrorClass::Transient, 5), 5);
    }

    #[test]
    fn local_gleaning_disabled_by_default() {
        let (enable, max) = resolve_gleaning_for_provider("ollama", true, 1, false);
        assert!(!enable);
        assert_eq!(max, 0);

        let (enable, max) = resolve_gleaning_for_provider("ollama", true, 1, true);
        assert!(enable);
        assert_eq!(max, 1);

        let (enable, max) = resolve_gleaning_for_provider("openai", true, 1, false);
        assert!(enable);
        assert_eq!(max, 1);
    }

    static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // T-160-U53 — decision backends use the local, one-at-a-time profile (EC-160-08).
    #[test]
    fn decision_provider_is_local() {
        assert!(is_local_extraction_provider("decision:ollama"));
        assert!(is_local_extraction_provider(" Decision:OpenAI_Logprobs "));
        assert!(is_local_extraction_provider("ollama"));
        assert!(!is_local_extraction_provider("openai"));
        let cfg = PipelineConfig::from_env_for_provider("decision:ollama");
        assert_eq!(
            cfg.max_concurrent_extractions,
            LOCAL_MAX_CONCURRENT_EXTRACTIONS
        );
    }
}
