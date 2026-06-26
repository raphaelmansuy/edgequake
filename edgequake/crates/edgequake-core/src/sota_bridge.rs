//! Bridge between core query types and `edgequake-query` SOTA engine (SPEC-017).
//!
//! WHY: Orchestrator and public API must share one query implementation.
//! Core owns stable `QueryParams` / `QueryResult`; SOTA owns retrieval logic.
//! `QueryMode` is canonical in `edgequake-query` and re-exported by core types.

use crate::error::{Error, Result};
use crate::types::{
    ContextChunk, ContextEntity, ContextRelationship, QueryContext, QueryMode, QueryParams,
    QueryResult, QueryStats,
};
use edgequake_query::engine::{QueryRequest, QueryResponse};
use edgequake_query::QueryEngine;

/// Convert core query params to SOTA request.
pub fn params_to_request(query: &str, params: &QueryParams) -> QueryRequest {
    let mut request = QueryRequest::new(query);
    request.mode = Some(params.mode);
    request.context_only = params.only_need_context;
    request.prompt_only = params.only_need_prompt;
    request.max_results = Some(params.top_k);

    if params.enable_history {
        if let Some(history) = &params.history_context {
            request
                .conversation_history
                .push(edgequake_query::engine::ConversationMessage {
                    role: "user".to_string(),
                    content: history.clone(),
                });
        }
    }

    if let Some(tenant_id) = &params.tenant_id {
        request
            .params
            .insert("tenant_id".to_string(), serde_json::json!(tenant_id));
    }
    if let Some(workspace_id) = &params.workspace_id {
        request
            .params
            .insert("workspace_id".to_string(), serde_json::json!(workspace_id));
    }

    request
}

/// Execute a core-style query via SOTA engine (Bypass handled by pipeline).
pub async fn query_via_sota(
    engine: &QueryEngine,
    llm: &std::sync::Arc<dyn edgequake_llm::traits::LLMProvider>,
    query: &str,
    params: QueryParams,
) -> Result<QueryResult> {
    // Preserve legacy bypass prompt for orchestrator callers (direct LLM, no RAG template).
    if params.mode.is_bypass() {
        return query_bypass(llm, query, &params).await;
    }

    let request = params_to_request(query, &params);
    let response = engine
        .query(request)
        .await
        .map_err(|e| Error::internal(format!("Query failed: {}", e)))?;
    Ok(sota_response_to_core(response))
}

async fn query_bypass(
    llm: &std::sync::Arc<dyn edgequake_llm::traits::LLMProvider>,
    query: &str,
    _params: &QueryParams,
) -> Result<QueryResult> {
    let generation_start = std::time::Instant::now();
    let prompt = format!(
        "Answer the following question to the best of your ability.\n\nQuestion: {}\n\nAnswer:",
        query
    );
    let response = llm
        .complete(&prompt)
        .await
        .map_err(|e| Error::internal(format!("LLM error: {}", e)))?;
    Ok(QueryResult {
        response: response.content,
        mode: QueryMode::Bypass,
        context: QueryContext::default(),
        stats: QueryStats {
            generation_time_ms: generation_start.elapsed().as_millis() as u64,
            prompt_tokens: response.prompt_tokens,
            response_tokens: response.completion_tokens,
            ..Default::default()
        },
    })
}

fn sota_response_to_core(response: QueryResponse) -> QueryResult {
    let QueryResponse {
        answer,
        context,
        mode,
        stats,
    } = response;

    let entities_len = context.entities.len();
    let relationships_len = context.relationships.len();
    let chunks_len = context.chunks.len();

    QueryResult {
        response: answer,
        mode,
        context: QueryContext {
            entities: context
                .entities
                .into_iter()
                .map(|e| ContextEntity {
                    name: e.name,
                    entity_type: e.entity_type,
                    description: e.description,
                    score: e.score,
                })
                .collect(),
            relationships: context
                .relationships
                .into_iter()
                .map(|r| ContextRelationship {
                    source: r.source,
                    target: r.target,
                    relation_type: r.relation_type,
                    description: r.description,
                    score: r.score,
                })
                .collect(),
            chunks: context
                .chunks
                .into_iter()
                .map(|c| ContextChunk {
                    chunk_id: c.id,
                    document_id: c.document_id.unwrap_or_default(),
                    content: c.content,
                    score: c.score,
                    start_line: c.start_line,
                    end_line: c.end_line,
                    chunk_index: c.chunk_index,
                })
                .collect(),
        },
        stats: QueryStats {
            retrieval_time_ms: stats.retrieval_time_ms,
            generation_time_ms: stats.generation_time_ms,
            total_time_ms: stats.total_time_ms,
            entities_retrieved: entities_len,
            relationships_retrieved: relationships_len,
            chunks_retrieved: chunks_len,
            prompt_tokens: stats.context_tokens,
            response_tokens: stats.generated_tokens,
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_to_request_tenant_workspace() {
        let params = QueryParams {
            tenant_id: Some("t1".to_string()),
            workspace_id: Some("w1".to_string()),
            mode: QueryMode::Hybrid,
            ..Default::default()
        };
        let req = params_to_request("hello", &params);
        assert_eq!(
            req.params.get("tenant_id").unwrap(),
            &serde_json::json!("t1")
        );
        assert_eq!(
            req.params.get("workspace_id").unwrap(),
            &serde_json::json!("w1")
        );
        assert_eq!(req.mode, Some(QueryMode::Hybrid));
    }

    #[test]
    fn bypass_mode_parsed_on_request() {
        let params = QueryParams {
            mode: QueryMode::Bypass,
            ..Default::default()
        };
        let req = params_to_request("hello", &params);
        assert_eq!(req.mode, Some(QueryMode::Bypass));
        assert!(req.mode.unwrap().is_bypass());
    }
}
