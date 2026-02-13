//! Lineage tracking API handlers (Phase 5).
//!
//! Provides endpoints for querying document lineage, including
//! entity provenance and extraction history.
//!
//! ## Implements
//!
//! - **FEAT0540**: Chunk detail retrieval with source tracking
//! - **FEAT0541**: Entity provenance showing extraction origin
//! - **FEAT0542**: Document lineage with graph relationships
//! - **FEAT0543**: Extraction statistics per document
//!
//! ## Use Cases
//!
//! - **UC2140**: User views chunk detail with source document info
//! - **UC2141**: User traces entity back to source document and line
//! - **UC2142**: User explores document's contribution to knowledge graph
//! - **UC2143**: User reviews extraction quality metrics
//!
//! ## Enforces
//!
//! - **BR0540**: Chunk IDs must be valid UUIDs
//! - **BR0541**: Lineage queries must respect workspace isolation
//! - **BR0542**: Extraction metadata must include version info

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// Re-export DTOs for backward compatibility
pub use crate::handlers::lineage_types::{
    CharRange, ChunkDetailResponse, ChunkLineageResponse, ChunkSourceInfo,
    DescriptionVersionResponse, DocumentGraphLineageResponse, EntityLineageResponse,
    EntityProvenanceResponse, EntitySourceInfo, EntitySummaryResponse, ExtractedEntityInfo,
    ExtractedRelationshipInfo, ExtractionMetadataInfo, ExtractionStatsResponse, LineRangeInfo,
    RelatedEntityInfo, RelationshipSummaryResponse, SourceDocumentInfo,
};

// ============================================================================
// Lineage Response Cache (OODA-23)
// ============================================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// WHY: Lineage data rarely changes after document processing completes.
/// Caching avoids repeated KV lookups for the same document, providing
/// sub-millisecond response times for dashboard and UI polling scenarios.
/// TTL of 120s balances freshness vs. performance (T1: P95 < 200ms).
const LINEAGE_CACHE_TTL: Duration = Duration::from_secs(120);

/// Maximum entries before evicting oldest. Prevents unbounded memory growth.
const LINEAGE_CACHE_MAX_ENTRIES: usize = 500;

#[derive(Clone)]
struct CachedLineage {
    data: serde_json::Value,
    cached_at: Instant,
}

type LineageCache = Arc<RwLock<HashMap<String, CachedLineage>>>;

lazy_static::lazy_static! {
    static ref LINEAGE_KV_CACHE: LineageCache = Arc::new(RwLock::new(HashMap::new()));
}

/// Read from lineage cache or fetch from KV storage.
///
/// WHY: Lineage queries hit KV storage on every request. After a document is
/// processed, the lineage data is immutable until reprocessing. Caching the
/// result avoids redundant I/O and meets the T1 latency target (<200ms P95).
async fn cached_kv_get(
    kv: &dyn edgequake_storage::traits::KVStorage,
    key: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    // Check cache first
    {
        let cache = LINEAGE_KV_CACHE.read().await;
        if let Some(entry) = cache.get(key) {
            if entry.cached_at.elapsed() < LINEAGE_CACHE_TTL {
                return Ok(Some(entry.data.clone()));
            }
        }
    }

    // Cache miss — fetch from storage
    let value = kv.get_by_id(key).await?;

    // Populate cache on hit
    if let Some(ref v) = value {
        let mut cache = LINEAGE_KV_CACHE.write().await;
        // WHY: Evict oldest entries when cache is full to prevent unbounded growth
        if cache.len() >= LINEAGE_CACHE_MAX_ENTRIES {
            // Simple eviction: remove entries older than TTL first
            cache.retain(|_, entry| entry.cached_at.elapsed() < LINEAGE_CACHE_TTL);
            // If still too full, clear half the cache
            if cache.len() >= LINEAGE_CACHE_MAX_ENTRIES {
                let keys_to_remove: Vec<String> = cache
                    .keys()
                    .take(cache.len() / 2)
                    .cloned()
                    .collect();
                for k in keys_to_remove {
                    cache.remove(&k);
                }
            }
        }
        cache.insert(
            key.to_string(),
            CachedLineage {
                data: v.clone(),
                cached_at: Instant::now(),
            },
        );
    }

    Ok(value)
}

/// Invalidate a lineage cache entry.
///
/// WHY: Called after document reprocessing to ensure fresh data is served.
/// Without invalidation, stale lineage data would persist until TTL expires.
#[allow(dead_code)]
pub async fn invalidate_lineage_cache(document_id: &str) {
    let mut cache = LINEAGE_KV_CACHE.write().await;
    let lineage_key = format!("{}-lineage", document_id);
    let metadata_key = format!("{}-metadata", document_id);
    cache.remove(&lineage_key);
    cache.remove(&metadata_key);
    tracing::debug!(
        document_id = %document_id,
        "Invalidated lineage cache entries"
    );
}

// ============================================================================
// Chunk Detail Endpoint (WebUI Spec WEBUI-006)
// ============================================================================

/// Get chunk detail.
#[utoipa::path(
    get,
    path = "/api/v1/chunks/{chunk_id}",
    tag = "Lineage",
    params(
        ("chunk_id" = String, Path, description = "Chunk ID to query")
    ),
    responses(
        (status = 200, description = "Chunk detail", body = ChunkDetailResponse),
        (status = 404, description = "Chunk not found")
    )
)]
pub async fn get_chunk_detail(
    State(state): State<AppState>,
    Path(chunk_id): Path<String>,
) -> ApiResult<Json<ChunkDetailResponse>> {
    // Look up chunk in KV storage
    let chunk_data = state
        .kv_storage
        .get_by_id(&chunk_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Chunk '{}' not found", chunk_id)))?;

    // Parse chunk data
    let content = chunk_data
        .get("content")
        .and_then(|v: &serde_json::Value| v.as_str())
        .unwrap_or("")
        .to_string();

    // OODA-07: Read index field (stored as "index" by OODA-05, fallback to "chunk_index" for legacy)
    let chunk_index = chunk_data
        .get("index")
        .or_else(|| chunk_data.get("chunk_index"))
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(0) as usize;

    let token_count = chunk_data
        .get("token_count")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(0) as usize;

    let start_offset = chunk_data
        .get("start_offset")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(0) as usize;

    let end_offset = chunk_data
        .get("end_offset")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(0) as usize;

    // OODA-07: Read line numbers from chunk KV data (stored by OODA-05)
    let start_line = chunk_data
        .get("start_line")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .map(|v| v as usize);

    let end_line = chunk_data
        .get("end_line")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .map(|v| v as usize);

    // WHY: Chunk IDs follow a deterministic format "{document_id}-chunk-{N}".
    // Extracting the document ID from this format avoids an extra KV lookup
    // and maintains the F8 bidirectional chain (Document ↔ Chunk).
    let document_id = if chunk_id.contains("-chunk-") {
        chunk_id
            .split("-chunk-")
            .next()
            .unwrap_or(&chunk_id)
            .to_string()
    } else {
        chunk_id.clone()
    };

    // Get document name from metadata
    let metadata_key = format!("{}-metadata", document_id);
    let doc_name = if let Ok(Some(metadata)) = state.kv_storage.get_by_id(&metadata_key).await {
        metadata
            .get("title")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    // Find entities extracted from this chunk
    let all_nodes = state.graph_storage.get_all_nodes().await?;
    let mut entities: Vec<ExtractedEntityInfo> = Vec::new();

    for node in &all_nodes {
        if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
            if source_id.contains(&chunk_id) {
                let entity_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let description = node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                entities.push(ExtractedEntityInfo {
                    id: node.id.clone(),
                    name: node.id.clone(),
                    entity_type,
                    description,
                });
            }
        }
    }

    // Find relationships from this chunk
    let all_edges = state.graph_storage.get_all_edges().await?;
    let mut relationships: Vec<ExtractedRelationshipInfo> = Vec::new();

    for edge in all_edges {
        if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
            if source_id.contains(&chunk_id) {
                let relation_type = edge
                    .properties
                    .get("keywords")
                    .and_then(|v| v.as_str())
                    .unwrap_or("related_to")
                    .to_string();
                let description = edge
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                relationships.push(ExtractedRelationshipInfo {
                    source_name: edge.source.clone(),
                    target_name: edge.target.clone(),
                    relation_type,
                    description,
                });
            }
        }
    }

    Ok(Json(ChunkDetailResponse {
        chunk_id,
        document_id,
        document_name: doc_name,
        content,
        index: chunk_index,
        char_range: CharRange {
            start: start_offset,
            end: end_offset,
        },
        start_line,
        end_line,
        token_count,
        entities,
        relationships,
        extraction_metadata: None, // Would need to be stored during extraction
    }))
}

/// Get entity provenance.
#[utoipa::path(
    get,
    path = "/api/v1/entities/{entity_id}/provenance",
    tag = "Lineage",
    params(
        ("entity_id" = String, Path, description = "Entity ID to query")
    ),
    responses(
        (status = 200, description = "Entity provenance", body = EntityProvenanceResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn get_entity_provenance(
    State(state): State<AppState>,
    Path(entity_id): Path<String>,
) -> ApiResult<Json<EntityProvenanceResponse>> {
    // WHY: Entity names are normalized to UPPERCASE_WITH_UNDERSCORES during
    // extraction (see entity_extraction.rs). We must apply the same normalization
    // here so lookups match stored graph nodes regardless of user input casing.
    let normalized_id = entity_id.to_uppercase().replace(' ', "_");

    // Look up entity
    let node = state
        .graph_storage
        .get_node(&normalized_id)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Entity '{}' not found (normalized: '{}'). \
                 Entity names are stored as UPPERCASE_WITH_UNDERSCORES.",
                entity_id, normalized_id
            ))
        })?;

    let entity_type = node
        .properties
        .get("entity_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let description = node
        .properties
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Parse source_id to find all source documents
    let source_id = node
        .properties
        .get("source_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let sources: Vec<String> = source_id.split('|').map(|s| s.to_string()).collect();
    let sources_count = sources.len();
    let mut doc_map: std::collections::HashMap<String, Vec<ChunkSourceInfo>> =
        std::collections::HashMap::new();

    for source in &sources {
        if source.contains("-chunk-") {
            if let Some(pos) = source.find("-chunk-") {
                let doc_id = &source[..pos];
                doc_map
                    .entry(doc_id.to_string())
                    .or_default()
                    .push(ChunkSourceInfo {
                        chunk_id: source.clone(),
                        start_line: None,
                        end_line: None,
                        source_text: None,
                    });
            }
        }
    }

    // OODA-27: Resolve document names and chunk positions from cached KV storage
    // WHY: Without document names, the UI shows UUIDs which are not user-friendly.
    // Using cached_kv_get avoids repeated I/O for documents with many entities.
    let mut entity_sources: Vec<EntitySourceInfo> = Vec::with_capacity(doc_map.len());
    for (doc_id, mut chunks) in doc_map {
        // Resolve document name from metadata
        let metadata_key = format!("{}-metadata", doc_id);
        let doc_name = if let Ok(Some(meta)) =
            cached_kv_get(state.kv_storage.as_ref(), &metadata_key).await
        {
            meta.get("title")
                .or_else(|| meta.get("file_name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        // Resolve chunk line positions from KV storage
        for chunk in &mut chunks {
            if let Ok(Some(chunk_data)) =
                cached_kv_get(state.kv_storage.as_ref(), &chunk.chunk_id).await
            {
                chunk.start_line = chunk_data
                    .get("start_line")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                chunk.end_line = chunk_data
                    .get("end_line")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
            }
        }

        entity_sources.push(EntitySourceInfo {
            document_id: doc_id,
            document_name: doc_name,
            chunks,
            first_extracted_at: None,
        });
    }

    // Find related entities
    let all_edges = state.graph_storage.get_all_edges().await?;
    let mut related: Vec<RelatedEntityInfo> = Vec::new();

    for edge in all_edges {
        if edge.source == normalized_id {
            related.push(RelatedEntityInfo {
                entity_id: edge.target.clone(),
                entity_name: edge.target.clone(),
                relationship_type: edge
                    .properties
                    .get("keywords")
                    .and_then(|v| v.as_str())
                    .unwrap_or("related_to")
                    .to_string(),
                shared_documents: 1,
            });
        } else if edge.target == normalized_id {
            related.push(RelatedEntityInfo {
                entity_id: edge.source.clone(),
                entity_name: edge.source.clone(),
                relationship_type: edge
                    .properties
                    .get("keywords")
                    .and_then(|v| v.as_str())
                    .unwrap_or("related_to")
                    .to_string(),
                shared_documents: 1,
            });
        }
    }

    Ok(Json(EntityProvenanceResponse {
        entity_id: normalized_id.clone(),
        entity_name: normalized_id,
        entity_type,
        description,
        sources: entity_sources,
        total_extraction_count: sources_count,
        related_entities: related,
    }))
}

/// Get lineage for an entity (all source documents).
#[utoipa::path(
    get,
    path = "/api/v1/lineage/entities/{entity_name}",
    tag = "Lineage",
    params(
        ("entity_name" = String, Path, description = "Entity name to query")
    ),
    responses(
        (status = 200, description = "Entity lineage", body = EntityLineageResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn get_entity_lineage(
    State(state): State<AppState>,
    Path(entity_name): Path<String>,
) -> ApiResult<Json<EntityLineageResponse>> {
    // WHY: Same normalization rule as get_entity_provenance — see comment there.
    let normalized_name = entity_name.to_uppercase().replace(' ', "_");

    // Look up entity in graph storage
    let node = state
        .graph_storage
        .get_node(&normalized_name)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Entity '{}' not found (normalized: '{}'). \
                 Entity names are stored as UPPERCASE_WITH_UNDERSCORES.",
                entity_name, normalized_name
            ))
        })?;

    // Parse source_id to extract document and chunk information
    let source_id = node
        .properties
        .get("source_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let sources: Vec<&str> = source_id.split('|').collect();
    let mut source_documents: Vec<SourceDocumentInfo> = Vec::new();
    let mut doc_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for source in sources {
        // Parse source format: "doc_id-chunk-N" or just "doc_id"
        if source.contains("-chunk-") {
            if let Some(pos) = source.find("-chunk-") {
                let doc_id = &source[..pos];
                let chunk_id = source.to_string();
                doc_map
                    .entry(doc_id.to_string())
                    .or_default()
                    .push(chunk_id);
            }
        } else if !source.is_empty() {
            doc_map.entry(source.to_string()).or_default();
        }
    }

    for (doc_id, chunk_ids) in doc_map {
        source_documents.push(SourceDocumentInfo {
            document_id: doc_id,
            chunk_ids,
            line_ranges: vec![], // Line ranges not stored in current implementation
        });
    }

    let entity_type = node
        .properties
        .get("entity_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(Json(EntityLineageResponse {
        entity_name: normalized_name,
        entity_type,
        source_count: source_documents.len(),
        source_documents,
        description_versions: vec![], // Description history not stored in current implementation
    }))
}

/// Get graph lineage for a document.
#[utoipa::path(
    get,
    path = "/api/v1/lineage/documents/{document_id}",
    tag = "Lineage",
    params(
        ("document_id" = String, Path, description = "Document ID to query")
    ),
    responses(
        (status = 200, description = "Document graph lineage", body = DocumentGraphLineageResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn get_document_lineage(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> ApiResult<Json<DocumentGraphLineageResponse>> {
    // WHY: We scan KV keys by prefix rather than querying a separate index.
    // This is correct for in-memory and moderate-scale PostgreSQL KV stores.
    // For very large datasets (>100K documents), consider adding a dedicated
    // chunk-count index to avoid full key scan.
    let keys = state.kv_storage.keys().await?;
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids: Vec<String> = keys
        .iter()
        .filter(|k| k.starts_with(&chunk_prefix))
        .cloned()
        .collect();

    let metadata_key = format!("{}-metadata", document_id);
    if chunk_ids.is_empty() && !keys.contains(&metadata_key) {
        return Err(ApiError::NotFound(format!(
            "Document '{}' not found",
            document_id
        )));
    }

    // Find all entities sourced from this document
    let all_nodes = state.graph_storage.get_all_nodes().await?;
    let mut entities: Vec<EntitySummaryResponse> = Vec::new();

    for node in &all_nodes {
        if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let doc_sources: Vec<String> = sources
                .iter()
                .filter(|s| s.starts_with(&chunk_prefix) || *s == &document_id)
                .map(|s| s.to_string())
                .collect();

            if !doc_sources.is_empty() {
                let entity_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let is_shared = sources.len() > doc_sources.len();

                entities.push(EntitySummaryResponse {
                    name: node.id.clone(),
                    entity_type,
                    source_chunks: doc_sources,
                    is_shared,
                });
            }
        }
    }

    // Find all relationships sourced from this document
    let all_edges = state.graph_storage.get_all_edges().await?;
    let mut relationships: Vec<RelationshipSummaryResponse> = Vec::new();

    for edge in all_edges {
        if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let doc_sources: Vec<String> = sources
                .iter()
                .filter(|s| s.starts_with(&chunk_prefix) || *s == &document_id)
                .map(|s| s.to_string())
                .collect();

            if !doc_sources.is_empty() {
                let keywords = edge
                    .properties
                    .get("keywords")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                relationships.push(RelationshipSummaryResponse {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                    keywords,
                    source_chunks: doc_sources,
                });
            }
        }
    }

    Ok(Json(DocumentGraphLineageResponse {
        document_id,
        chunk_count: chunk_ids.len(),
        extraction_stats: ExtractionStatsResponse {
            total_entities: entities.len(),
            unique_entities: entities.len(),
            total_relationships: relationships.len(),
            unique_relationships: relationships.len(),
            processing_time_ms: None,
        },
        entities,
        relationships,
    }))
}

// ============================================================================
// Chunk Lineage Endpoint (OODA-08)
// ============================================================================

/// Get chunk lineage with parent document refs and extracted entities.
///
/// OODA-08: Returns a chunk's complete lineage chain — parent document info,
/// position data, and entity/relationship summary — in a single API call.
///
/// @implements F3: Every chunk contains parent_document_id and complete position info
/// @implements F8: PDF → Document → Chunk → Entity chain is traceable
#[utoipa::path(
    get,
    path = "/api/v1/chunks/{chunk_id}/lineage",
    tag = "Lineage",
    params(
        ("chunk_id" = String, Path, description = "Chunk ID to query lineage for")
    ),
    responses(
        (status = 200, description = "Chunk lineage with parent refs", body = ChunkLineageResponse),
        (status = 404, description = "Chunk not found")
    )
)]
pub async fn get_chunk_lineage(
    State(state): State<AppState>,
    Path(chunk_id): Path<String>,
) -> ApiResult<Json<ChunkLineageResponse>> {
    // Look up chunk in KV storage
    let chunk_data = state
        .kv_storage
        .get_by_id(&chunk_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Chunk '{}' not found", chunk_id)))?;

    // Parse chunk fields
    let content = chunk_data
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // WHY: Truncate content to 200 chars for the preview field. The full content
    // is available via the chunk detail endpoint. This keeps lineage responses
    // compact for dashboard/tree views where only a preview is needed.
    let content_preview = if content.len() > 200 {
        format!("{}...", &content[..200])
    } else {
        content.to_string()
    };

    let index = chunk_data
        .get("index")
        .or_else(|| chunk_data.get("chunk_index"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let token_count = chunk_data
        .get("token_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let start_line = chunk_data
        .get("start_line")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let end_line = chunk_data
        .get("end_line")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let start_offset = chunk_data
        .get("start_offset")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let end_offset = chunk_data
        .get("end_offset")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    // Extract document ID from chunk ID (format: doc_id-chunk-N)
    let document_id = if chunk_id.contains("-chunk-") {
        chunk_id
            .split("-chunk-")
            .next()
            .unwrap_or(&chunk_id)
            .to_string()
    } else {
        chunk_data
            .get("document_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&chunk_id)
            .to_string()
    };

    // OODA-23: Use cached KV lookup for metadata
    let metadata_key = format!("{}-metadata", document_id);
    let doc_metadata = cached_kv_get(state.kv_storage.as_ref(), &metadata_key)
        .await?
        .unwrap_or(serde_json::json!({"id": document_id}));

    let document_name = doc_metadata
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let document_type = doc_metadata
        .get("document_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Count entities and relationships from this chunk
    let all_nodes = state.graph_storage.get_all_nodes().await?;
    let mut entity_names: Vec<String> = Vec::new();

    for node in &all_nodes {
        if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
            if source_id.contains(&chunk_id) {
                entity_names.push(node.id.clone());
            }
        }
    }

    let all_edges = state.graph_storage.get_all_edges().await?;
    let mut relationship_count = 0usize;
    for edge in &all_edges {
        if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
            if source_id.contains(&chunk_id) {
                relationship_count += 1;
            }
        }
    }

    let entity_count = entity_names.len();

    Ok(Json(ChunkLineageResponse {
        chunk_id,
        document_id,
        document_name,
        document_type,
        index,
        start_line,
        end_line,
        start_offset,
        end_offset,
        token_count,
        content_preview,
        entity_count,
        relationship_count,
        entity_names,
        document_metadata: Some(doc_metadata),
    }))
}

// ============================================================================
// Document Full Lineage Endpoint (OODA-07)
// ============================================================================

/// Get complete document lineage from persisted KV storage.
///
/// OODA-07: Returns the full DocumentLineage tree (chunks, entities, relationships)
/// persisted by OODA-06 after pipeline processing. This is a single-call endpoint
/// that returns everything needed for lineage visualization.
///
/// @implements F5: Single API call retrieves complete document lineage tree
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/lineage",
    tag = "Lineage",
    params(
        ("document_id" = String, Path, description = "Document ID to query lineage for")
    ),
    responses(
        (status = 200, description = "Complete document lineage tree"),
        (status = 404, description = "Document or lineage not found")
    )
)]
pub async fn get_document_full_lineage(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // OODA-23: Use cached KV lookup for sub-millisecond cache hits
    let lineage_key = format!("{}-lineage", document_id);
    let lineage_data = cached_kv_get(state.kv_storage.as_ref(), &lineage_key)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Lineage for document '{}' not found. Document may not have been processed yet.",
                document_id
            ))
        })?;

    // WHY: Combine lineage tree + document metadata in one response so the UI
    // can render both the hierarchy and document context without a second API call.
    // This satisfies F5: "Single API call retrieves complete document lineage tree."
    let metadata_key = format!("{}-metadata", document_id);
    let metadata = cached_kv_get(state.kv_storage.as_ref(), &metadata_key)
        .await?
        .unwrap_or(serde_json::json!({"id": document_id, "status": "unknown"}));
    Ok(Json(serde_json::json!({
        "document_id": document_id,
        "metadata": metadata,
        "lineage": lineage_data,
    })))
}

/// Get document metadata (all fields in a single response).
///
/// OODA-07: Returns all document metadata fields stored in KV storage.
///
/// @implements F1: All document metadata is stored and retrievable
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/metadata",
    tag = "Lineage",
    params(
        ("document_id" = String, Path, description = "Document ID to query metadata for")
    ),
    responses(
        (status = 200, description = "Document metadata"),
        (status = 404, description = "Document not found")
    )
)]
pub async fn get_document_metadata(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // OODA-23: Use cached KV lookup for metadata
    let metadata_key = format!("{}-metadata", document_id);
    let metadata = cached_kv_get(state.kv_storage.as_ref(), &metadata_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Document '{}' not found", document_id)))?;

    Ok(Json(metadata))
}

// ============================================================================
// Lineage Export Endpoint (OODA-22)
// ============================================================================

/// Query parameters for lineage export.
#[derive(Debug, serde::Deserialize, serde::Serialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct ExportParams {
    /// Export format: "json" (default) or "csv".
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".to_string()
}

/// Export complete document lineage as JSON or CSV file.
///
/// OODA-22: Returns lineage data as a downloadable file with proper
/// Content-Disposition headers. CSV format flattens the hierarchical
/// lineage into a table with one row per chunk.
///
/// @implements F5: Single API call retrieves complete document lineage tree
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/lineage/export",
    tag = "Lineage",
    params(
        ("document_id" = String, Path, description = "Document ID to export lineage for"),
        ExportParams,
    ),
    responses(
        (status = 200, description = "Lineage export file (JSON or CSV)"),
        (status = 404, description = "Document or lineage not found")
    )
)]
pub async fn export_document_lineage(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, ApiError> {
    // OODA-23: Use cached KV lookup for export
    let lineage_key = format!("{}-lineage", document_id);
    let lineage_data = cached_kv_get(state.kv_storage.as_ref(), &lineage_key)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Lineage for document '{}' not found. \
                 Document may not have been processed yet.",
                document_id
            ))
        })?;

    // Read metadata for context (cached)
    let metadata_key = format!("{}-metadata", document_id);
    let metadata = cached_kv_get(state.kv_storage.as_ref(), &metadata_key)
        .await?
        .unwrap_or(serde_json::json!({"id": document_id}));

    let combined = serde_json::json!({
        "document_id": document_id,
        "metadata": metadata,
        "lineage": lineage_data,
    });

    match params.format.as_str() {
        "csv" => {
            // WHY: CSV flattens hierarchical lineage into a chunk-per-row table.
            // This is useful for spreadsheet analysis and data pipeline ingestion.
            let csv_content = lineage_to_csv(&document_id, &lineage_data);
            let filename = format!("{}-lineage.csv", document_id);
            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                csv_content,
            ))
        }
        _ => {
            // Default: JSON export
            let json_content =
                serde_json::to_string_pretty(&combined).unwrap_or_else(|_| "{}".to_string());
            let filename = format!("{}-lineage.json", document_id);
            Ok((
                StatusCode::OK,
                [
                    (
                        header::CONTENT_TYPE,
                        "application/json; charset=utf-8".to_string(),
                    ),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                json_content,
            ))
        }
    }
}

/// Convert lineage data to CSV format.
///
/// WHY: Flattens the document → chunks hierarchy into a tabular format
/// with one row per chunk, suitable for spreadsheets and data analysis.
fn lineage_to_csv(document_id: &str, lineage: &serde_json::Value) -> String {
    let mut csv = String::new();
    csv.push_str(
        "document_id,chunk_index,content_preview,tokens,start_line,end_line,entity_count\n",
    );

    if let Some(chunks) = lineage.get("chunks").and_then(|c| c.as_array()) {
        for chunk in chunks {
            let index = chunk
                .get("chunk_index")
                .or_else(|| chunk.get("index"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let content = chunk
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let preview = if content.len() > 100 {
                &content[..100]
            } else {
                content
            };
            // WHY: Escape CSV fields — wrap in quotes and double any internal quotes
            let escaped_preview = preview.replace('"', "\"\"").replace('\n', " ");
            let tokens = chunk.get("tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let start_line = chunk
                .get("start_line")
                .and_then(|v| v.as_u64())
                .map(|v| v.to_string())
                .unwrap_or_default();
            let end_line = chunk
                .get("end_line")
                .and_then(|v| v.as_u64())
                .map(|v| v.to_string())
                .unwrap_or_default();
            let entity_count = chunk
                .get("entity_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            csv.push_str(&format!(
                "{},{},\"{}\",{},{},{},{}\n",
                document_id, index, escaped_preview, tokens, start_line, end_line, entity_count
            ));
        }
    }

    csv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_lineage_response_serialization() {
        let response = EntityLineageResponse {
            entity_name: "JOHN_DOE".to_string(),
            entity_type: Some("person".to_string()),
            source_documents: vec![SourceDocumentInfo {
                document_id: "doc-123".to_string(),
                chunk_ids: vec!["doc-123-chunk-0".to_string()],
                line_ranges: vec![LineRangeInfo {
                    start_line: 1,
                    end_line: 10,
                }],
            }],
            source_count: 1,
            description_versions: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("JOHN_DOE"));
        assert!(json.contains("doc-123"));
    }

    #[test]
    fn test_document_graph_lineage_response_serialization() {
        let response = DocumentGraphLineageResponse {
            document_id: "doc-123".to_string(),
            chunk_count: 5,
            entities: vec![EntitySummaryResponse {
                name: "JOHN_DOE".to_string(),
                entity_type: "person".to_string(),
                source_chunks: vec!["doc-123-chunk-0".to_string()],
                is_shared: false,
            }],
            relationships: vec![RelationshipSummaryResponse {
                source: "JOHN_DOE".to_string(),
                target: "ACME_CORP".to_string(),
                keywords: "works_at".to_string(),
                source_chunks: vec!["doc-123-chunk-0".to_string()],
            }],
            extraction_stats: ExtractionStatsResponse {
                total_entities: 1,
                unique_entities: 1,
                total_relationships: 1,
                unique_relationships: 1,
                processing_time_ms: Some(500),
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-123"));
        assert!(json.contains("JOHN_DOE"));
        assert!(json.contains("works_at"));
    }

    #[test]
    fn test_line_range_info_serialization() {
        let info = LineRangeInfo {
            start_line: 10,
            end_line: 20,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"start_line\":10"));
        assert!(json.contains("\"end_line\":20"));
    }

    #[test]
    fn test_extraction_stats_response_serialization() {
        let stats = ExtractionStatsResponse {
            total_entities: 100,
            unique_entities: 50,
            total_relationships: 200,
            unique_relationships: 80,
            processing_time_ms: None,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_entities\":100"));
        assert!(json.contains("\"unique_entities\":50"));
    }

    #[test]
    fn test_description_version_response() {
        let version = DescriptionVersionResponse {
            version: 1,
            description: "Initial description".to_string(),
            source_chunk_id: Some("chunk-123".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&version).unwrap();
        assert!(json.contains("\"version\":1"));
        assert!(json.contains("Initial description"));
    }

    // OODA-22: Export tests
    #[test]
    fn test_lineage_to_csv_basic() {
        let lineage = serde_json::json!({
            "chunks": [
                {
                    "chunk_index": 0,
                    "content": "Hello world",
                    "tokens": 2,
                    "start_line": 1,
                    "end_line": 5,
                    "entity_count": 3
                },
                {
                    "chunk_index": 1,
                    "content": "Second chunk",
                    "tokens": 4,
                    "start_line": 6,
                    "end_line": 10,
                    "entity_count": 1
                }
            ]
        });
        let csv = lineage_to_csv("doc-001", &lineage);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 rows
        assert!(lines[0].starts_with("document_id,chunk_index"));
        assert!(lines[1].contains("doc-001"));
        assert!(lines[1].contains("Hello world"));
        assert!(lines[2].contains("Second chunk"));
    }

    #[test]
    fn test_lineage_to_csv_empty_chunks() {
        let lineage = serde_json::json!({ "chunks": [] });
        let csv = lineage_to_csv("doc-empty", &lineage);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1); // header only
    }

    #[test]
    fn test_lineage_to_csv_no_chunks_key() {
        let lineage = serde_json::json!({ "metadata": {} });
        let csv = lineage_to_csv("doc-no-chunks", &lineage);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1); // header only
    }

    #[test]
    fn test_lineage_to_csv_escapes_quotes() {
        let lineage = serde_json::json!({
            "chunks": [{
                "chunk_index": 0,
                "content": "He said \"hello\" to her",
                "tokens": 5,
                "entity_count": 0
            }]
        });
        let csv = lineage_to_csv("doc-esc", &lineage);
        // Escaped quotes should be doubled inside CSV field
        assert!(csv.contains("\"\"hello\"\""));
    }

    #[test]
    fn test_export_params_default_format() {
        let params: ExportParams = serde_json::from_str("{}").unwrap();
        assert_eq!(params.format, "json");
    }

    #[test]
    fn test_export_params_csv_format() {
        let params: ExportParams = serde_json::from_str(r#"{"format":"csv"}"#).unwrap();
        assert_eq!(params.format, "csv");
    }

    // OODA-23: Cache configuration tests
    #[test]
    fn test_lineage_cache_ttl_is_reasonable() {
        // WHY: TTL must be long enough to absorb polling but short enough for freshness
        assert!(LINEAGE_CACHE_TTL.as_secs() >= 30, "TTL too short for dashboard polling");
        assert!(LINEAGE_CACHE_TTL.as_secs() <= 300, "TTL too long for freshness");
    }

    #[test]
    fn test_lineage_cache_max_entries_bounded() {
        // WHY: Unbounded cache = memory leak in production
        assert!(LINEAGE_CACHE_MAX_ENTRIES > 0);
        assert!(LINEAGE_CACHE_MAX_ENTRIES <= 10_000, "Cache too large");
    }

    #[tokio::test]
    async fn test_invalidate_lineage_cache() {
        // Populate cache directly
        {
            let mut cache = LINEAGE_KV_CACHE.write().await;
            cache.insert(
                "test-doc-lineage".to_string(),
                CachedLineage {
                    data: serde_json::json!({"test": true}),
                    cached_at: Instant::now(),
                },
            );
            cache.insert(
                "test-doc-metadata".to_string(),
                CachedLineage {
                    data: serde_json::json!({"meta": true}),
                    cached_at: Instant::now(),
                },
            );
        }

        // Verify entries exist
        {
            let cache = LINEAGE_KV_CACHE.read().await;
            assert!(cache.contains_key("test-doc-lineage"));
            assert!(cache.contains_key("test-doc-metadata"));
        }

        // Invalidate
        invalidate_lineage_cache("test-doc").await;

        // Verify entries removed
        {
            let cache = LINEAGE_KV_CACHE.read().await;
            assert!(!cache.contains_key("test-doc-lineage"));
            assert!(!cache.contains_key("test-doc-metadata"));
        }
    }
}
