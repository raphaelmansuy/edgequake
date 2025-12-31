# Phase 2: API Integration

## Objective

Expose SOTA features through the REST API with proper configuration.

## Duration: 2-3 hours

---

## Task 2.1: Extend Query Request Schema

### Current State

- API has `enable_rerank` but missing gleaning config
- Located at [edgequake/crates/edgequake-api/src/handlers/query.rs](../edgequake/crates/edgequake-api/src/handlers/query.rs)

### Changes Required

**File: [edgequake/crates/edgequake-api/src/handlers/query.rs](../edgequake/crates/edgequake-api/src/handlers/query.rs)**

```rust
/// Extended query request with SOTA features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// The query text.
    pub query: String,

    /// Query mode: local, global, hybrid, or adaptive.
    #[serde(default = "default_mode")]
    pub mode: QueryMode,

    // --- SOTA Features ---

    /// Enable reranking for improved precision.
    #[serde(default)]
    pub enable_rerank: bool,

    /// Reranking model (jina, cohere, aliyun).
    #[serde(default)]
    pub rerank_model: Option<String>,

    /// Top-K results to keep after reranking.
    #[serde(default = "default_rerank_top_k")]
    pub rerank_top_k: Option<usize>,

    /// Minimum rerank score threshold.
    #[serde(default)]
    pub min_rerank_score: Option<f32>,

    /// Enable keyword extraction (for adaptive mode).
    #[serde(default = "default_true")]
    pub enable_keywords: bool,

    /// Enable degree-based entity ranking.
    #[serde(default = "default_true")]
    pub enable_degree_ranking: bool,

    /// Maximum entities to include.
    #[serde(default = "default_max_entities")]
    pub max_entities: Option<usize>,

    /// Maximum relationships to include.
    #[serde(default = "default_max_relationships")]
    pub max_relationships: Option<usize>,

    /// Token budget for context.
    #[serde(default = "default_token_budget")]
    pub token_budget: Option<usize>,

    /// Include source chunks in response.
    #[serde(default = "default_true")]
    pub include_sources: bool,
}

fn default_mode() -> QueryMode { QueryMode::Adaptive }
fn default_true() -> bool { true }
fn default_rerank_top_k() -> Option<usize> { Some(10) }
fn default_max_entities() -> Option<usize> { Some(20) }
fn default_max_relationships() -> Option<usize> { Some(50) }
fn default_token_budget() -> Option<usize> { Some(4096) }
```

---

## Task 2.2: Extend Query Response Schema

**File: [edgequake/crates/edgequake-api/src/handlers/query.rs](../edgequake/crates/edgequake-api/src/handlers/query.rs)**

```rust
/// Extended query response with SOTA metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// The generated answer.
    pub answer: String,

    /// Query mode used (may differ from requested in adaptive mode).
    pub mode: QueryMode,

    /// Whether reranking was applied.
    pub reranked: bool,

    /// Source chunks used to generate the answer.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SourceChunk>,

    /// Processing statistics.
    pub stats: QueryStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceChunk {
    /// Source document ID.
    pub doc_id: String,

    /// Chunk content (truncated).
    pub content: String,

    /// Relevance score.
    pub score: f32,

    /// Rerank score (if reranking was applied).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStats {
    /// Total latency in milliseconds.
    pub latency_ms: u64,

    /// Retrieval latency in milliseconds.
    pub retrieval_ms: u64,

    /// Generation latency in milliseconds.
    pub generation_ms: u64,

    /// Entities retrieved.
    pub entities_count: usize,

    /// Relationships retrieved.
    pub relationships_count: usize,

    /// Chunks retrieved.
    pub chunks_count: usize,

    /// Tokens used for context.
    pub context_tokens: usize,

    /// Keywords extracted (if adaptive mode).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}
```

---

## Task 2.3: Extend Ingestion Request Schema

**File: [edgequake/crates/edgequake-api/src/handlers/ingest.rs](../edgequake/crates/edgequake-api/src/handlers/ingest.rs)**

```rust
/// Extended ingestion request with SOTA features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    /// Document content.
    pub content: String,

    /// Document ID (optional, auto-generated if not provided).
    #[serde(default)]
    pub doc_id: Option<String>,

    /// Document metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    // --- SOTA Features ---

    /// Enable gleaning for multi-pass extraction.
    #[serde(default = "default_true")]
    pub enable_gleaning: bool,

    /// Maximum gleaning iterations (1-3 recommended).
    #[serde(default = "default_max_gleaning")]
    pub max_gleaning: Option<usize>,

    /// Enable LLM-based description merging.
    #[serde(default = "default_true")]
    pub use_llm_summarization: bool,

    /// Chunking size in tokens.
    #[serde(default = "default_chunk_size")]
    pub chunk_size: Option<usize>,

    /// Chunk overlap in tokens.
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: Option<usize>,
}

fn default_max_gleaning() -> Option<usize> { Some(1) }
fn default_chunk_size() -> Option<usize> { Some(1200) }
fn default_chunk_overlap() -> Option<usize> { Some(100) }
```

---

## Task 2.4: Extend Ingestion Response Schema

**File: [edgequake/crates/edgequake-api/src/handlers/ingest.rs](../edgequake/crates/edgequake-api/src/handlers/ingest.rs)**

```rust
/// Extended ingestion response with SOTA metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResponse {
    /// Whether ingestion was successful.
    pub success: bool,

    /// Document ID.
    pub doc_id: String,

    /// Processing statistics.
    pub stats: IngestStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestStats {
    /// Total latency in milliseconds.
    pub latency_ms: u64,

    /// Chunks processed.
    pub chunks_count: usize,

    /// Entities extracted.
    pub entities_count: usize,

    /// Relationships extracted.
    pub relationships_count: usize,

    /// New entities added (not duplicates).
    pub new_entities_count: usize,

    /// New relationships added.
    pub new_relationships_count: usize,

    /// Descriptions merged via LLM.
    pub descriptions_merged: usize,

    /// Gleaning iterations performed.
    pub gleaning_iterations: usize,

    /// Tokens used.
    pub tokens_used: usize,

    /// Estimated cost in USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
}
```

---

## Task 2.5: Create Configuration Endpoint

**File: [edgequake/crates/edgequake-api/src/handlers/config.rs](../edgequake/crates/edgequake-api/src/handlers/config.rs)** (NEW)

```rust
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

/// Server configuration info.
#[derive(Debug, Clone, Serialize)]
pub struct ConfigResponse {
    /// LLM provider name.
    pub llm_provider: String,

    /// LLM model name.
    pub llm_model: String,

    /// Embedding model name.
    pub embedding_model: String,

    /// Whether reranking is available.
    pub reranking_available: bool,

    /// Available rerank providers.
    pub rerank_providers: Vec<String>,

    /// Storage type.
    pub storage_type: String,

    /// Maximum upload size in bytes.
    pub max_upload_size: usize,

    /// Default query config.
    pub default_query_config: DefaultQueryConfig,

    /// Default ingest config.
    pub default_ingest_config: DefaultIngestConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct DefaultQueryConfig {
    pub mode: String,
    pub enable_rerank: bool,
    pub enable_keywords: bool,
    pub enable_degree_ranking: bool,
    pub token_budget: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DefaultIngestConfig {
    pub enable_gleaning: bool,
    pub max_gleaning: usize,
    pub use_llm_summarization: bool,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
}

/// GET /api/config
pub async fn get_config(
    State(state): State<AppState>,
) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        llm_provider: state.config.llm_provider.clone(),
        llm_model: state.config.llm_model.clone(),
        embedding_model: state.config.embedding_model.clone(),
        reranking_available: state.reranker.is_some(),
        rerank_providers: vec!["jina", "cohere", "aliyun"].iter().map(|s| s.to_string()).collect(),
        storage_type: state.config.storage_type.clone(),
        max_upload_size: 50 * 1024 * 1024, // 50MB
        default_query_config: DefaultQueryConfig {
            mode: "adaptive".to_string(),
            enable_rerank: true,
            enable_keywords: true,
            enable_degree_ranking: true,
            token_budget: 4096,
        },
        default_ingest_config: DefaultIngestConfig {
            enable_gleaning: true,
            max_gleaning: 1,
            use_llm_summarization: true,
            chunk_size: 1200,
            chunk_overlap: 100,
        },
    })
}
```

**Register in router:**

```rust
// In routes.rs
.route("/api/config", get(handlers::config::get_config))
```

---

## Task 2.6: Update Handler Implementation

**File: [edgequake/crates/edgequake-api/src/handlers/query.rs](../edgequake/crates/edgequake-api/src/handlers/query.rs)**

```rust
pub async fn query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    let start = Instant::now();

    // Build query config from request
    let query_config = SOTAQueryConfig {
        mode: req.mode,
        enable_rerank: req.enable_rerank,
        min_rerank_score: req.min_rerank_score.unwrap_or(0.3),
        enable_keywords: req.enable_keywords,
        enable_degree_ranking: req.enable_degree_ranking,
        max_entities: req.max_entities.unwrap_or(20),
        max_relationships: req.max_relationships.unwrap_or(50),
        token_budget: req.token_budget.unwrap_or(4096),
    };

    // Create engine with optional reranker
    let reranker = if req.enable_rerank {
        state.get_reranker(req.rerank_model.as_deref())?
    } else {
        None
    };

    let engine = SOTAQueryEngine::new(
        state.graph_storage.clone(),
        state.vector_storage.clone(),
        state.llm_provider.clone(),
        query_config,
    ).with_reranker(reranker);

    // Execute query
    let result = engine.query(&req.query).await?;

    // Build response
    let latency_ms = start.elapsed().as_millis() as u64;

    let sources = if req.include_sources {
        result.context.chunks.iter()
            .take(10)
            .map(|c| SourceChunk {
                doc_id: c.doc_id.clone(),
                content: c.content.chars().take(500).collect(),
                score: c.score,
                rerank_score: c.rerank_score,
            })
            .collect()
    } else {
        vec![]
    };

    Ok(Json(QueryResponse {
        answer: result.answer,
        mode: result.mode_used,
        reranked: req.enable_rerank && reranker.is_some(),
        sources,
        stats: QueryStats {
            latency_ms,
            retrieval_ms: result.stats.retrieval_ms,
            generation_ms: result.stats.generation_ms,
            entities_count: result.stats.entities_count,
            relationships_count: result.stats.relationships_count,
            chunks_count: result.stats.chunks_count,
            context_tokens: result.stats.context_tokens,
            keywords: result.keywords,
        },
    }))
}
```

---

## Tests to Add

```rust
#[tokio::test]
async fn test_query_api_with_reranking() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({
                    "query": "What is OpenAI?",
                    "enable_rerank": true,
                    "mode": "adaptive"
                })).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: QueryResponse = serde_json::from_slice(
        &hyper::body::to_bytes(response.into_body()).await.unwrap()
    ).unwrap();

    assert!(!body.answer.is_empty());
    assert!(body.stats.latency_ms > 0);
}

#[tokio::test]
async fn test_ingest_api_with_gleaning() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({
                    "content": "Sarah Chen founded OpenAI in San Francisco.",
                    "enable_gleaning": true,
                    "max_gleaning": 1
                })).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: IngestResponse = serde_json::from_slice(
        &hyper::body::to_bytes(response.into_body()).await.unwrap()
    ).unwrap();

    assert!(body.success);
    assert!(body.stats.gleaning_iterations > 0);
}

#[tokio::test]
async fn test_config_endpoint() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: ConfigResponse = serde_json::from_slice(
        &hyper::body::to_bytes(response.into_body()).await.unwrap()
    ).unwrap();

    assert!(!body.llm_model.is_empty());
    assert!(body.default_query_config.token_budget > 0);
}
```

---

## Verification Checklist

- [ ] `cargo test --package edgequake-api` passes
- [ ] OpenAPI spec updated (if using)
- [ ] API documentation updated
- [ ] `cargo clippy` clean

---

## Cross-References

- **Previous Phase**: [01-phase-1-wire-features.md](01-phase-1-wire-features.md)
- **Next Phase**: [02-phase-3-ui-integration.md](02-phase-3-ui-integration.md)
- **Current State**: [00-current-state-analysis.md](00-current-state-analysis.md)
