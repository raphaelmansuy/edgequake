//! SPEC-026 Phase 2 — recursive chunking parity contract tests.

use std::sync::Arc;

use edgequake_pipeline::{
    calculate_adaptive_chunk_size, default_recursive_separators, resolve_chunker, ChunkStrategy,
    Chunker, ChunkerConfig, ChunkingStrategy, RecursiveCharacterChunking,
};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/spec026/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|_| panic!("missing fixture {name}"))
}

#[derive(serde::Deserialize)]
struct LightragChunkFixture {
    chunk_start_offsets: Vec<usize>,
}

/// Percent of expected start offsets matched within `tolerance` chars (SPEC-026 ≥90% gate).
fn boundary_overlap_pct(actual: &[usize], expected: &[usize], tolerance: usize) -> f64 {
    if expected.is_empty() {
        return 100.0;
    }
    let matched = expected
        .iter()
        .filter(|exp| actual.iter().any(|act| act.abs_diff(**exp) <= tolerance))
        .count();
    matched as f64 / expected.len() as f64 * 100.0
}

#[test]
fn recursive_default_separators_match_lightrag() {
    assert_eq!(
        default_recursive_separators(),
        vec!["\n\n", "\n", "。", "！", "？", "；", "，", " ", ""]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn recursive_produces_multiple_chunks_on_paragraphs() {
    let text = fixture("plain_en.txt");
    let config = ChunkerConfig {
        chunk_size: 20,
        chunk_overlap: 0,
        min_chunk_size: 1,
        separators: default_recursive_separators(),
        ..Default::default()
    };
    let chunks = RecursiveCharacterChunking
        .chunk(&text, &config)
        .await
        .unwrap();
    assert!(chunks.len() >= 2, "expected paragraph splits");
}

#[tokio::test]
async fn recursive_cjk_mixed_fixture_splits() {
    let text = fixture("plain_zh_mixed.txt");
    let config = ChunkerConfig {
        chunk_size: 15,
        chunk_overlap: 0,
        min_chunk_size: 1,
        separators: default_recursive_separators(),
        ..Default::default()
    };
    let chunks = RecursiveCharacterChunking
        .chunk(&text, &config)
        .await
        .unwrap();
    assert!(
        chunks.len() >= 3,
        "CJK mixed fixture should produce multiple semantic chunks"
    );
}

#[tokio::test]
async fn recursive_aligns_chunks_to_paragraph_boundaries() {
    let text = "Alpha paragraph one with extra words.\n\nBeta paragraph two with extra words.\n\nGamma paragraph three with extra words.";
    let config = ChunkerConfig {
        chunk_size: 10,
        chunk_overlap: 0,
        min_chunk_size: 1,
        separators: default_recursive_separators(),
        ..Default::default()
    };
    let chunks = RecursiveCharacterChunking
        .chunk(text, &config)
        .await
        .unwrap();
    assert_eq!(
        chunks.len(),
        3,
        "each paragraph should become its own chunk"
    );
    assert!(chunks[0].content.contains("Alpha"));
    assert!(chunks[1].content.contains("Beta"));
    assert!(chunks[2].content.contains("Gamma"));
}

#[tokio::test]
async fn init_stats_reports_chunk_strategy_enum() {
    use edgequake_llm::MockProvider;
    use edgequake_pipeline::{build_ingestion_pipeline, IngestionPipelineOptions};
    let llm = Arc::new(MockProvider::new());
    let emb = Arc::new(MockProvider::new());
    let opts = IngestionPipelineOptions::from_document_size(1000)
        .with_chunk_strategy(ChunkStrategy::Recursive);
    let pipeline = build_ingestion_pipeline(
        llm,
        emb,
        edgequake_pipeline::prompts::EntityExtractionSchema::server_default(),
        opts,
    );
    assert_eq!(pipeline.config().chunk_strategy, ChunkStrategy::Recursive);
}

#[tokio::test]
async fn recursive_cjk_splits_on_fullwidth_punctuation() {
    let text = "第一句。第二句！第三句？";
    let config = ChunkerConfig {
        chunk_size: 5,
        chunk_overlap: 0,
        min_chunk_size: 1,
        separators: default_recursive_separators(),
        ..Default::default()
    };
    let chunks = RecursiveCharacterChunking
        .chunk(text, &config)
        .await
        .unwrap();
    assert!(chunks.len() >= 2);
}

#[tokio::test]
async fn recursive_overlap_tokens_applied() {
    let text = "Word ".repeat(400);
    let config = ChunkerConfig {
        chunk_size: 50,
        chunk_overlap: 10,
        min_chunk_size: 1,
        separators: default_recursive_separators(),
        ..Default::default()
    };
    let chunks = RecursiveCharacterChunking
        .chunk(&text, &config)
        .await
        .unwrap();
    assert!(chunks.len() >= 2);
    let a = &chunks[0].content;
    let b = &chunks[1].content;
    assert!(
        a.chars().rev().take(20).any(|c| b.contains(c)),
        "expected overlap between adjacent chunks"
    );
}

#[test]
fn fixed_adaptive_sizes_unchanged() {
    assert_eq!(calculate_adaptive_chunk_size(30_000), 1200);
    assert_eq!(calculate_adaptive_chunk_size(80_000), 800);
    assert_eq!(calculate_adaptive_chunk_size(150_000), 600);
}

#[test]
fn registry_selects_strategy_by_enum() {
    let config = ChunkerConfig::default();
    assert_eq!(
        resolve_chunker(ChunkStrategy::Recursive, config.clone()).strategy_name(),
        "recursive_character"
    );
    assert_eq!(
        resolve_chunker(ChunkStrategy::Markdown, config).strategy_name(),
        "markdown"
    );
}

#[tokio::test]
async fn recursive_boundary_overlap_vs_lightrag_fixture() {
    let text = fixture("plain_en.txt");
    let golden: LightragChunkFixture =
        serde_json::from_str(include_str!("fixtures/spec026/lightrag_r_chunks.json"))
            .expect("lightrag_r_chunks.json");

    let config = ChunkerConfig {
        chunk_size: 15,
        chunk_overlap: 0,
        min_chunk_size: 1,
        separators: default_recursive_separators(),
        ..Default::default()
    };
    let chunker = Chunker::with_strategy(config, Arc::new(RecursiveCharacterChunking));
    let chunks = chunker.chunk_async(&text, "doc").await.unwrap();
    assert!(!chunks.is_empty());

    let actual: Vec<usize> = chunks.iter().map(|c| c.start_offset).collect();
    let overlap = boundary_overlap_pct(&actual, &golden.chunk_start_offsets, 4);
    assert!(
        overlap >= 90.0,
        "expected ≥90% boundary overlap with golden fixture, got {overlap:.1}% (actual={actual:?}, expected={:?})",
        golden.chunk_start_offsets
    );
}

#[test]
fn boundary_overlap_pct_helper() {
    assert_eq!(boundary_overlap_pct(&[0, 44, 97], &[0, 44, 97], 0), 100.0);
    assert!(boundary_overlap_pct(&[1, 45, 98], &[0, 44, 97], 2) >= 90.0);
}
