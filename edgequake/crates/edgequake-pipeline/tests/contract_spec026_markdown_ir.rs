//! SPEC-026 Phase 2 — markdown IR + section context contract tests.

use std::sync::Arc;

use edgequake_pipeline::prompts::{json_extraction_prompt, EntityExtractionSchema};
use edgequake_pipeline::{
    extract_markdown_blocks, format_breadcrumb, format_section_context, text_with_section_context,
    truncate_section_context, Chunker, ChunkerConfig, ChunkingStrategy, MarkdownChunking,
    SectionMetadata,
};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/spec026/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn markdown_ir_builds_heading_stack() {
    let md = "# Install\n\nBody one.\n\n## Prerequisites\n\nBody two.";
    let blocks = extract_markdown_blocks(md);
    let prereq = blocks
        .iter()
        .find(|b| b.heading == "Prerequisites")
        .unwrap();
    assert_eq!(prereq.parent_headings, vec!["Install"]);
}

#[test]
fn markdown_ir_resets_stack_on_h1() {
    let md = "# A\n\n## B\n\n# C\n\nText under C.";
    let blocks = extract_markdown_blocks(md);
    let c_block = blocks.iter().find(|b| b.heading == "C").unwrap();
    assert!(c_block.parent_headings.is_empty());
}

#[tokio::test]
async fn markdown_chunking_splits_at_headings() {
    let md = fixture("structured_manual.md");
    let config = ChunkerConfig {
        chunk_size: 200,
        chunk_overlap: 10,
        ..Default::default()
    };
    let chunks = MarkdownChunking.chunk(&md, &config).await.unwrap();
    assert!(chunks.len() >= 2);
}

#[tokio::test]
async fn markdown_chunk_carries_section_metadata() {
    let md = fixture("structured_manual.md");
    let config = ChunkerConfig {
        chunk_size: 400,
        chunk_overlap: 10,
        ..Default::default()
    };
    let chunker = Chunker::with_strategy(config, Arc::new(MarkdownChunking));
    let chunks = chunker.chunk_async(&md, "doc").await.unwrap();
    let advanced = chunks.iter().find(|c| {
        c.section
            .as_ref()
            .is_some_and(|s| s.heading_path.iter().any(|h| h == "Advanced"))
    });
    assert!(
        advanced.is_some(),
        "expected Advanced section chunk, got paths: {:?}",
        chunks
            .iter()
            .filter_map(|c| c.section.as_ref())
            .map(|s| &s.heading_path)
            .collect::<Vec<_>>()
    );
}

#[test]
fn section_context_format_matches_lightrag() {
    let section = SectionMetadata {
        heading_path: vec!["Methods".into(), "Data Collection".into()],
        heading_level: 2,
    };
    let block = format_section_context(Some(&section));
    assert!(block.contains("---Section Context---"));
    assert!(block.contains("Methods → Data Collection"));
}

#[test]
fn section_context_truncates_long_paths() {
    let long = "A".repeat(200);
    let path = format!("{long} → {long} → Leaf");
    let truncated = truncate_section_context(&path, 32);
    assert!(truncated.len() < path.len());
}

#[test]
fn breadcrumb_format() {
    assert_eq!(
        format_breadcrumb(&["Install".into()], "Prerequisites"),
        "Install → Prerequisites"
    );
}

#[test]
fn extractor_prompt_includes_section_when_present() {
    let section = SectionMetadata {
        heading_path: vec!["Guide".into()],
        heading_level: 1,
    };
    let text = text_with_section_context("Body text.", Some(&section));
    let prompt = json_extraction_prompt(&text, &EntityExtractionSchema::server_default());
    assert!(prompt.contains("---Section Context---"));
    assert!(prompt.contains("Guide"));
}
