//! Shared engine `QueryRequest` builder (SPEC-028 DRY SSOT).

use edgequake_query::{QueryMode, QueryRequest as EngineQueryRequest};

use crate::handlers::query_types::{ConversationMessage, MixWeightRequest};

/// Common query execution parameters shared by `/query`, `/query/context`, and `/chat`.
#[derive(Debug, Clone)]
pub struct QueryExecutionParams {
    pub query: String,
    pub mode: QueryMode,
    pub max_results: Option<usize>,
    pub context_only: bool,
    pub prompt_only: bool,
    pub enable_rerank: bool,
    pub rerank_top_k: Option<usize>,
    pub mix_weights: Option<MixWeightRequest>,
    pub conversation_history: Option<Vec<ConversationMessage>>,
    pub system_prompt: Option<String>,
    pub allowed_document_ids: Option<Vec<String>>,
    pub data_tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
}

impl QueryExecutionParams {
    /// Parse mode string; rejects bypass for context-only endpoints.
    pub fn parse_mode(
        mode: Option<&String>,
        default: QueryMode,
    ) -> Result<QueryMode, &'static str> {
        match mode {
            Some(m) => {
                let parsed = QueryMode::parse(m).ok_or("INVALID_MODE")?;
                Ok(parsed)
            }
            None => Ok(default),
        }
    }

    pub fn reject_bypass(mode: QueryMode) -> Result<(), &'static str> {
        if mode.is_bypass() {
            Err("INVALID_MODE")
        } else {
            Ok(())
        }
    }
}

/// Build engine request from shared params (single SSOT for all query paths).
pub fn build_engine_request(params: &QueryExecutionParams) -> EngineQueryRequest {
    let mut engine_request = EngineQueryRequest::new(&params.query).with_mode(params.mode);

    if let Some(ref system_prompt) = params.system_prompt {
        engine_request = engine_request.with_system_prompt(system_prompt);
    }
    if let Some(ref tenant_id) = params.data_tenant_id {
        engine_request = engine_request.with_tenant_id(tenant_id.clone());
    }
    if let Some(ref workspace_id) = params.workspace_id {
        engine_request = engine_request.with_workspace_id(workspace_id.clone());
    }
    if let Some(max) = params.max_results {
        engine_request.max_results = Some(max);
    }
    if params.context_only {
        engine_request = engine_request.context_only();
    }
    if params.prompt_only {
        engine_request = engine_request.prompt_only();
    }
    if let Some(ref mix_weights) = params.mix_weights {
        if mix_weights.is_set() {
            engine_request.mix_weights = Some(mix_weights.to_engine_override());
        }
    }
    engine_request = engine_request.with_rerank(params.enable_rerank);
    if let Some(top_k) = params.rerank_top_k {
        engine_request = engine_request.with_rerank_top_k(top_k);
    }
    if let Some(ref provider) = params.llm_provider {
        engine_request = engine_request.with_llm_provider(provider);
    }
    if let Some(ref model) = params.llm_model {
        engine_request = engine_request.with_llm_model(model);
    }
    if let Some(history) = &params.conversation_history {
        let engine_history: Vec<edgequake_query::ConversationMessage> = history
            .iter()
            .map(|m| edgequake_query::ConversationMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();
        engine_request = engine_request.with_conversation_history(engine_history);
    }
    if let Some(ref allowed_ids) = params.allowed_document_ids {
        engine_request = engine_request.with_allowed_document_ids(allowed_ids.clone());
    }

    engine_request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_only_flag_propagates() {
        let params = QueryExecutionParams {
            query: "test".into(),
            mode: QueryMode::Mix,
            max_results: None,
            context_only: true,
            prompt_only: false,
            enable_rerank: true,
            rerank_top_k: None,
            mix_weights: None,
            conversation_history: None,
            system_prompt: None,
            allowed_document_ids: None,
            data_tenant_id: None,
            workspace_id: None,
            llm_provider: None,
            llm_model: None,
        };
        let req = build_engine_request(&params);
        assert!(req.context_only);
    }

    #[test]
    fn reject_bypass_for_context() {
        assert!(QueryExecutionParams::reject_bypass(QueryMode::Bypass).is_err());
        assert!(QueryExecutionParams::reject_bypass(QueryMode::Mix).is_ok());
    }
}
