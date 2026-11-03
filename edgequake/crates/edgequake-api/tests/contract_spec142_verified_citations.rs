//! SPEC-142 — verified citation links (document name + page) are retrieval-grounded.
//!
//! Unfakable: scripted answer + catalog; no live LLM. Fails if href page ≠ stored page.

use edgequake_api::handlers::query_types::SourceReference;
use edgequake_api::services::verified_citations::{
    apply_verified_citations, catalog_from_sources, verified_answer,
};
use edgequake_query::{
    is_gold_answer_extension, rewrite_verified_citations, strip_gold_citation_artifacts,
    CitationCatalog, CitationEntry,
};

fn chunk_source(
    ref_id: usize,
    page: u32,
    name: &str,
    doc_id: &str,
    chunk_id: &str,
) -> SourceReference {
    SourceReference {
        source_type: "chunk".into(),
        id: chunk_id.into(),
        score: 0.91,
        similarity: None,
        rerank_score: None,
        snippet: Some("UNIQUE_MARKER_PAGE_4 evidence string".into()),
        reference_id: Some(ref_id),
        document_id: Some(doc_id.into()),
        file_path: Some(name.into()),
        start_line: None,
        end_line: None,
        chunk_index: Some(3),
        page_start: Some(page),
        page_end: Some(page),
        entity_type: None,
        degree: None,
        source_chunk_ids: None,
        doc_kind: None,
        mime_type: None,
        source_modality: None,
    }
}

#[test]
fn http142_01_query_answer_href_matches_fixture_page() {
    let raw = "The unique marker is present [1]. See also [99] and page 999.";
    let sources = vec![chunk_source(
        1,
        4,
        "Fixture.pdf",
        "cccccccc-0142-0142-0142-cccccccccccc",
        "span-chunk-4",
    )];

    let out = verified_answer(raw, &sources);
    assert!(
        out.contains(
            "[p.4](/documents/cccccccc-0142-0142-0142-cccccccccccc?chunk=span-chunk-4&page=4 \"Fixture.pdf\")"
        ),
        "verified link missing: {out}"
    );
    assert!(
        !out.contains("[99]"),
        "hallucinated cite must be stripped: {out}"
    );
    assert!(
        !out.contains("page=999"),
        "hallucinated page must not appear in href: {out}"
    );
    assert!(
        !out.contains("page 999"),
        "LAW-142-11 prose page scrub: {out}"
    );
    assert_eq!(sources[0].page_start, Some(4));
    assert_eq!(sources[0].file_path.as_deref(), Some("Fixture.pdf"));
}

#[test]
fn http142_02_stream_done_equiv_sync_rewrite() {
    let raw = "Fact [1][2].";
    let sources = vec![
        chunk_source(1, 4, "Fixture.pdf", "doc-a", "c1"),
        chunk_source(2, 3, "Fixture.pdf", "doc-a", "c2"),
    ];
    let sync = verified_answer(raw, &sources);
    let stream_done = apply_verified_citations(raw, &sources).text;
    assert_eq!(sync, stream_done);
    assert!(sync.contains("page=4"));
    assert!(sync.contains("page=3"));
    assert!(sync.contains("[p.4]("));
}

#[test]
fn http142_03_chat_persist_shape_keeps_href() {
    let verified = verified_answer(
        "Answer [1]",
        &[chunk_source(1, 4, "Report.pdf", "doc-1", "chunk-1")],
    );
    assert!(verified.contains("[p.4]("));
    assert!(verified.contains("\"Report.pdf\""));
    assert!(verified.contains("?chunk=chunk-1&page=4"));
}

#[test]
fn mcp142_sources_catalog_same_markdown_as_http() {
    let sources = vec![chunk_source(1, 4, "Fixture.pdf", "doc-mcp", "c-mcp")];
    let catalog = catalog_from_sources(&sources);
    let from_catalog = rewrite_verified_citations("x [1]", &catalog).text;
    let from_http = verified_answer("x [1]", &sources);
    assert_eq!(from_catalog, from_http);
    assert!(from_http.contains("/documents/doc-mcp?chunk=c-mcp&page=4"));
}

#[test]
fn acc_gold_path_skips_markers() {
    assert!(is_gold_answer_extension(Some(
        "Do NOT append citation markers. Plain answer text only."
    )));
    let cleaned = strip_gold_citation_artifacts("Answer [1] ### References\nfoo");
    assert!(!cleaned.contains("[1]"));
    assert!(!cleaned.contains("References"));
}

#[test]
fn non_pdf_omits_page_param() {
    let mut catalog = CitationCatalog::new();
    catalog.insert(CitationEntry {
        reference_id: 1,
        chunk_id: "c".into(),
        document_id: "d".into(),
        document_name: "notes.md".into(),
        page_start: None,
        page_end: None,
    });
    let out = rewrite_verified_citations("See [1]", &catalog).text;
    assert!(
        out.contains("[notes](/documents/d?chunk=c \"notes.md\")"),
        "got: {out}"
    );
    assert!(!out.contains("page="));
}
