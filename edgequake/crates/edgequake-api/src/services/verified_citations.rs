//! SPEC-142 / P0.5: apply verified citation rewrite using retrieval sources as catalog.
//!
//! DRY: one SSOT for Query sync, stream Done, and Chat persist.
//! LAW-142-13: emit rewrite telemetry on every product apply.

use edgequake_query::{
    rewrite_verified_citations, CitationCatalog, CitationSourceRow, RewriteReport,
};

use crate::handlers::query_types::SourceReference;

/// Build a citation catalog from enriched source references (after title resolve).
pub fn catalog_from_sources(sources: &[SourceReference]) -> CitationCatalog {
    let rows: Vec<CitationSourceRow<'_>> = sources
        .iter()
        .map(|s| CitationSourceRow {
            source_type: s.source_type.as_str(),
            chunk_id: s.id.as_str(),
            reference_id: s.reference_id,
            document_id: s.document_id.as_deref(),
            document_name: s.file_path.as_deref(),
            page_start: s.page_start,
            page_end: s.page_end,
        })
        .collect();
    CitationCatalog::from_source_rows(&rows)
}

/// Rewrite answer markdown with verified document+page links and prose scrub.
///
/// [`rewrite_verified_citations`] normalizes math spans before `[N]` rewrite.
/// Empty catalog → identity (math delimiters still restored). Acc gold answers
/// typically have no `[N]` after strip. Records LAW-142-13 Prometheus / tracing
/// counters.
pub fn apply_verified_citations(answer: &str, sources: &[SourceReference]) -> RewriteReport {
    let catalog = catalog_from_sources(sources);
    let report = rewrite_verified_citations(answer, &catalog);
    record_rewrite_observability(&report);
    report
}

/// Convenience: return rewritten text only.
pub fn verified_answer(answer: &str, sources: &[SourceReference]) -> String {
    apply_verified_citations(answer, sources).text
}

fn record_rewrite_observability(report: &RewriteReport) {
    edgequake_observability::record_citation_rewrite(
        report.rewritten_ids.len() as u64,
        report.stripped_ids.len() as u64,
        report.prose_pages_stripped as u64,
        report.citation_validity(),
        report.uncited_sentence_ratio,
    );
    tracing::debug!(
        rewritten = report.rewritten_ids.len(),
        stripped_unknown = report.stripped_ids.len(),
        prose_pages_stripped = report.prose_pages_stripped,
        unique_document_ids = report.unique_document_ids,
        citation_validity = ?report.citation_validity(),
        uncited_sentence_ratio = ?report.uncited_sentence_ratio,
        "SPEC-142 citation rewrite applied"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk_source(ref_id: usize, page: u32) -> SourceReference {
        SourceReference {
            source_type: "chunk".into(),
            id: "chunk-a".into(),
            score: 0.9,
            rerank_score: None,
            snippet: Some("evidence".into()),
            reference_id: Some(ref_id),
            document_id: Some("doc-1".into()),
            file_path: Some("Fixture.pdf".into()),
            start_line: None,
            end_line: None,
            chunk_index: Some(0),
            page_start: Some(page),
            page_end: Some(page),
            entity_type: None,
            degree: None,
            source_chunk_ids: None,
        }
    }

    #[test]
    fn rewrites_from_sources() {
        let sources = vec![chunk_source(1, 4)];
        let out = verified_answer("Answer [1] and [99]. See page 999.", &sources);
        assert!(
            out.contains("[p.4](/documents/doc-1?chunk=chunk-a&page=4 \"Fixture.pdf\")"),
            "got: {out}"
        );
        assert!(out.contains("\"Fixture.pdf\""));
        assert!(!out.contains("[99]"));
        assert!(!out.contains("page=99"));
        assert!(!out.contains("page 999"), "prose scrub: {out}");
    }
}
