//! P-G1 contract test (RC-6 / SPEC-021): entity identity is a newtype.
//!
//! Acceptance (plan-19 §3 P-G1): ingest the same entity under casing variants
//! "John Doe" / "john doe" / "JOHN DOE" through the merger; assert exactly
//! **one** graph node and **one** entity vector.
//!
//! This locks in that the graph node id and the entity vector id are both
//! derived from the single canonical `EntityId`, so the three-convention
//! divergence documented in file 18 cannot recur.

use std::sync::Arc;

use edgequake_pipeline::{ExtractedEntity, ExtractionResult, KnowledgeGraphMerger, MergerConfig};
use edgequake_storage::traits::{GraphStorage, GraphStorageReadOps, VectorStorage};
use edgequake_storage::{EntityId, MemoryGraphStorage, MemoryVectorStorage};

fn make_entity(name: &str, embedding: Vec<f32>) -> ExtractedEntity {
    ExtractedEntity {
        name: name.to_string(),
        entity_type: "PERSON".to_string(),
        description: format!("A person named {name}"),
        importance: 0.7,
        source_spans: vec![],
        source_chunk_ids: vec![format!("doc-{}-chunk-0", name.len())],
        embedding: Some(embedding),
        source_document_id: Some("doc-1".to_string()),
        source_file_path: None,
    }
}

#[tokio::test]
async fn casing_variants_collapse_to_one_node_and_one_vector() {
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    let vector = Arc::new(MemoryVectorStorage::new("test", 4));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    // Three casing variants of the same real entity, each in its own extraction
    // result (simulating three chunks/documents).
    let variants = ["John Doe", "john doe", "JOHN DOE"];
    let results: Vec<ExtractionResult> = variants
        .iter()
        .map(|v| {
            let mut r = ExtractionResult::new(format!("doc-{v}-chunk-0"));
            r.add_entity(make_entity(v, vec![0.1, 0.2, 0.3, 0.4]));
            r
        })
        .collect();

    let stats = merger.merge(results).await.expect("merge must succeed");

    // Exactly one entity was created (the other two were updates of the same node).
    assert_eq!(
        stats.entities_created, 1,
        "expected 1 new node, got {}: casing variants must collapse via EntityId",
        stats.entities_created
    );

    // The single graph node id is the normalized EntityId.
    let expected = EntityId::new("John Doe");
    let node = graph
        .get_node(expected.as_graph_node_id())
        .await
        .unwrap()
        .expect("the normalized node must exist");
    assert_eq!(node.id, "JOHN_DOE");

    // Exactly one entity vector exists, keyed by the derived vector id.
    let vector_id = expected.as_vector_id();
    let stored_emb = vector.get_by_id(&vector_id).await.unwrap();
    assert!(
        stored_emb.is_some(),
        "expected one entity vector at {vector_id}, found none"
    );

    // No raw-name vector leaked into the store.
    assert!(
        vector.get_by_id("John Doe").await.unwrap().is_none(),
        "raw-name entity vector must not exist"
    );
    assert!(
        vector.get_by_id("entity:John Doe").await.unwrap().is_none(),
        "raw-name prefixed entity vector must not exist"
    );
    assert!(
        vector.get_by_id("entity:john doe").await.unwrap().is_none(),
        "raw-name prefixed entity vector must not exist (lowercase variant)"
    );

    // The stored metadata's entity_name is the normalized name (RC-6 fix), so
    // the query decoder recovers the same id the graph uses. Retrieve metadata
    // by querying with the vector id as a filter (memory adapter returns full
    // VectorSearchResult including metadata).
    let probe = vec![0.1_f32, 0.2, 0.3, 0.4];
    let hits = vector
        .query(&probe, 10, Some(std::slice::from_ref(&vector_id)))
        .await
        .unwrap();
    let entity_hit = hits
        .iter()
        .find(|h| h.id == vector_id)
        .expect("the entity vector must be retrievable by its id");
    assert_eq!(
        entity_hit
            .metadata
            .get("entity_name")
            .and_then(|v| v.as_str()),
        Some("JOHN_DOE"),
        "metadata.entity_name must be the normalized name, not the raw extraction name"
    );
}

#[tokio::test]
async fn empty_entity_name_is_skipped() {
    // E1: an empty/whitespace name must not create a "" node.
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    let vector = Arc::new(MemoryVectorStorage::new("test", 4));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    let mut r = ExtractionResult::new("doc-empty-chunk-0");
    r.add_entity(make_entity("   ", vec![0.1, 0.2, 0.3, 0.4]));
    let stats = merger.merge(vec![r]).await.unwrap();

    assert_eq!(stats.entities_created, 0);
    assert!(graph.get_node("").await.unwrap().is_none());
}
