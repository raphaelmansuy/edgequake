//! Build flat `SourceReference` citations from engine `QueryContext` (SPEC-028 DRY SSOT).

use edgequake_query::QueryContext;

use crate::handlers::context_types::ContentGranularity;
use crate::handlers::query_types::SourceReference;
use crate::services::truncate_for_granularity;

/// Returns true when a document/file path indicates knowledge injection (not citable).
pub fn is_injection_source(document_id: Option<&str>, file_path: Option<&str>) -> bool {
    if document_id.unwrap_or("").starts_with("injection::") {
        return true;
    }
    if file_path == Some("injection") {
        return true;
    }
    false
}

/// Map engine context to flat citation sources (chunks → entities → relationships).
pub fn build_sources_from_context(
    context: &QueryContext,
    include_reference_ids: bool,
    rerank_top_k: Option<usize>,
    reranked: bool,
    granularity: ContentGranularity,
) -> Vec<SourceReference> {
    let mut sources = Vec::new();
    // X-20: continue entity/relationship refs after the highest chunk citation_id.
    let mut ref_counter = 1usize;

    // 026/034: prefer citation_chunks (Mix∪CE) for L2/sources when dual-list is set.
    // Dual-list owns its own budget — do NOT re-truncate to rerank_top_k, or Mix fill
    // after a full CE[:30] set is silently dropped (026 S0 Fact ER flat root cause).
    let dual_list = context.citation_chunks.is_some();
    let chunk_iter = context
        .citation_chunks
        .as_deref()
        .unwrap_or(context.chunks.as_slice());

    let mut chunk_sources: Vec<SourceReference> = chunk_iter
        .iter()
        .filter(|chunk| !is_injection_source(chunk.document_id.as_deref(), None))
        .map(|chunk| {
            let ref_id = if include_reference_ids {
                // X-20: stable citation_id stamped before format — never renumber by position.
                if let Some(id) = chunk.citation_id {
                    ref_counter = ref_counter.max(id + 1);
                    Some(id)
                } else {
                    let id = ref_counter;
                    ref_counter += 1;
                    Some(id)
                }
            } else {
                None
            };

            SourceReference {
                source_type: "chunk".to_string(),
                id: chunk.id.clone(),
                score: chunk.score,
                rerank_score: None,
                snippet: Some(truncate_for_granularity(&chunk.content, granularity)),
                reference_id: ref_id,
                document_id: chunk.document_id.clone(),
                file_path: None,
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                chunk_index: chunk.chunk_index,
                page_start: chunk.page_start,
                page_end: chunk.page_end,
                entity_type: None,
                degree: None,
                source_chunk_ids: None,
            }
        })
        .collect();

    if reranked && !dual_list {
        if let Some(top_k) = rerank_top_k {
            chunk_sources.truncate(top_k);
        }
    }

    sources.extend(chunk_sources);

    for entity in &context.entities {
        if is_injection_source(
            entity.source_document_id.as_deref(),
            entity.source_file_path.as_deref(),
        ) {
            continue;
        }

        let ref_id = if include_reference_ids {
            let id = ref_counter;
            ref_counter += 1;
            Some(id)
        } else {
            None
        };

        sources.push(SourceReference {
            source_type: "entity".to_string(),
            id: entity.name.clone(),
            score: entity.score,
            rerank_score: None,
            snippet: Some(truncate_for_granularity(&entity.description, granularity)),
            reference_id: ref_id,
            document_id: entity.source_document_id.clone(),
            file_path: entity.source_file_path.clone(),
            start_line: None,
            end_line: None,
            chunk_index: None,
            page_start: None,
            page_end: None,
            entity_type: Some(entity.entity_type.clone()),
            degree: if entity.degree > 0 {
                Some(entity.degree)
            } else {
                None
            },
            source_chunk_ids: if entity.source_chunk_ids.is_empty() {
                None
            } else {
                Some(entity.source_chunk_ids.clone())
            },
        });
    }

    for rel in &context.relationships {
        if is_injection_source(
            rel.source_document_id.as_deref(),
            rel.source_file_path.as_deref(),
        ) {
            continue;
        }

        let ref_id = if include_reference_ids {
            let id = ref_counter;
            ref_counter += 1;
            Some(id)
        } else {
            None
        };

        sources.push(SourceReference {
            source_type: "relationship".to_string(),
            // Keep identity in id for dedupe; snippet uses presentation labels (073).
            id: format!("{}->{}", rel.source, rel.target),
            score: rel.score,
            rerank_score: None,
            snippet: Some(format!(
                "{} {} {}",
                rel.display_source(),
                rel.relation_type,
                rel.display_target()
            )),
            reference_id: ref_id,
            document_id: rel.source_document_id.clone(),
            file_path: rel.source_file_path.clone(),
            start_line: None,
            end_line: None,
            chunk_index: None,
            page_start: None,
            page_end: None,
            entity_type: None,
            degree: None,
            source_chunk_ids: None,
        });
    }

    sources
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_query::context::{RetrievedChunk, RetrievedEntity};

    #[test]
    fn excludes_injection_chunks() {
        let mut ctx = QueryContext::default();
        ctx.chunks.push(RetrievedChunk {
            id: "c1".into(),
            content: "secret injection".into(),
            score: 1.0,
            document_id: Some("injection::ws::1".into()),
            document_name: None,
            token_count: 3,
            start_line: None,
            end_line: None,
            chunk_index: None,
            page_start: None,
            page_end: None,
            modality: None,
            source_modality: None,
            citation_id: None,
        });
        ctx.chunks.push(RetrievedChunk {
            id: "c2".into(),
            content: "real doc".into(),
            score: 0.9,
            document_id: Some("doc-1".into()),
            document_name: None,
            token_count: 2,
            start_line: None,
            end_line: None,
            chunk_index: None,
            page_start: None,
            page_end: None,
            modality: None,
            source_modality: None,
            citation_id: None,
        });

        let sources =
            build_sources_from_context(&ctx, true, None, false, ContentGranularity::Citation);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, "c2");
    }

    #[test]
    fn include_reference_ids_respected() {
        let mut ctx = QueryContext::default();
        ctx.entities.push(RetrievedEntity {
            name: "FOO".into(),
            entity_type: "CONCEPT".into(),
            description: "desc".into(),
            score: 0.5,
            degree: 2,
            source_chunk_ids: vec![],
            source_document_id: None,
            source_file_path: None,
            source_document_ids: vec![],
        });

        let with_refs =
            build_sources_from_context(&ctx, true, None, false, ContentGranularity::Citation);
        assert_eq!(with_refs[0].reference_id, Some(1));

        let without =
            build_sources_from_context(&ctx, false, None, false, ContentGranularity::Citation);
        assert!(without[0].reference_id.is_none());
    }

    #[test]
    fn prefers_citation_chunks_for_sources() {
        let mut ctx = QueryContext::default();
        ctx.chunks.push(RetrievedChunk {
            id: "ce-only".into(),
            content: "prompt chunk".into(),
            score: 0.9,
            document_id: Some("doc-1".into()),
            document_name: None,
            token_count: 2,
            start_line: None,
            end_line: None,
            chunk_index: None,
            page_start: None,
            page_end: None,
            modality: None,
            source_modality: None,
            citation_id: None,
        });
        ctx.citation_chunks = Some(vec![
            RetrievedChunk {
                id: "ce-only".into(),
                content: "prompt chunk".into(),
                score: 0.9,
                document_id: Some("doc-1".into()),
                document_name: None,
                token_count: 2,
                start_line: None,
                end_line: None,
                chunk_index: None,
                page_start: None,
                page_end: None,
                modality: None,
                source_modality: None,
                citation_id: None,
            },
            RetrievedChunk {
                id: "mix-only".into(),
                content: "mix gold".into(),
                score: 0.8,
                document_id: Some("doc-2".into()),
                document_name: None,
                token_count: 2,
                start_line: None,
                end_line: None,
                chunk_index: None,
                page_start: None,
                page_end: None,
                modality: None,
                source_modality: None,
                citation_id: None,
            },
        ]);

        let sources =
            build_sources_from_context(&ctx, true, None, false, ContentGranularity::Agent);
        let ids: Vec<&str> = sources.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["ce-only", "mix-only"]);
    }

    #[test]
    fn dual_list_survives_rerank_top_k_truncate() {
        // 034: Acc path passes reranked=true + top_k=30. If CE already fills 30,
        // Mix∪CE fill must still appear in API sources for L2 evidence_recall.
        let mut ctx = QueryContext::default();
        let mut citation = Vec::new();
        for i in 0..30 {
            citation.push(RetrievedChunk {
                id: format!("ce-{i}"),
                content: format!("ce {i}"),
                score: 1.0 - (i as f32) * 0.01,
                document_id: Some("doc-ce".into()),
                document_name: None,
                token_count: 2,
                start_line: None,
                end_line: None,
                chunk_index: None,
                page_start: None,
                page_end: None,
                modality: None,
                source_modality: None,
                citation_id: None,
            });
        }
        citation.push(RetrievedChunk {
            id: "mix-gold".into(),
            content: "mix gold evidence".into(),
            score: 0.5,
            document_id: Some("doc-mix".into()),
            document_name: None,
            token_count: 2,
            start_line: None,
            end_line: None,
            chunk_index: None,
            page_start: None,
            page_end: None,
            modality: None,
            source_modality: None,
            citation_id: None,
        });
        ctx.chunks = citation[..30].to_vec();
        ctx.citation_chunks = Some(citation);

        let sources =
            build_sources_from_context(&ctx, true, Some(30), true, ContentGranularity::Citation);
        let ids: Vec<&str> = sources
            .iter()
            .filter(|s| s.source_type == "chunk")
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(ids.len(), 31);
        assert_eq!(ids[30], "mix-gold");
    }

    #[test]
    fn citation_truncates_chunk_snippet() {
        let mut ctx = QueryContext::default();
        ctx.chunks.push(RetrievedChunk {
            id: "c1".into(),
            content: "a".repeat(500),
            score: 0.9,
            document_id: Some("doc-1".into()),
            document_name: None,
            token_count: 100,
            start_line: None,
            end_line: None,
            chunk_index: None,
            page_start: None,
            page_end: None,
            modality: None,
            source_modality: None,
            citation_id: None,
        });

        let citation =
            build_sources_from_context(&ctx, true, None, false, ContentGranularity::Citation);
        assert_eq!(citation[0].snippet.as_ref().unwrap().len(), 200);

        let agent = build_sources_from_context(&ctx, true, None, false, ContentGranularity::Agent);
        assert_eq!(agent[0].snippet.as_ref().unwrap().len(), 500);
    }

    #[test]
    fn contract_citation_stable_ids() {
        let mut ctx = QueryContext::default();
        ctx.chunks.push(
            RetrievedChunk::new("a", "first", 0.9)
                .with_document_id("d1")
                .with_citation_id(7),
        );
        ctx.chunks.push(
            RetrievedChunk::new("b", "second", 0.8)
                .with_document_id("d1")
                .with_citation_id(3),
        );
        // Reorder relative to citation_id sequence — refs must follow stamped ids.
        let sources =
            build_sources_from_context(&ctx, true, None, false, ContentGranularity::Citation);
        assert_eq!(sources[0].reference_id, Some(7));
        assert_eq!(sources[1].reference_id, Some(3));
    }
}
