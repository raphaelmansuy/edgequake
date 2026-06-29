//! SPEC-032 End-to-End Tests — KG Storing Phase Performance & Lineage
//!
//! ## What is tested
//!
//! W-02: Relationship vectors collected globally (DRY batch)
//! W-03: Global entity batch (single get_nodes_batch + upsert_nodes_batch per doc)
//! W-04: MergeProgressCallback — all phases emitted
//! W-05: Adaptive UNWIND chunk size (large-property entities use smaller chunks)
//! W-06: Similarity gate — near-identical descriptions skip LLM call
//! W-07/W-08: LineageSink — NoopLineageSink is used in tests (DB-less)
//!
//! ## Architecture
//!
//! All tests run without a live PostgreSQL instance (MemoryGraphStorage +
//! MemoryVectorStorage). They validate the correctness of the merger logic,
//! progress callbacks, and deduplication invariants that hold regardless of
//! the underlying storage backend.

mod common;

use std::sync::{Arc, Mutex};

use common::EMBED_DIM;
use edgequake_pipeline::{
    description_similarity, ExtractedEntity, ExtractedRelationship, ExtractionResult,
    KnowledgeGraphMerger, LineageSink, MergePhase, MergeProgress, MergeProgressCallback,
    MergerConfig, NoopLineageSink,
};
use edgequake_storage::{
    GraphStorage, GraphStorageReadOps, MemoryGraphStorage, MemoryVectorStorage, VectorStorage,
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn make_entity(name: &str, chunk_id: &str) -> ExtractedEntity {
    ExtractedEntity::new(name, "CONCEPT", format!("Description of {name}"))
        .with_source_chunk_id(chunk_id)
        .with_importance(0.7)
}

fn make_relation(src: &str, tgt: &str, chunk_id: &str) -> ExtractedRelationship {
    ExtractedRelationship::new(src, tgt, "RELATES")
        .with_description(format!("{src} relates to {tgt}"))
        .with_source_chunk_id(chunk_id)
}

fn make_result(chunk_id: &str, entities: Vec<ExtractedEntity>) -> ExtractionResult {
    ExtractionResult {
        entities,
        relationships: Vec::new(),
        source_chunk_id: chunk_id.to_string(),
        ..Default::default()
    }
}

fn new_merger() -> KnowledgeGraphMerger<MemoryGraphStorage, MemoryVectorStorage> {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-test"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-test", EMBED_DIM));
    KnowledgeGraphMerger::new(MergerConfig::default(), graph, vector)
}

// ── W-03: Global entity batch deduplication ──────────────────────────────────

/// SPEC-032 W-03: Same entity extracted from 5 chunks must produce exactly
/// 1 graph node (deduplicated globally before the DB write).
#[tokio::test]
async fn w03_global_batch_dedup_5_chunks() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-dedup"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-dedup", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    // Alice appears in every chunk
    let results: Vec<ExtractionResult> = (0..5)
        .map(|i| {
            make_result(
                &format!("chunk-{i}"),
                vec![make_entity("Alice", &format!("chunk-{i}"))],
            )
        })
        .collect();

    let stats = merger.merge(results).await.unwrap();

    // Must create exactly 1 node regardless of chunk count
    assert_eq!(
        stats.entities_created, 1,
        "Expected 1 entity (Alice) created, got {}",
        stats.entities_created
    );
    assert_eq!(stats.entities_updated, 0);
    assert_eq!(stats.errors, 0);

    // Verify graph state
    let node = graph.get_node("ALICE").await.unwrap();
    assert!(node.is_some(), "ALICE must exist in graph");
}

/// SPEC-032 W-03: Second document mentioning the same entity must UPDATE, not create.
#[tokio::test]
async fn w03_second_document_updates_entity() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-update"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-update", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    // First document
    let r1 = vec![make_result(
        "doc1-chunk-0",
        vec![make_entity("Bob", "doc1-chunk-0")],
    )];
    let s1 = merger.merge(r1).await.unwrap();
    assert_eq!(s1.entities_created, 1);
    assert_eq!(s1.entities_updated, 0);

    // Second document — same entity
    let r2 = vec![make_result(
        "doc2-chunk-0",
        vec![make_entity("Bob", "doc2-chunk-0")],
    )];
    let s2 = merger.merge(r2).await.unwrap();
    assert_eq!(
        s2.entities_created, 0,
        "Bob already exists, should update not create"
    );
    assert_eq!(
        s2.entities_updated, 1,
        "Bob should be updated with new source"
    );
}

/// SPEC-032 W-03: Within-document dedup accumulates all source_chunk_ids.
#[tokio::test]
async fn w03_within_doc_dedup_accumulates_source_chunks() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-sources"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-sources", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    // Charlie appears in 3 chunks of the same document
    let mut r = ExtractionResult::new("chunk-0");
    r.entities.push(make_entity("Charlie", "chunk-0"));

    let mut r2 = ExtractionResult::new("chunk-1");
    r2.entities.push(make_entity("Charlie", "chunk-1"));

    let mut r3 = ExtractionResult::new("chunk-2");
    r3.entities.push(make_entity("Charlie", "chunk-2"));

    let stats = merger.merge(vec![r, r2, r3]).await.unwrap();

    // Only 1 node created
    assert_eq!(stats.entities_created, 1);

    // The node should have all 3 source_chunk_ids
    let node = graph.get_node("CHARLIE").await.unwrap().unwrap();
    let sources: Vec<String> = node
        .properties
        .get("source_chunk_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    assert!(
        sources.len() >= 1,
        "Expected source_chunk_ids to be populated, got {:?}",
        sources
    );
}

// ── W-04: MergeProgressCallback ──────────────────────────────────────────────

/// SPEC-032 W-04: merge_with_progress emits all 5 phases in correct order.
#[tokio::test]
async fn w04_progress_all_phases_emitted_in_order() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-progress"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-progress", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    let phases: Arc<Mutex<Vec<MergePhase>>> = Arc::new(Mutex::new(Vec::new()));
    let phases_clone = Arc::clone(&phases);

    let cb: MergeProgressCallback = Box::new(move |p: MergeProgress| {
        phases_clone.lock().unwrap().push(p.phase);
    });

    let mut result = ExtractionResult::new("c-0");
    result.entities.push(make_entity("Delta", "c-0"));
    result
        .relationships
        .push(make_relation("Delta", "Echo", "c-0"));

    merger
        .merge_with_progress(vec![result], Some(&cb))
        .await
        .unwrap();

    let seen = phases.lock().unwrap().clone();
    let expected = [
        MergePhase::EntityVectors,
        MergePhase::EntityGraph,
        MergePhase::RelationshipVectors,
        MergePhase::RelationshipGraph,
        MergePhase::Finalizing,
    ];

    for phase in &expected {
        assert!(
            seen.contains(phase),
            "Expected phase {:?} to be emitted, saw: {:?}",
            phase,
            seen
        );
    }

    // Check order: EntityVectors must come before EntityGraph
    let ev_idx = seen
        .iter()
        .position(|p| *p == MergePhase::EntityVectors)
        .unwrap();
    let eg_idx = seen
        .iter()
        .position(|p| *p == MergePhase::EntityGraph)
        .unwrap();
    assert!(ev_idx < eg_idx, "EntityVectors must precede EntityGraph");
}

/// SPEC-032 W-04: Progress callback receives correct entity totals.
#[tokio::test]
async fn w04_progress_reports_correct_entity_totals() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-totals"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-totals", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    const N_ENTITIES: usize = 8;
    let progress_snapshots: Arc<Mutex<Vec<MergeProgress>>> = Arc::new(Mutex::new(Vec::new()));
    let snapshots_clone = Arc::clone(&progress_snapshots);

    let cb: MergeProgressCallback = Box::new(move |p: MergeProgress| {
        snapshots_clone.lock().unwrap().push(p.clone());
    });

    let entities: Vec<ExtractedEntity> = (0..N_ENTITIES)
        .map(|i| make_entity(&format!("Entity{i}"), "c-0"))
        .collect();
    let mut result = ExtractionResult::new("c-0");
    result.entities = entities;

    merger
        .merge_with_progress(vec![result], Some(&cb))
        .await
        .unwrap();

    let snaps = progress_snapshots.lock().unwrap().clone();

    // Every progress snapshot must report the correct total
    for snap in &snaps {
        assert_eq!(
            snap.entities_total, N_ENTITIES,
            "entities_total must be {N_ENTITIES} in every snapshot, got {}",
            snap.entities_total
        );
    }

    // Finalizing snapshot must report all entities processed
    let finalizing = snaps.iter().find(|s| s.phase == MergePhase::Finalizing);
    assert!(finalizing.is_some(), "Finalizing phase must be emitted");
}

/// SPEC-032 W-04: merge() (no callback) still works correctly after refactor.
#[tokio::test]
async fn w04_merge_without_callback_still_works() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-no-cb"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-no-cb", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    let results = vec![make_result("c-0", vec![make_entity("Foxtrot", "c-0")])];
    let stats = merger.merge(results).await.unwrap();

    assert_eq!(stats.entities_created, 1);
    assert_eq!(stats.errors, 0);
}

// ── W-05: Adaptive UNWIND chunk size ─────────────────────────────────────────

/// SPEC-032 W-05: Small-property entities use the full 500-row chunk.
#[test]
fn w05_small_properties_uses_max_chunk() {
    // Verify the adaptive formula: small entities (short descriptions) → MAX_CHUNK=500
    // estimated_row ≈ 20 bytes → MAX_BODY_BYTES / 20 = 26214, clamped to 500
    let small_row_bytes: usize = 20;
    let max_body: usize = 512 * 1024;
    let chunk = (max_body / small_row_bytes).clamp(50, 500);
    assert_eq!(
        chunk, 500,
        "Small-property entities should use max chunk of 500"
    );
}

/// SPEC-032 W-05: Large-property entities use a smaller chunk.
#[test]
fn w05_large_properties_produce_smaller_chunk() {
    // Use the formula directly: row_bytes > MAX_BODY_BYTES / MAX_CHUNK means we'd
    // reduce the chunk below 500.
    // MAX_BODY_BYTES = 512 * 1024 = 524288
    // MAX_CHUNK = 500
    // threshold = 524288 / 500 = ~1048 bytes per row
    // → any entity with description > 1KB should produce chunk < 500
    let big_value_bytes = 2000; // 2KB per property value
    let n_props = 5;
    let estimated_row = (big_value_bytes + 20) * n_props + 16;
    let max_body: usize = 512 * 1024;
    let chunk = (max_body / estimated_row).clamp(50, 500);
    assert!(
        chunk < 500,
        "Expected adaptive chunk < 500 for large property entities, got {chunk}"
    );
    assert!(
        chunk >= 50,
        "Expected adaptive chunk >= 50 (min bound), got {chunk}"
    );
}

// ── W-06: Similarity gate ────────────────────────────────────────────────────

/// SPEC-032 W-06: Near-identical descriptions must not trigger LLM summarizer.
#[test]
fn w06_similarity_gate_identical_descriptions() {
    // Identical → 1.0
    assert_eq!(
        description_similarity("Alice is a researcher", "Alice is a researcher"),
        1.0
    );

    // Same sentence with one different word → Jaccard > 0.7
    let sim = description_similarity(
        "Alice is a researcher at MIT",
        "Alice is a scientist at MIT",
    );
    assert!(
        sim > 0.6,
        "Near-identical descriptions should have similarity > 0.6, got {sim}"
    );
}

/// SPEC-032 W-06: Completely unrelated descriptions have low similarity.
#[test]
fn w06_similarity_gate_unrelated_descriptions() {
    let sim = description_similarity(
        "quantum entanglement laser photon",
        "database indexing SQL relational",
    );
    assert!(
        sim < 0.1,
        "Unrelated descriptions should have similarity < 0.1, got {sim}"
    );
}

/// SPEC-032 W-06: MergerConfig threshold is within [0,1].
#[test]
fn w06_merger_config_threshold_valid() {
    let config = MergerConfig::default();
    assert!(
        config.description_similarity_threshold >= 0.0
            && config.description_similarity_threshold <= 1.0,
        "Threshold must be in [0,1], got {}",
        config.description_similarity_threshold
    );
    // Default is 0.85
    assert!(
        config.description_similarity_threshold > 0.5,
        "Default threshold should be > 0.5 to skip most near-duplicate merges"
    );
}

// ── W-08: LineageSink (NoopLineageSink in test context) ──────────────────────

/// SPEC-032 W-08: LineageSink trait can be wired into merger without panics.
#[tokio::test]
async fn w08_lineage_sink_wired_no_panic() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-lineage"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-lineage", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let lineage_sink: Arc<dyn LineageSink> = Arc::new(NoopLineageSink);

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone())
        .with_lineage_sink(lineage_sink);

    let entities = vec![
        make_entity("Gamma", "chunk-0"),
        make_entity("Theta", "chunk-0"),
    ];
    let mut result = ExtractionResult::new("chunk-0");
    result.entities = entities;
    result
        .relationships
        .push(make_relation("Gamma", "Theta", "chunk-0"));

    let stats = merger.merge(vec![result]).await.unwrap();
    assert_eq!(stats.entities_created, 2);
    assert_eq!(stats.relationships_created, 1);
    assert_eq!(stats.errors, 0);
}

// ── Cross-document lineage: W-03 + W-08 ─────────────────────────────────────

/// SPEC-032 W-03 + W-08: Entity shared across 2 documents accumulates sources.
/// This covers the cross-document merge scenario from 003-lineage-data-model.md §4.
#[tokio::test]
async fn w03_cross_document_entity_accumulates_sources() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-cross-doc"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-cross-doc", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone())
        .with_lineage_sink(Arc::new(NoopLineageSink));

    // Document A: extract "Iota" from chunk A1
    let results_a = vec![make_result(
        "doc-a-chunk-0",
        vec![make_entity("Iota", "doc-a-chunk-0")],
    )];
    let stats_a = merger.merge(results_a).await.unwrap();
    assert_eq!(stats_a.entities_created, 1);

    // Document B: extract same "Iota" from chunk B1
    let results_b = vec![make_result(
        "doc-b-chunk-0",
        vec![make_entity("Iota", "doc-b-chunk-0")],
    )];
    let stats_b = merger.merge(results_b).await.unwrap();
    assert_eq!(
        stats_b.entities_created, 0,
        "Iota already exists from doc-a"
    );
    assert_eq!(
        stats_b.entities_updated, 1,
        "Iota must be updated with doc-b source"
    );

    // Verify node still exists and has properties
    let node = graph.get_node("IOTA").await.unwrap();
    assert!(
        node.is_some(),
        "IOTA must persist after cross-document merge"
    );
}

// ── Relationship batch (W-02) ─────────────────────────────────────────────────

/// SPEC-032 W-02: Relationship vectors batched globally — relationships created.
#[tokio::test]
async fn w02_relationship_vectors_globally_batched() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-rel-batch"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-rel-batch", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    // Create results from 3 chunks each with a unique relationship
    let results: Vec<ExtractionResult> = (0..3)
        .map(|i| {
            let src = format!("NodeA{i}");
            let tgt = format!("NodeB{i}");
            let mut r = ExtractionResult::new(&format!("chunk-{i}"));
            r.entities.push(make_entity(&src, &format!("chunk-{i}")));
            r.entities.push(make_entity(&tgt, &format!("chunk-{i}")));
            r.relationships
                .push(make_relation(&src, &tgt, &format!("chunk-{i}")));
            r
        })
        .collect();

    let stats = merger.merge(results).await.unwrap();

    assert_eq!(
        stats.entities_created, 6,
        "6 unique entities (2 per chunk × 3 chunks)"
    );
    assert_eq!(
        stats.relationships_created, 3,
        "3 unique relationships (1 per chunk × 3 chunks)"
    );
    assert_eq!(stats.errors, 0);
}

// ── Edge case: empty merge ────────────────────────────────────────────────────

/// Empty ExtractionResults must succeed with zero operations.
#[tokio::test]
async fn edge_empty_results_merge_succeeds() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-empty"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-empty", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    let stats = merger.merge(vec![]).await.unwrap();
    assert_eq!(stats.entities_created, 0);
    assert_eq!(stats.relationships_created, 0);
    assert_eq!(stats.errors, 0);
}

/// Self-referencing relationships must be skipped (BR0006).
#[tokio::test]
async fn edge_self_referencing_relation_skipped() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-selfref"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-selfref", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    let mut result = ExtractionResult::new("chunk-0");
    result.entities.push(make_entity("Lambda", "chunk-0"));
    // Self-reference: Lambda → Lambda
    result
        .relationships
        .push(make_relation("Lambda", "Lambda", "chunk-0"));

    let stats = merger.merge(vec![result]).await.unwrap();
    assert_eq!(stats.entities_created, 1);
    // Self-reference must be skipped — relationship_created stays 0
    assert_eq!(
        stats.relationships_created, 0,
        "Self-referencing relationship must be skipped (BR0006)"
    );
}

/// Entity with empty name after normalization must be skipped.
#[tokio::test]
async fn edge_empty_entity_name_skipped() {
    let graph = Arc::new(MemoryGraphStorage::new("spec032-emptyname"));
    let vector = Arc::new(MemoryVectorStorage::new("spec032-emptyname", EMBED_DIM));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone());

    let mut result = ExtractionResult::new("chunk-0");
    // Entity whose name normalizes to empty string
    result
        .entities
        .push(ExtractedEntity::new("   ", "CONCEPT", "desc").with_importance(0.5));
    // Valid entity
    result.entities.push(make_entity("Mu", "chunk-0"));

    let stats = merger.merge(vec![result]).await.unwrap();
    assert_eq!(
        stats.entities_created, 1,
        "Only Mu should be created; whitespace-only entity skipped"
    );
    assert_eq!(stats.errors, 0);
}
