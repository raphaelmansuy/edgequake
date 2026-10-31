//! SPEC-046 e2e: adaptive routing + PPR + path prune + truncation +
//! community reports + multimodal injection + GraphRAG-Bench report.
//!
//! Runs without Postgres — memory storages + mock LLM/embedder.

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_pipeline::chunker::TextChunk;
use edgequake_pipeline::extractor::{ExtractedEntity, ExtractionResult};
use edgequake_pipeline::{
    inject_modality_relations, MmChunkSidecarMeta, MmHeadingBlock, MmSidecarBlock, MmSidecarRef,
};
use edgequake_query::community_global::append_community_report_vector_chunks;
use edgequake_query::eval::{
    assert_case_routing, run_spec046_acc_report, run_spec046_bench_report, spec046_synthetic_bench,
};
use edgequake_query::graph_expand::expand_neighborhood_edges;
use edgequake_query::graph_ppr::GraphWalkMode;
use edgequake_query::keywords::QueryIntent;
use edgequake_query::kg_chunk_pick::pick_chunks_by_weight;
use edgequake_query::path_prune::{prune_relationships, PathPruneConfig};
use edgequake_query::tokenizer::MockTokenizer;
use edgequake_query::truncation::{balance_context, TruncationConfig};
use edgequake_query::{
    context::{QueryContext, RetrievedChunk, RetrievedEntity, RetrievedRelationship},
    QueryEngine, QueryEngineConfig, QueryMode, QueryRequest,
};
use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryVectorStorage};
use edgequake_storage::traits::{
    GraphReadView, GraphStorage, GraphStorageMutateOps, VectorStorage,
};
use edgequake_storage::{Community, CommunityDetectionResult, VectorSearchResult};

#[test]
fn e2e_spec046_bench_routing_table() {
    for case in spec046_synthetic_bench() {
        assert_case_routing(&case);
    }
}

#[test]
fn e2e_spec046_bench_report_full_pass() {
    let report = run_spec046_bench_report();
    assert_eq!(report.failed, 0, "failures: {:?}", report.cases);
    assert_eq!(report.pass_rate, 1.0);
    assert!(report.total >= 8);
    let json = serde_json::to_string(&report).expect("report serializes");
    assert!(json.contains("l1_what_is"));
}

#[test]
fn e2e_spec046_acc_harness_full_pass() {
    let report = run_spec046_acc_report();
    assert!(
        report.is_full_pass(),
        "ACC failures: {:?}",
        report
            .checks
            .iter()
            .filter(|c| !c.passed)
            .collect::<Vec<_>>()
    );
}

#[test]
fn e2e_factual_heuristic_maps_to_naive() {
    let intent = QueryIntent::classify_heuristic("What is Rust?");
    assert_eq!(intent, QueryIntent::Factual);
    assert_eq!(intent.recommended_mode(), QueryMode::Naive);
}

#[tokio::test]
async fn e2e_ppr_walk_expands_seed_neighborhood() {
    let graph = MemoryGraphStorage::new("ppr-e2e");
    graph.initialize().await.unwrap();
    for (a, b) in [("SEED", "N1"), ("N1", "N2"), ("N2", "N3"), ("OTHER", "Z")] {
        graph.upsert_edge(a, b, HashMap::new()).await.unwrap();
    }
    let view = GraphReadView::new(&graph);
    let edges = expand_neighborhood_edges(
        &view,
        &["SEED".into()],
        2,
        10,
        GraphWalkMode::Ppr,
        None,
        None,
    )
    .await
    .unwrap();
    assert!(!edges.is_empty());
    assert!(
        edges
            .iter()
            .any(|e| e.source == "SEED" || e.target == "SEED" || e.source == "N1"),
        "PPR should keep seed-local edges; got {edges:?}"
    );
}

#[test]
fn e2e_path_prune_reduces_relation_tax() {
    let rels: Vec<_> = (0..20)
        .map(|i| {
            RetrievedRelationship::new("A", format!("T{i}"), "REL")
                .with_score(i as f32 * 0.05)
                .with_description(if i > 10 { "rich evidence path" } else { "" })
        })
        .collect();
    let cfg = PathPruneConfig {
        drop_fraction: 0.4,
        min_keep: 3,
        min_input: 5,
        ..Default::default()
    };
    let kept = prune_relationships(rels, &cfg);
    assert_eq!(kept.len(), 12);
}

#[test]
fn e2e_dynamic_truncation_gives_chunks_remainder() {
    let tokenizer = MockTokenizer::with_rate(1.0);
    let config = TruncationConfig {
        max_entity_tokens: 50,
        max_relation_tokens: 50,
        max_total_tokens: 100,
        buffer_tokens: 10,
        min_chunk_budget_ratio: 0.0,
    };
    let entities = vec![RetrievedEntity::new("E", "T", "x")];
    let rels = vec![RetrievedRelationship::new("A", "B", "R")];
    let chunks = vec![
        RetrievedChunk::new("c1", "AAAAAAAAAA", 1.0),
        RetrievedChunk::new("c2", "BBBBBBBBBB", 0.9),
        RetrievedChunk::new("c3", "CCCCCCCCCC", 0.8),
        RetrievedChunk::new("c4", "DDDDDDDDDD", 0.7),
    ];
    let (e, r, c) = balance_context(entities, rels, chunks, &config, &tokenizer);
    assert!(!e.is_empty());
    assert!(!r.is_empty());
    assert!(!c.is_empty(), "chunks should receive dynamic remainder");
}

#[tokio::test]
async fn e2e_local_mode_with_ppr_config_completes() {
    let config = QueryEngineConfig {
        use_adaptive_mode: false,
        use_keyword_extraction: false,
        graph_walk: GraphWalkMode::Ppr,
        enable_rerank: false,
        enable_bm25_retrieval: false,
        path_prune: PathPruneConfig {
            drop_fraction: 0.0,
            ..PathPruneConfig::default()
        },
        min_score: 0.0,
        ..QueryEngineConfig::default()
    };

    let vs = Arc::new(MemoryVectorStorage::new("ppr-local", 1536));
    vs.initialize().await.unwrap();
    let gs = Arc::new(MemoryGraphStorage::new("ppr-local"));
    gs.initialize().await.unwrap();
    gs.upsert_edge("ALPHA", "BETA", HashMap::new())
        .await
        .unwrap();

    let mut meta = serde_json::Map::new();
    meta.insert("type".into(), serde_json::json!("entity"));
    meta.insert("entity_name".into(), serde_json::json!("ALPHA"));
    vs.upsert(&[(
        "entity:ALPHA".into(),
        vec![0.15_f32; 1536],
        serde_json::Value::Object(meta),
    )])
    .await
    .unwrap();

    let embed = Arc::new(MockProvider::new());
    let llm = Arc::new(MockProvider::new());
    let engine = QueryEngine::new(config, vs, gs, embed, llm);

    let req = QueryRequest::new("ALPHA connections").with_mode(QueryMode::Local);
    let resp = engine.query(req).await.expect("local+ppr query");
    let names: Vec<_> = resp
        .context
        .entities
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert!(
        names.contains(&"ALPHA") || !resp.context.relationships.is_empty(),
        "expected ALPHA or relationships, got entities={names:?} rels={}",
        resp.context.relationships.len()
    );
}

#[test]
fn e2e_kg_chunk_pick_weight_orders_by_frequency() {
    let mut ctx = QueryContext::new();
    let mut e1 = RetrievedEntity::new("A", "P", "d");
    e1.source_chunk_ids = vec!["shared".into(), "rare".into()];
    let mut e2 = RetrievedEntity::new("B", "P", "d");
    e2.source_chunk_ids = vec!["shared".into()];
    ctx.add_entity(e1);
    ctx.add_entity(e2);
    let picked = pick_chunks_by_weight(&ctx, 2);
    assert_eq!(picked.first().map(String::as_str), Some("shared"));
}

#[test]
fn e2e_community_report_vector_chunks_merge_into_context() {
    let mut ctx = QueryContext::new();
    let hits = vec![
        VectorSearchResult {
            id: "community_report:0".into(),
            score: 0.91,
            metadata: serde_json::json!({
                "type": "community_report",
                "content": "Community 0 (2 entities): ALPHA, BETA."
            }),
        },
        VectorSearchResult {
            id: "community_report:0".into(), // duplicate id — should dedupe
            score: 0.5,
            metadata: serde_json::json!({
                "type": "community_report",
                "content": "dup"
            }),
        },
        VectorSearchResult {
            id: "community_report:1".into(),
            score: 0.8,
            metadata: serde_json::json!({
                "type": "community_report",
                "content": "Community 1 (1 entities): GAMMA."
            }),
        },
    ];
    append_community_report_vector_chunks(&mut ctx, &hits, 8);
    assert_eq!(ctx.chunks.len(), 2);
    assert!(ctx.chunks[0].content.contains("ALPHA"));
}

#[tokio::test]
async fn e2e_community_report_index_with_embedder() {
    std::env::set_var("EDGEQUAKE_COMMUNITY_REPORTS", "true");

    struct FixedEmbedder;
    #[async_trait::async_trait]
    impl edgequake_storage::TextEmbedder for FixedEmbedder {
        async fn embed_texts(
            &self,
            texts: &[String],
        ) -> edgequake_storage::error::Result<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|_| vec![0.25_f32; 8]).collect())
        }
    }

    let mut map = HashMap::new();
    map.insert("ALPHA".into(), 0usize);
    map.insert("BETA".into(), 0usize);
    let result = CommunityDetectionResult {
        communities: vec![Community {
            id: 0,
            members: vec!["ALPHA".into(), "BETA".into()],
            properties: HashMap::new(),
        }],
        node_to_community: map,
        modularity: 0.1,
        hierarchy_levels: 1,
    };
    let vs = MemoryVectorStorage::new("comm-index-e2e", 8);
    vs.initialize().await.unwrap();
    let n = edgequake_storage::index_community_reports_with_embedder(
        &result,
        &vs,
        &FixedEmbedder,
        Some("ws"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(n, 1);
    std::env::remove_var("EDGEQUAKE_COMMUNITY_REPORTS");
}

#[test]
fn e2e_role_llm_keyword_and_summary_resolve() {
    use edgequake_core::{resolve_role_llm, role_config_from_workspace, LlmRole, Workspace};
    use uuid::Uuid;

    let mut meta = HashMap::new();
    meta.insert(
        "llm_roles".into(),
        serde_json::json!({
            "keyword": { "provider": "mock", "model": "mock-kw" },
            "summary": { "provider": "mock", "model": "mock-sum" }
        }),
    );
    let ws = Workspace {
        workspace_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        name: "roles".into(),
        slug: "roles".into(),
        description: None,
        is_active: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        metadata: meta,
        llm_model: "gemma3:latest".into(),
        llm_provider: "ollama".into(),
        embedding_model: "embeddinggemma:latest".into(),
        embedding_provider: "ollama".into(),
        embedding_dimension: 768,
        vision_llm_model: None,
        vision_llm_provider: None,
        pdf_parser_backend: None,
        rag_profile: Default::default(),
        embedding_modalities: Default::default(),
        fts_regconfig: Default::default(),
    };
    assert!(role_config_from_workspace(&ws, LlmRole::Keyword).is_some());
    assert_eq!(resolve_role_llm(&ws, LlmRole::Keyword).model, "mock-kw");
    assert_eq!(resolve_role_llm(&ws, LlmRole::Summary).model, "mock-sum");
}

#[test]
fn e2e_multimodal_orphan_injection_guarantees_entity() {
    let mm = MmChunkSidecarMeta {
        item_id: "eq1".into(),
        modality: "equation".into(),
        text: "[Equation Name]E=mc^2\n\nbody".into(),
        sidecar: MmSidecarBlock {
            sidecar_type: "equation".into(),
            id: "eq1".into(),
            refs: vec![MmSidecarRef {
                ref_type: "equation".into(),
                id: "eq1".into(),
            }],
        },
        heading: Some(MmHeadingBlock {
            level: 1,
            heading: "Physics".into(),
            parent_headings: vec![],
        }),
        llm_cache_list: vec![],
    };
    let chunks = vec![TextChunk {
        id: "chunk-eq".into(),
        content: mm.text.clone(),
        index: 0,
        start_offset: 0,
        end_offset: 0,
        start_line: 1,
        end_line: 1,
        token_count: 8,
        embedding: None,
        section: None,
        page_start: None,
        page_end: None,
        modality: None,
    }];
    let mut extractions: Vec<ExtractionResult> = Vec::new();
    inject_modality_relations(&mut extractions, &chunks, &[mm], "paper.pdf", None);
    assert_eq!(extractions.len(), 1);
    assert!(extractions[0].entities.iter().any(|e| e.name == "eq1"));
    assert_eq!(extractions[0].entities[0].entity_type, "equation");
}

#[test]
fn e2e_multimodal_injection_links_existing_entities() {
    let mm = MmChunkSidecarMeta {
        item_id: "d1".into(),
        modality: "drawing".into(),
        text: "[Image Name]Arch\n[Image Type]Chart\n\nbody".into(),
        sidecar: MmSidecarBlock {
            sidecar_type: "drawing".into(),
            id: "d1".into(),
            refs: vec![],
        },
        heading: None,
        llm_cache_list: vec![],
    };
    let chunks = vec![TextChunk {
        id: "c0".into(),
        content: mm.text.clone(),
        index: 0,
        start_offset: 0,
        end_offset: 0,
        start_line: 1,
        end_line: 1,
        token_count: 5,
        embedding: None,
        section: None,
        page_start: None,
        page_end: None,
        modality: None,
    }];
    let mut extractions = vec![ExtractionResult {
        entities: vec![ExtractedEntity::new("SYSTEM", "CONCEPT", "ctx")],
        relationships: vec![],
        source_chunk_id: "c0".into(),
        metadata: Default::default(),
        input_tokens: 0,
        output_tokens: 0,
        extraction_time_ms: 0,
    }];
    inject_modality_relations(&mut extractions, &chunks, &[mm], "demo.pdf", None);
    assert!(extractions[0].entities.iter().any(|e| e.name == "d1"));
    assert_eq!(extractions[0].relationships.len(), 1);
}

#[test]
fn e2e_dual_node_ppr_chunk_pick_orders_by_entity_score() {
    use edgequake_query::kg_chunk_pick::pick_chunks_by_entity_ppr;

    let mut ctx = QueryContext::new();
    let mut hot = RetrievedEntity::new("ALPHA", "CONCEPT", "high");
    hot.score = 0.9;
    hot.source_chunk_ids = vec!["passage-a".into()];
    let mut cold = RetrievedEntity::new("BETA", "CONCEPT", "low");
    cold.score = 0.05;
    cold.source_chunk_ids = vec!["passage-b".into()];
    ctx.add_entity(hot);
    ctx.add_entity(cold);

    let ranked = pick_chunks_by_entity_ppr(&ctx, 2);
    assert_eq!(ranked[0], "passage-a");
    assert!(ranked.contains(&"passage-b".into()));
}

#[test]
fn e2e_semantic_chunk_strategy_requires_embeddings() {
    use edgequake_pipeline::ChunkStrategy;
    assert!(ChunkStrategy::Semantic.requires_embeddings());
    assert_eq!(ChunkStrategy::parse("V"), Some(ChunkStrategy::Semantic));
    assert_eq!(
        ChunkStrategy::parse("semantic"),
        Some(ChunkStrategy::Semantic)
    );
}

#[tokio::test]
async fn e2e_llm_text_embedder_adapter_embeds() {
    use edgequake_pipeline::LlmTextEmbedder;
    use edgequake_storage::TextEmbedder;

    let emb = Arc::new(MockProvider::new());
    let adapter = LlmTextEmbedder::arc(emb);
    let vectors = TextEmbedder::embed_texts(adapter.as_ref(), &[String::from("community report")])
        .await
        .expect("embed");
    assert_eq!(vectors.len(), 1);
    assert!(!vectors[0].is_empty());
}

#[test]
fn e2e_summary_role_helper_prefers_configured_model() {
    use edgequake_core::{resolve_role_llm, role_config_from_workspace, LlmRole, Workspace};
    use uuid::Uuid;

    let mut meta = HashMap::new();
    meta.insert(
        "llm_roles".into(),
        serde_json::json!({ "summary": { "provider": "mock", "model": "sum-1" } }),
    );
    let ws = Workspace {
        workspace_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        name: "s".into(),
        slug: "s".into(),
        description: None,
        is_active: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        metadata: meta,
        llm_model: "base".into(),
        llm_provider: "mock".into(),
        embedding_model: "e".into(),
        embedding_provider: "mock".into(),
        embedding_dimension: 8,
        vision_llm_model: None,
        vision_llm_provider: None,
        pdf_parser_backend: None,
        rag_profile: Default::default(),
        embedding_modalities: Default::default(),
        fts_regconfig: Default::default(),
    };
    assert!(role_config_from_workspace(&ws, LlmRole::Summary).is_some());
    assert_eq!(resolve_role_llm(&ws, LlmRole::Summary).model, "sum-1");
}
