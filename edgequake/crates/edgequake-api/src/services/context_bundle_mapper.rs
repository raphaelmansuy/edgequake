//! Map engine `QueryContext` → agent-grade `ContextBundle` (SPEC-028).

use std::collections::{HashMap, HashSet};

use edgequake_query::{QueryContext, QueryResponse};

use crate::handlers::context_types::{
    AgentHints, ChunkLineage, ContentGranularity, ContextBundle, ContextChunk,
    ContextDocumentSummary, ContextEntity, ContextRelationship, ContextRetrievalStats,
    EntityLineage, ItemsRetrieved, RelationshipLineage, RetrievalQuality, SubgraphBundle,
    TruncationInfo,
};
use crate::services::source_reference_builder::is_injection_source;

use crate::services::truncate_for_granularity;

const DEFAULT_TOKEN_BUDGET: usize = 30_000;
const COVERAGE_SUFFICIENT_THRESHOLD: f32 = 0.35;

/// Options controlling bundle mapping.
#[derive(Debug, Clone)]
pub struct MappingOptions {
    pub granularity: ContentGranularity,
    pub include_lineage: bool,
    pub include_documents: bool,
    pub include_agent_hints: bool,
    /// When false, subgraph entities/relationships are omitted (chunks-only payload).
    pub include_subgraph: bool,
    pub rerank_top_k: Option<usize>,
    pub reranked: bool,
}

impl Default for MappingOptions {
    fn default() -> Self {
        Self {
            granularity: ContentGranularity::Agent,
            include_lineage: true,
            include_documents: true,
            include_agent_hints: true,
            include_subgraph: true,
            rerank_top_k: None,
            reranked: false,
        }
    }
}

pub struct DocumentMeta {
    pub title: String,
    pub mime_type: Option<String>,
    pub created_at: Option<String>,
}

pub fn map_engine_response_to_bundle(
    result: &QueryResponse,
    options: &MappingOptions,
    document_titles: &HashMap<String, DocumentMeta>,
) -> ContextBundle {
    map_query_context_to_bundle(&result.context, options, document_titles)
}

pub fn map_query_context_to_bundle(
    context: &QueryContext,
    options: &MappingOptions,
    document_titles: &HashMap<String, DocumentMeta>,
) -> ContextBundle {
    let mut ref_counter = 1usize;

    let mut chunks: Vec<ContextChunk> = context
        .chunks
        .iter()
        .filter(|c| !is_injection_source(c.document_id.as_deref(), None))
        .map(|chunk| {
            let content = truncate_for_granularity(&chunk.content, options.granularity);
            let reference_id = {
                let id = ref_counter;
                ref_counter += 1;
                Some(id)
            };

            let lineage = if options.include_lineage {
                Some(ChunkLineage {
                    document_id: chunk.document_id.clone(),
                    file_path: chunk
                        .document_id
                        .as_ref()
                        .and_then(|id| document_titles.get(id).map(|m| m.title.clone())),
                    start_line: chunk.start_line,
                    end_line: chunk.end_line,
                    chunk_index: chunk.chunk_index,
                })
            } else {
                None
            };

            ContextChunk {
                id: chunk.id.clone(),
                content,
                score: chunk.score,
                rerank_score: None,
                token_count: chunk.token_count,
                reference_id,
                is_truncated: None,
                lineage,
            }
        })
        .collect();

    if options.reranked {
        if let Some(top_k) = options.rerank_top_k {
            chunks.truncate(top_k);
        }
    }

    let subgraph = map_query_context_to_subgraph(context, options);

    let documents = if options.include_documents {
        build_document_summaries(&chunks, &subgraph.entities, document_titles)
    } else {
        Vec::new()
    };

    let context_string = if options.granularity == ContentGranularity::Debug {
        Some(context.to_context_string())
    } else {
        None
    };

    ContextBundle {
        subgraph,
        chunks,
        documents,
        context_string,
    }
}

/// Map engine context to structured query-matched subgraph (SPEC-028 / FP-028-09).
pub fn map_query_context_to_subgraph(
    context: &QueryContext,
    options: &MappingOptions,
) -> SubgraphBundle {
    if !options.include_subgraph {
        return SubgraphBundle::default();
    }

    let entities: Vec<ContextEntity> = context
        .entities
        .iter()
        .filter(|e| {
            !is_injection_source(
                e.source_document_id.as_deref(),
                e.source_file_path.as_deref(),
            )
        })
        .map(|entity| {
            let description = truncate_for_granularity(&entity.description, options.granularity);

            let lineage = if options.include_lineage {
                Some(EntityLineage {
                    source_chunk_ids: entity.source_chunk_ids.clone(),
                    source_document_id: entity.source_document_id.clone(),
                    source_file_path: entity.source_file_path.clone(),
                })
            } else {
                None
            };

            ContextEntity {
                id: format!("ent:{}", entity.name),
                name: entity.name.clone(),
                entity_type: entity.entity_type.clone(),
                description,
                score: entity.score,
                degree: entity.degree,
                lineage,
            }
        })
        .collect();

    let relationships: Vec<ContextRelationship> = context
        .relationships
        .iter()
        .filter(|r| {
            !is_injection_source(
                r.source_document_id.as_deref(),
                r.source_file_path.as_deref(),
            )
        })
        .map(|rel| {
            let lineage = if options.include_lineage {
                Some(RelationshipLineage {
                    source_chunk_id: rel.source_chunk_id.clone(),
                    source_document_id: rel.source_document_id.clone(),
                    source_file_path: rel.source_file_path.clone(),
                })
            } else {
                None
            };

            ContextRelationship {
                id: format!("rel:{}:{}:{}", rel.source, rel.relation_type, rel.target),
                source: rel.source.clone(),
                target: rel.target.clone(),
                relation_type: rel.relation_type.clone(),
                description: rel.description.clone(),
                score: rel.score,
                lineage,
            }
        })
        .collect();

    SubgraphBundle {
        entities,
        relationships,
    }
}

/// Graph preview for MCP `edgequake_search` metadata (FP-028-09 / DRY SSOT).
pub fn build_search_graph_metadata(bundle: &ContextBundle, mode: &str) -> serde_json::Value {
    use serde_json::json;
    json!({
        "mode": mode,
        "entity_count": bundle.subgraph.entities.len(),
        "relationship_count": bundle.subgraph.relationships.len(),
        "chunk_count": bundle.chunks.len(),
        "document_count": bundle.documents.len(),
        "top_entities": bundle
            .subgraph
            .entities
            .iter()
            .take(5)
            .map(|e| {
                json!({
                    "name": e.name,
                    "entity_type": e.entity_type,
                    "score": e.score,
                    "degree": e.degree,
                })
            })
            .collect::<Vec<_>>(),
        "top_relationships": bundle
            .subgraph
            .relationships
            .iter()
            .take(3)
            .map(|r| {
                json!({
                    "source": r.source,
                    "target": r.target,
                    "relation_type": r.relation_type,
                    "score": r.score,
                })
            })
            .collect::<Vec<_>>(),
    })
}

fn build_document_summaries(
    chunks: &[ContextChunk],
    entities: &[ContextEntity],
    document_titles: &HashMap<String, DocumentMeta>,
) -> Vec<ContextDocumentSummary> {
    let mut doc_chunk_counts: HashMap<String, usize> = HashMap::new();
    let mut doc_entity_counts: HashMap<String, usize> = HashMap::new();

    for chunk in chunks {
        if let Some(ref doc_id) = chunk.lineage.as_ref().and_then(|l| l.document_id.clone()) {
            *doc_chunk_counts.entry(doc_id.clone()).or_insert(0) += 1;
        }
    }

    for entity in entities {
        if let Some(ref doc_id) = entity
            .lineage
            .as_ref()
            .and_then(|l| l.source_document_id.clone())
        {
            *doc_entity_counts.entry(doc_id.clone()).or_insert(0) += 1;
        }
    }

    let all_doc_ids: HashSet<String> = doc_chunk_counts
        .keys()
        .chain(doc_entity_counts.keys())
        .cloned()
        .collect();

    all_doc_ids
        .into_iter()
        .map(|document_id| {
            let meta = document_titles.get(&document_id);
            ContextDocumentSummary {
                document_id: document_id.clone(),
                title: meta
                    .map(|m| m.title.clone())
                    .unwrap_or_else(|| document_id.clone()),
                mime_type: meta.and_then(|m| m.mime_type.clone()),
                created_at: meta.and_then(|m| m.created_at.clone()),
                chunk_count_in_bundle: doc_chunk_counts.get(&document_id).copied().unwrap_or(0),
                entity_count_in_bundle: doc_entity_counts.get(&document_id).copied().unwrap_or(0),
            }
        })
        .collect()
}

pub fn build_retrieval_stats(result: &QueryResponse, reranked: bool) -> ContextRetrievalStats {
    ContextRetrievalStats {
        embedding_time_ms: result.stats.embedding_time_ms,
        retrieval_time_ms: result.stats.retrieval_time_ms,
        rerank_time_ms: result.stats.rerank_time_ms,
        total_time_ms: result.stats.total_time_ms,
        items_retrieved: ItemsRetrieved {
            chunks: result.context.chunks.len(),
            entities: result.context.entities.len(),
            relationships: result.context.relationships.len(),
            documents: count_unique_documents(&result.context),
        },
        keywords_extracted: Vec::new(),
        reranked,
    }
}

fn count_unique_documents(context: &QueryContext) -> usize {
    let mut ids = HashSet::new();
    for chunk in &context.chunks {
        if let Some(ref id) = chunk.document_id {
            ids.insert(id.clone());
        }
    }
    for entity in &context.entities {
        if let Some(ref id) = entity.source_document_id {
            ids.insert(id.clone());
        }
    }
    ids.len()
}

pub fn compute_retrieval_quality(context: &QueryContext) -> RetrievalQuality {
    if context.is_empty() {
        return RetrievalQuality {
            coverage_score: 0.0,
            is_sufficient: false,
            empty_context: true,
        };
    }

    let mut scores: Vec<f32> = context
        .chunks
        .iter()
        .map(|c| c.score)
        .chain(context.entities.iter().map(|e| e.score))
        .chain(context.relationships.iter().map(|r| r.score))
        .collect();

    let coverage_score = if scores.is_empty() {
        0.0
    } else {
        scores.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let top_n = scores.len().min(5);
        scores[..top_n].iter().sum::<f32>() / top_n as f32
    };

    RetrievalQuality {
        coverage_score,
        is_sufficient: coverage_score >= COVERAGE_SUFFICIENT_THRESHOLD,
        empty_context: false,
    }
}

pub fn build_truncation_info(context: &QueryContext) -> TruncationInfo {
    TruncationInfo {
        is_truncated: context.is_truncated,
        token_budget: DEFAULT_TOKEN_BUDGET,
        tokens_used: context.token_count,
        dropped: Default::default(),
    }
}

pub fn build_agent_hints(context: &QueryContext, bundle: &ContextBundle) -> AgentHints {
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for entity in &bundle.subgraph.entities {
        *type_counts.entry(entity.entity_type.clone()).or_insert(0) += 1;
    }

    let mut dominant: Vec<(String, usize)> = type_counts.into_iter().collect();
    dominant.sort_by_key(|b| std::cmp::Reverse(b.1));

    let suggested_followups = if context.is_empty() {
        vec!["Try broadening the query or relaxing document filters.".to_string()]
    } else {
        bundle
            .subgraph
            .entities
            .iter()
            .take(2)
            .map(|e| format!("Tell me more about {} ({})", e.name, e.entity_type))
            .collect()
    };

    AgentHints {
        suggested_followups,
        dominant_entity_types: dominant.into_iter().take(3).map(|(t, _)| t).collect(),
        documents_touched: bundle.documents.len(),
        data_quality_warnings: Vec::new(),
    }
}

pub fn compute_retrieval_fingerprint(
    query: &str,
    mode: &str,
    workspace_id: Option<&str>,
    filter_json: Option<&str>,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    hasher.update(mode.as_bytes());
    if let Some(ws) = workspace_id {
        hasher.update(ws.as_bytes());
    }
    if let Some(f) = filter_json {
        hasher.update(f.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_query::context::RetrievedChunk;
    use edgequake_query::{QueryContext, QueryMode, QueryResponse, QueryStats};

    #[test]
    fn agent_granularity_includes_full_chunk() {
        let mut ctx = QueryContext::default();
        ctx.chunks.push(RetrievedChunk {
            id: "c1".into(),
            content: "x".repeat(500),
            score: 0.9,
            document_id: Some("doc1".into()),
            token_count: 100,
            start_line: None,
            end_line: None,
            chunk_index: None,
            page_start: None,
            page_end: None,
        });

        let bundle = map_query_context_to_bundle(
            &ctx,
            &MappingOptions {
                granularity: ContentGranularity::Agent,
                ..Default::default()
            },
            &HashMap::new(),
        );

        assert_eq!(bundle.chunks[0].content.len(), 500);
    }

    #[test]
    fn empty_context_zero_coverage() {
        let quality = compute_retrieval_quality(&QueryContext::default());
        assert!(quality.empty_context);
    }

    #[test]
    fn fingerprint_is_stable() {
        let a = compute_retrieval_fingerprint("q", "mix", Some("ws"), None);
        let b = compute_retrieval_fingerprint("q", "mix", Some("ws"), None);
        assert_eq!(a, b);
    }

    #[test]
    fn build_stats_from_response() {
        let response = QueryResponse {
            answer: String::new(),
            context: QueryContext::default(),
            mode: QueryMode::Mix,
            stats: QueryStats {
                embedding_time_ms: 10,
                retrieval_time_ms: 20,
                ..Default::default()
            },
        };
        assert_eq!(
            build_retrieval_stats(&response, false).embedding_time_ms,
            10
        );
    }
}
