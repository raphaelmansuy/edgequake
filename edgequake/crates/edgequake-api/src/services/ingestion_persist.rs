//! SPEC-022 P-H1 — single ingestion persistence port for HTTP + worker paths (DIP).
//!
//! All callers delegate here after `Pipeline::process*` so chunk vectors, graph
//! merge, and saga compensation cannot diverge.

use std::sync::Arc;

use edgequake_llm::traits::LLMProvider;
use edgequake_pipeline::{
    ChunkVectorBuildOptions, DefaultIngestionPersister, IngestionPersistContext,
    IngestionPersistOutput, IngestionPersistSettings, IngestionPersister, LineageSink,
    MergeProgressCallback, NoopEntitySink, NoopLineageSink, ProcessingResult, RelationalEntitySink,
};
use edgequake_query::QueryResultCacheInvalidator;
use edgequake_storage::traits::domain::ChunkRepository;
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};

#[cfg(feature = "postgres")]
use crate::postgres_entity_sink::PostgresEntitySink;
use crate::state::AppState;

/// Parameters for the shared persist path.
pub struct PersistIngestionParams<'a> {
    pub document_id: &'a str,
    pub tenant_id: Option<String>,
    pub workspace_id: String,
    pub result: &'a ProcessingResult,
    pub chunk_options: ChunkVectorBuildOptions,
    /// Optional chunk vector metadata (e.g. `"injection"` for SPEC-0002).
    pub source_type: Option<&'a str>,
    pub source_file_path: Option<&'a str>,
}

impl<'a> PersistIngestionParams<'a> {
    /// Standard document upload/worker params (no source overlay).
    pub fn for_document(
        document_id: &'a str,
        tenant_id: Option<String>,
        workspace_id: String,
        result: &'a ProcessingResult,
        chunk_options: ChunkVectorBuildOptions,
        source_file: Option<&'a str>,
    ) -> Self {
        Self {
            document_id,
            tenant_id,
            workspace_id,
            result,
            chunk_options,
            source_type: None,
            source_file_path: source_file,
        }
    }
}

/// Tag extraction records for knowledge injection (SPEC-0002 / SPEC-023 I1).
pub fn tag_injection_sources(result: &mut ProcessingResult, doc_id: &str) {
    for extraction in &mut result.extractions {
        for entity in &mut extraction.entities {
            entity.source_document_id = Some(doc_id.to_string());
            entity.source_file_path = Some("injection".to_string());
            if entity.source_chunk_ids.is_empty() {
                entity.source_chunk_ids = vec![format!("{doc_id}-chunk-0")];
            }
        }
        for rel in &mut extraction.relationships {
            rel.source_document_id = Some(doc_id.to_string());
            rel.source_file_path = Some("injection".to_string());
            if rel.source_chunk_id.is_none() {
                rel.source_chunk_id = Some(format!("{doc_id}-chunk-0"));
            }
        }
    }
}

/// Resolve the relational entity sink (CQRS dual-write when enabled).
///
/// Under typed embeddings, always returns a fail-closed Postgres sink so fleet
/// mirror has relational FKs even when `entity_sync_mode=disabled`.
pub async fn resolve_relational_sink(state: &AppState) -> Arc<dyn RelationalEntitySink> {
    #[cfg(feature = "postgres")]
    if let Some(ref pool) = state.pg_pool {
        return PostgresEntitySink::create_for_runtime(Arc::new(pool.clone())).await;
    }
    let _ = state;
    Arc::new(NoopEntitySink)
}

/// SPEC-091 W1: relational chunk writer when Postgres pool is available.
/// Authority flag (`EDGEQUAKE_CHUNK_TEXT_AUTHORITY`) still gates whether it is used.
#[cfg(feature = "postgres")]
pub fn resolve_relational_chunk_repo(
    pool: crate::services::OptionalPgPool<'_>,
) -> Option<Arc<dyn ChunkRepository>> {
    pool.map(|pool| {
        Arc::new(edgequake_storage::PostgresChunkRepository::new(
            pool.clone(),
        )) as Arc<dyn ChunkRepository>
    })
}

/// SPEC-091 W3: typed chunk embedding index when Postgres pool is available.
/// Under `typed_embeddings`, this is write SSOT for chunk vectors (legacy
/// `eq_*_vectors` upserts are write-stopped at the adapter).
#[cfg(feature = "postgres")]
pub fn resolve_typed_embedding_index(
    pool: Option<sqlx::PgPool>,
) -> Option<Arc<dyn edgequake_storage::traits::domain::EmbeddingIndex>> {
    pool.map(|pool| {
        let model = std::env::var("EDGEQUAKE_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());
        Arc::new(edgequake_storage::PgChunkEmbeddingIndex::new(pool, model))
            as Arc<dyn edgequake_storage::traits::domain::EmbeddingIndex>
    })
}

/// SPEC-091 IW2: typed fleet embedding index (entity/relationship/report).
#[cfg(feature = "postgres")]
pub fn resolve_fleet_embedding_index(
    pool: Option<sqlx::PgPool>,
) -> Option<Arc<dyn edgequake_storage::traits::FleetEmbeddingIndex>> {
    pool.map(|pool| {
        let model = std::env::var("EDGEQUAKE_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());
        Arc::new(edgequake_storage::PgFleetEmbeddingIndex::new(pool, model))
            as Arc<dyn edgequake_storage::traits::FleetEmbeddingIndex>
    })
}

/// Non-postgres builds have no relational spine; the authority flag defaults
/// to `kv`, so this is never reached in practice.
#[cfg(not(feature = "postgres"))]
pub fn resolve_relational_chunk_repo(_pool: Option<&()>) -> Option<Arc<dyn ChunkRepository>> {
    None
}

#[cfg(feature = "postgres")]
fn relational_chunk_pool(state: &AppState) -> crate::services::OptionalPgPool<'_> {
    state.optional_pg_pool()
}

#[cfg(not(feature = "postgres"))]
fn relational_chunk_pool(_state: &AppState) -> Option<&'static ()> {
    None
}

/// Persist pipeline output via `IngestionPersister` and invalidate query result cache.
pub async fn persist_ingestion_result(
    state: &AppState,
    graph_storage: Arc<dyn GraphStorage>,
    vector_storage: Arc<dyn VectorStorage>,
    relational_sink: Arc<dyn RelationalEntitySink>,
    params: PersistIngestionParams<'_>,
) -> Result<IngestionPersistOutput, edgequake_pipeline::error::PipelineError> {
    persist_with_providers_progress_and_embedder(
        Arc::clone(&state.query.llm_provider),
        Some(state.query.engine_impl.as_ref() as &dyn QueryResultCacheInvalidator),
        graph_storage,
        vector_storage,
        Arc::clone(&state.storage.kv_storage),
        relational_sink,
        Arc::new(NoopLineageSink),
        None,
        resolve_relational_chunk_repo(relational_chunk_pool(state)),
        #[cfg(feature = "postgres")]
        state.ingestion_committer.clone(),
        #[cfg(feature = "postgres")]
        state.document_reader.clone(),
        #[cfg(feature = "postgres")]
        state.pg_pool.clone(),
        params,
        None,
    )
    .await
}

/// Same as [`persist_ingestion_result`] but accepts explicit LLM + cache invalidator (worker processor).
///
/// Does **not** attach a durable committer — prefer
/// [`persist_with_providers_progress_and_embedder`] for product serving.
pub async fn persist_with_providers(
    llm_provider: Arc<dyn LLMProvider>,
    cache_invalidator: Option<&dyn QueryResultCacheInvalidator>,
    graph_storage: Arc<dyn GraphStorage>,
    vector_storage: Arc<dyn VectorStorage>,
    kv_storage: Arc<dyn KVStorage>,
    relational_sink: Arc<dyn RelationalEntitySink>,
    params: PersistIngestionParams<'_>,
) -> Result<IngestionPersistOutput, edgequake_pipeline::error::PipelineError> {
    persist_with_providers_progress_and_embedder(
        llm_provider,
        cache_invalidator,
        graph_storage,
        vector_storage,
        kv_storage,
        relational_sink,
        Arc::new(NoopLineageSink),
        None,
        None,
        #[cfg(feature = "postgres")]
        None,
        #[cfg(feature = "postgres")]
        None,
        #[cfg(feature = "postgres")]
        None,
        params,
        None,
    )
    .await
}

/// Full variant: accepts an optional merge progress callback and lineage sink (SPEC-032 W-04/W-08).
///
/// Does **not** attach a durable committer — prefer
/// [`persist_with_providers_progress_and_embedder`] for product serving.
#[allow(clippy::too_many_arguments)]
pub async fn persist_with_providers_and_progress(
    llm_provider: Arc<dyn LLMProvider>,
    cache_invalidator: Option<&dyn QueryResultCacheInvalidator>,
    graph_storage: Arc<dyn GraphStorage>,
    vector_storage: Arc<dyn VectorStorage>,
    kv_storage: Arc<dyn KVStorage>,
    relational_sink: Arc<dyn RelationalEntitySink>,
    lineage_sink: Arc<dyn LineageSink>,
    params: PersistIngestionParams<'_>,
    merge_progress: Option<MergeProgressCallback>,
) -> Result<IngestionPersistOutput, edgequake_pipeline::error::PipelineError> {
    persist_with_providers_progress_and_embedder(
        llm_provider,
        cache_invalidator,
        graph_storage,
        vector_storage,
        kv_storage,
        relational_sink,
        lineage_sink,
        None,
        None,
        #[cfg(feature = "postgres")]
        None,
        #[cfg(feature = "postgres")]
        None,
        #[cfg(feature = "postgres")]
        None,
        params,
        merge_progress,
    )
    .await
}

/// Persist with optional text embedder for community_report vectors (SPEC-046).
///
/// SPEC-091: pass `relational_chunks` (from [`resolve_relational_chunk_repo`]) so
/// `EDGEQUAKE_CHUNK_TEXT_AUTHORITY=dual|relational` can write the spine.
#[allow(clippy::too_many_arguments)]
pub async fn persist_with_providers_progress_and_embedder(
    llm_provider: Arc<dyn LLMProvider>,
    cache_invalidator: Option<&dyn QueryResultCacheInvalidator>,
    graph_storage: Arc<dyn GraphStorage>,
    vector_storage: Arc<dyn VectorStorage>,
    kv_storage: Arc<dyn KVStorage>,
    relational_sink: Arc<dyn RelationalEntitySink>,
    lineage_sink: Arc<dyn LineageSink>,
    text_embedder: Option<Arc<dyn edgequake_storage::TextEmbedder>>,
    relational_chunks: Option<Arc<dyn ChunkRepository>>,
    #[cfg(feature = "postgres")] ingestion_committer: Option<
        Arc<dyn edgequake_storage::contracts::IngestionCommitter>,
    >,
    #[cfg(feature = "postgres")] document_reader: Option<
        Arc<dyn edgequake_storage::contracts::DocumentReader>,
    >,
    #[cfg(feature = "postgres")] typed_embedding_pool: Option<sqlx::PgPool>,
    params: PersistIngestionParams<'_>,
    merge_progress: Option<MergeProgressCallback>,
) -> Result<IngestionPersistOutput, edgequake_pipeline::error::PipelineError> {
    let workspace_id = params.workspace_id.clone();
    #[cfg(feature = "postgres")]
    let require_authority = ingestion_committer.is_some();
    #[cfg(not(feature = "postgres"))]
    let require_authority = false;
    let ingest_generation = allocate_ingest_generation(
        #[cfg(feature = "postgres")]
        document_reader.as_deref(),
        #[cfg(not(feature = "postgres"))]
        None,
        require_authority,
        params.tenant_id.as_deref(),
        &workspace_id,
        params.document_id,
    )
    .await?;
    let ctx = IngestionPersistContext::new(
        params.document_id,
        params.tenant_id,
        Some(workspace_id.clone()),
    )
    .with_source_metadata(
        params.source_type.map(str::to_string),
        params.source_file_path.map(str::to_string),
    )
    // SPEC-149: generation = authority document revision + 1 (1 on first ingest).
    .with_ingest_generation(ingest_generation);

    let mut persister = DefaultIngestionPersister::from_settings(
        graph_storage,
        vector_storage,
        IngestionPersistSettings::default(),
        relational_sink,
        Some(llm_provider),
        Some(kv_storage),
    )
    .with_lineage_sink(lineage_sink);

    let chunks_for_authority = relational_chunks.clone();
    persister = persister.with_relational_chunks(relational_chunks);

    // SPEC-091 W3/IW2: typed chunk + fleet embedding writes. Under
    // `typed_embeddings` the legacy `eq_*_vectors` adapter is write-stopped;
    // these hooks are the SSOT (fail-closed when typed).
    #[cfg(feature = "postgres")]
    {
        match (ingestion_committer, chunks_for_authority) {
            (Some(committer), Some(repo)) => {
                let embedding_model_id = std::env::var("EDGEQUAKE_EMBEDDING_MODEL")
                    .unwrap_or_else(|_| "text-embedding-3-small".into());
                persister = persister.with_ingestion_authority(
                    edgequake_pipeline::IngestionAuthority::DurableCommitter {
                        committer,
                        relational_chunks: repo,
                        embedding_model_id,
                    },
                );
            }
            (Some(_), None) => {
                return Err(edgequake_pipeline::error::PipelineError::StorageError(
                    edgequake_storage::StorageError::InvalidData(
                        "durable ingest requires relational_chunks with ingestion_committer".into(),
                    ),
                ));
            }
            (None, _) => {}
        }
        let typed_index = resolve_typed_embedding_index(typed_embedding_pool.clone());
        persister = persister.with_typed_embedding_index(typed_index);
        let fleet_index = resolve_fleet_embedding_index(typed_embedding_pool.clone());
        persister = persister.with_fleet_embedding_index(fleet_index);
        // SPEC-091 IP2: transactional outbox (LAW-D3) — same pool as typed writers.
        if let Some(pool) = typed_embedding_pool {
            let outbox: Arc<dyn edgequake_storage::OutboxSink> =
                Arc::new(edgequake_storage::PostgresOutboxSink::new(pool));
            persister = persister.with_outbox(Some(outbox));
        }
    }

    if let Some(embedder) = text_embedder {
        persister = persister.with_text_embedder(embedder);
    }

    if let Some(cb) = merge_progress {
        persister = persister.with_merge_progress(cb);
    }

    let out = persister
        .persist(&ctx, params.result, params.chunk_options)
        .await?;

    if let Some(invalidator) = cache_invalidator {
        invalidator.invalidate_query_result_cache_for_workspace(&workspace_id);
    }

    Ok(out)
}

/// Allocate the next ingest generation from P0 relational authority state.
///
/// Returns `current_document_revision + 1`, or `1` when authority is unavailable
/// (memory path / no committer). When `require_authority` is true (committer
/// wired), missing reader or unparseable scope ids fail closed — never collide
/// `{doc}:1:0` after a real commit.
pub(crate) async fn allocate_ingest_generation(
    reader: Option<&dyn edgequake_storage::contracts::DocumentReader>,
    require_authority: bool,
    tenant_id: Option<&str>,
    workspace_id: &str,
    document_id: &str,
) -> Result<u64, edgequake_pipeline::error::PipelineError> {
    if !require_authority {
        return Ok(1);
    }
    let reader = reader.ok_or_else(|| {
        edgequake_pipeline::error::PipelineError::StorageError(
            edgequake_storage::StorageError::InvalidData(
                "durable ingest requires document_reader".into(),
            ),
        )
    })?;
    let tenant_raw = tenant_id.ok_or_else(|| {
        edgequake_pipeline::error::PipelineError::StorageError(
            edgequake_storage::StorageError::InvalidData(
                "durable ingest requires tenant_id".into(),
            ),
        )
    })?;
    let tenant_uuid = uuid::Uuid::parse_str(tenant_raw).map_err(|error| {
        edgequake_pipeline::error::PipelineError::StorageError(
            edgequake_storage::StorageError::InvalidData(format!(
                "invalid tenant_id for durable ingest: {error}"
            )),
        )
    })?;
    let workspace_uuid = uuid::Uuid::parse_str(workspace_id).map_err(|error| {
        edgequake_pipeline::error::PipelineError::StorageError(
            edgequake_storage::StorageError::InvalidData(format!(
                "invalid workspace_id for durable ingest: {error}"
            )),
        )
    })?;
    let document_uuid =
        edgequake_pipeline::persistence::resolve_relational_document_id(document_id)
            .map(|id| id.into_uuid())
            .map_err(|error| {
                edgequake_pipeline::error::PipelineError::StorageError(
                    edgequake_storage::StorageError::InvalidData(format!(
                        "invalid document_id for durable ingest: {error}"
                    )),
                )
            })?;
    let scope = edgequake_storage::contracts::AccessScope::new(
        edgequake_storage::contracts::TenantId::new(tenant_uuid),
        edgequake_storage::contracts::WorkspaceId::new(workspace_uuid),
    );
    let views = reader
        .get_many(
            &scope,
            &[edgequake_storage::contracts::DocumentId::new(document_uuid)],
        )
        .await
        .map_err(edgequake_storage::StorageError::from)
        .map_err(edgequake_pipeline::error::PipelineError::StorageError)?;
    let Some(view) = views.into_iter().next().flatten() else {
        return Ok(1);
    };
    if view.deleted {
        return Err(edgequake_pipeline::error::PipelineError::StorageError(
            edgequake_storage::StorageError::Conflict(
                "cannot ingest into a tombstoned document".into(),
            ),
        ));
    }
    view.revision.checked_add(1).ok_or_else(|| {
        edgequake_pipeline::error::PipelineError::StorageError(
            edgequake_storage::StorageError::InvalidData("ingest generation overflow".into()),
        )
    })
}

/// Build KV chunk records for a processed document (outside persister scope).
pub fn build_chunk_kv_records(
    document_id: &str,
    filename: &str,
    result: &ProcessingResult,
) -> Vec<(String, serde_json::Value)> {
    edgequake_pipeline::build_chunk_kv_records(document_id, Some(filename), result)
}

#[cfg(test)]
mod tests {
    use super::allocate_ingest_generation;
    use async_trait::async_trait;
    use edgequake_storage::contracts::{
        AccessResult, AccessScope, CursorPage, DocumentId, DocumentPageRequest, DocumentReader,
        DocumentView,
    };

    struct StubReader;

    #[async_trait]
    impl DocumentReader for StubReader {
        async fn get_many(
            &self,
            _scope: &AccessScope,
            ids: &[DocumentId],
        ) -> AccessResult<Vec<Option<DocumentView>>> {
            Ok(ids.iter().map(|_| None).collect())
        }

        async fn list(
            &self,
            _scope: &AccessScope,
            _request: &DocumentPageRequest,
        ) -> AccessResult<CursorPage<DocumentView>> {
            Ok(CursorPage {
                items: Vec::new(),
                next_cursor: None,
            })
        }
    }

    #[tokio::test]
    async fn memory_path_without_authority_uses_generation_one() {
        let generation = allocate_ingest_generation(None, false, None, "ws", "doc")
            .await
            .unwrap();
        assert_eq!(generation, 1);
    }

    #[tokio::test]
    async fn durable_path_without_reader_fails_closed() {
        let err = allocate_ingest_generation(None, true, Some("t"), "ws", "doc")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("document_reader"));
    }

    #[tokio::test]
    async fn durable_path_without_tenant_fails_closed() {
        let reader: &dyn DocumentReader = &StubReader;
        let err = allocate_ingest_generation(Some(reader), true, None, "ws", "doc")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("tenant_id"));
    }

    #[tokio::test]
    async fn durable_path_with_unparseable_tenant_fails_closed() {
        let reader: &dyn DocumentReader = &StubReader;
        let err = allocate_ingest_generation(Some(reader), true, Some("not-a-uuid"), "ws", "doc")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("invalid tenant_id"));
    }
}
