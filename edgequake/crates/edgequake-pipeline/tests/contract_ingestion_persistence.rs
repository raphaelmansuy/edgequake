//! SPEC-021 P-G2 contract — one persist sequence, deduplicated graph identity.

mod common;

use std::sync::Arc;

use common::{sample_persist_context, sample_processing_result, EMBED_DIM};
use edgequake_pipeline::{
    persist_processing_result, ChunkVectorBuildOptions, IngestionPersistConfig,
    IngestionPersistContext, IngestionPersistSettings, MergerConfig, NoopEntitySink,
};
use edgequake_storage::{
    EntityId, GraphStorageReadOps, MemoryGraphStorage, MemoryVectorStorage, VectorStorage,
};

#[tokio::test]
async fn contract_double_persist_merges_to_single_normalized_entity() {
    let graph = Arc::new(MemoryGraphStorage::new("contract"));
    let vector = Arc::new(MemoryVectorStorage::new("contract", EMBED_DIM));
    vector.initialize().await.unwrap();

    let config = IngestionPersistConfig::from_settings(
        IngestionPersistSettings {
            use_llm_summarization: false,
        },
        Arc::new(NoopEntitySink),
        None,
    );
    let ctx = sample_persist_context();
    let options = ChunkVectorBuildOptions::STANDARD;

    let result = sample_processing_result();
    persist_processing_result(
        graph.clone(),
        vector.clone(),
        &config,
        &ctx,
        &result,
        options,
    )
    .await
    .expect("first persist");

    persist_processing_result(
        graph.clone(),
        vector.clone(),
        &config,
        &ctx,
        &result,
        options,
    )
    .await
    .expect("second persist");

    let entity_id = EntityId::new("Sarah Chen");
    let node_id = entity_id.as_graph_node_id().to_string();
    assert!(
        graph.get_node(&node_id).await.unwrap().is_some(),
        "normalized graph node must exist"
    );
    let nodes = graph
        .get_nodes_by_ids(std::slice::from_ref(&node_id))
        .await
        .unwrap();
    assert_eq!(
        nodes.len(),
        1,
        "duplicate persist must not fork graph nodes"
    );

    let chunk_vectors = vector
        .query(&[0.0_f32; EMBED_DIM], 100, None)
        .await
        .unwrap();
    let chunk_count = chunk_vectors
        .iter()
        .filter(|r| r.metadata.get("type").and_then(|v| v.as_str()) == Some("chunk"))
        .count();
    assert_eq!(
        chunk_count, 1,
        "chunk vectors should not duplicate on re-persist"
    );
}

#[tokio::test]
async fn contract_chunk_vector_metadata_uses_content_ref_not_inline_body() {
    let graph = Arc::new(MemoryGraphStorage::new("dedupe"));
    let vector = Arc::new(MemoryVectorStorage::new("dedupe", EMBED_DIM));
    vector.initialize().await.unwrap();

    let config = IngestionPersistConfig::from_settings(
        IngestionPersistSettings {
            use_llm_summarization: false,
        },
        Arc::new(NoopEntitySink),
        None,
    );
    let ctx = sample_persist_context();
    let result = sample_processing_result();

    persist_processing_result(
        graph,
        vector.clone(),
        &config,
        &ctx,
        &result,
        ChunkVectorBuildOptions::STANDARD,
    )
    .await
    .expect("persist");

    let chunk_vectors = vector.query(&[0.0_f32; EMBED_DIM], 10, None).await.unwrap();
    let chunk_meta = chunk_vectors
        .iter()
        .find(|r| r.metadata.get("type").and_then(|v| v.as_str()) == Some("chunk"))
        .expect("chunk vector row");

    assert!(
        chunk_meta.metadata.get("content").is_none(),
        "vector metadata must not duplicate chunk text (SPEC-024 2.5)"
    );
    assert_eq!(
        chunk_meta
            .metadata
            .get("content_ref")
            .and_then(|v| v.as_str()),
        Some(result.chunks[0].id.as_str())
    );
}

#[test]
fn contract_persist_config_parity_across_callers() {
    let settings = IngestionPersistSettings {
        use_llm_summarization: false,
    };
    let sink: Arc<dyn edgequake_pipeline::RelationalEntitySink> = Arc::new(NoopEntitySink);

    let orchestrator_style = IngestionPersistConfig::from_settings(settings, sink.clone(), None);
    let processor_style = IngestionPersistConfig::from_settings(settings, sink, None);

    assert_eq!(
        orchestrator_style.merger_config.use_llm_summarization,
        processor_style.merger_config.use_llm_summarization
    );
    assert_eq!(
        orchestrator_style.merger_config.max_description_length,
        MergerConfig::default().max_description_length
    );
    assert_eq!(
        ChunkVectorBuildOptions::default(),
        ChunkVectorBuildOptions::STANDARD
    );
}

#[tokio::test]
async fn contract_cross_document_entity_merge() {
    let graph = Arc::new(MemoryGraphStorage::new("cross-doc"));
    let vector = Arc::new(MemoryVectorStorage::new("cross-doc", EMBED_DIM));
    vector.initialize().await.unwrap();

    let config = IngestionPersistConfig::from_settings(
        IngestionPersistSettings {
            use_llm_summarization: false,
        },
        Arc::new(NoopEntitySink),
        None,
    );

    let mut doc_a = sample_processing_result();
    doc_a.document_id = "doc-a".to_string();
    doc_a.chunks[0].id = "doc-a-chunk-0".to_string();
    doc_a.extractions[0].source_chunk_id = "doc-a-chunk-0".to_string();
    doc_a.extractions[0].entities[0] =
        edgequake_pipeline::ExtractedEntity::new("Sarah Chen", "PERSON", "Engineer")
            .with_source_chunk_id("doc-a-chunk-0")
            .with_importance(0.9);

    let mut doc_b = sample_processing_result();
    doc_b.document_id = "doc-b".to_string();
    doc_b.chunks[0].id = "doc-b-chunk-0".to_string();
    doc_b.extractions[0].source_chunk_id = "doc-b-chunk-0".to_string();
    doc_b.extractions[0].entities[0] =
        edgequake_pipeline::ExtractedEntity::new("Sarah Chen", "PERSON", "Engineer")
            .with_source_chunk_id("doc-b-chunk-0")
            .with_importance(0.9);

    persist_processing_result(
        graph.clone(),
        vector.clone(),
        &config,
        &IngestionPersistContext::new("doc-a", None, None),
        &doc_a,
        ChunkVectorBuildOptions::STANDARD,
    )
    .await
    .expect("doc a");

    persist_processing_result(
        graph.clone(),
        vector.clone(),
        &config,
        &IngestionPersistContext::new("doc-b", None, None),
        &doc_b,
        ChunkVectorBuildOptions::STANDARD,
    )
    .await
    .expect("doc b");

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    let node = graph
        .get_node(&node_id)
        .await
        .unwrap()
        .expect("merged node");
    let chunk_ids: Vec<String> = node
        .properties
        .get("source_chunk_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    assert!(
        chunk_ids.iter().any(|id| id == "doc-a-chunk-0"),
        "doc-a chunk id must be on merged node"
    );
    assert!(
        chunk_ids.iter().any(|id| id == "doc-b-chunk-0"),
        "doc-b chunk id must be on merged node"
    );
}

#[tokio::test]
async fn contract_persister_trait_matches_free_function() {
    let graph = Arc::new(MemoryGraphStorage::new("trait"));
    let vector = Arc::new(MemoryVectorStorage::new("trait", 4));
    vector.initialize().await.unwrap();

    let config = IngestionPersistConfig::from_settings(
        IngestionPersistSettings {
            use_llm_summarization: false,
        },
        Arc::new(NoopEntitySink),
        None,
    );
    let ctx = sample_persist_context();
    let result = sample_processing_result();

    let free = persist_processing_result(
        graph.clone(),
        vector.clone(),
        &config,
        &ctx,
        &result,
        ChunkVectorBuildOptions::STANDARD,
    )
    .await
    .expect("free fn");

    let graph2 = Arc::new(MemoryGraphStorage::new("trait2"));
    let vector2 = Arc::new(MemoryVectorStorage::new("trait2", 4));
    vector2.initialize().await.unwrap();

    let settings = IngestionPersistSettings {
        use_llm_summarization: false,
    };
    let persister = edgequake_pipeline::DefaultIngestionPersister::from_settings(
        graph2.clone(),
        vector2.clone(),
        settings,
        Arc::new(NoopEntitySink),
        None,
        None,
    );
    use edgequake_pipeline::IngestionPersister;
    let trait_out = persister
        .persist(&ctx, &result, ChunkVectorBuildOptions::STANDARD)
        .await
        .expect("trait persist");

    assert_eq!(
        free.chunk_vector_ids.len(),
        trait_out.chunk_vector_ids.len()
    );
    assert!(trait_out.merge_stats.entities_created + trait_out.merge_stats.entities_updated > 0);
}
