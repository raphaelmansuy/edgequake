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
    /// GraphRAG-Bench / product question type (047 type-scoped prompts).
    pub question_type: Option<String>,
    /// 083 LightRAG-shaped keyword override.
    pub hl_keywords: Option<Vec<String>>,
    pub ll_keywords: Option<Vec<String>>,
    pub response_type: Option<String>,
    pub allowed_document_ids: Option<Vec<String>>,
    pub data_tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
    /// SPEC-109: effective reasoning effort (post-clamp) for answer generation.
    pub reasoning_effort: Option<String>,
    /// SPEC-146: cache isolation fields (set when ABAC on).
    pub authz_principal: Option<String>,
    pub policy_generation: Option<u64>,
    pub allow_fingerprint: Option<String>,
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
    if let Some(ref question_type) = params.question_type {
        let trimmed = question_type.trim();
        if !trimmed.is_empty() {
            engine_request = engine_request.with_question_type(trimmed);
        }
    }
    if let Some(ref tenant_id) = params.data_tenant_id {
        engine_request = engine_request.with_tenant_id(tenant_id.clone());
    }
    // SPEC-091 IW0 (GAP-091-11, LAW-I4): vector scope is never unscoped.
    // Headerless requests clamp to the default workspace UUID (the form
    // ingestion persists into vector metadata); a malformed header passes
    // through raw so it matches nothing instead of silently defaulting.
    let scoped_workspace = match params.workspace_id.as_deref().map(str::trim) {
        None | Some("") => crate::middleware::default_workspace_uuid().to_string(),
        Some(raw) => match crate::middleware::resolve_workspace_uuid(Some(raw)) {
            Some(uuid) => uuid.to_string(),
            None => raw.to_string(),
        },
    };
    engine_request = engine_request.with_workspace_id(scoped_workspace);
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
    if let Some(ref effort) = params.reasoning_effort {
        engine_request = engine_request.with_reasoning_effort(effort);
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
    if let (Some(principal), Some(fp)) = (
        params.authz_principal.as_ref(),
        params.allow_fingerprint.as_ref(),
    ) {
        engine_request = engine_request.with_authz_cache_scope(
            principal.clone(),
            params.policy_generation.unwrap_or(0),
            fp.clone(),
        );
    }
    if let Some(ref hl) = params.hl_keywords {
        engine_request = engine_request.with_hl_keywords(hl.clone());
    }
    if let Some(ref ll) = params.ll_keywords {
        engine_request = engine_request.with_ll_keywords(ll.clone());
    }
    if let Some(ref rt) = params.response_type {
        let trimmed = rt.trim();
        if !trimmed.is_empty() {
            engine_request = engine_request.with_response_type(trimmed);
        }
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
            question_type: None,
            hl_keywords: None,
            ll_keywords: None,
            response_type: None,
            allowed_document_ids: None,
            data_tenant_id: None,
            workspace_id: None,
            llm_provider: None,
            llm_model: None,
            reasoning_effort: None,
        authz_principal: None,
        policy_generation: None,
        allow_fingerprint: None,
        };
        let req = build_engine_request(&params);
        assert!(req.context_only);
    }

    #[test]
    fn question_type_propagates_to_engine_params() {
        let params = QueryExecutionParams {
            query: "test".into(),
            mode: QueryMode::Mix,
            max_results: None,
            context_only: false,
            prompt_only: false,
            enable_rerank: true,
            rerank_top_k: None,
            mix_weights: None,
            conversation_history: None,
            system_prompt: None,
            question_type: Some("Complex Reasoning".into()),
            hl_keywords: None,
            ll_keywords: None,
            response_type: None,
            allowed_document_ids: None,
            data_tenant_id: None,
            workspace_id: None,
            llm_provider: None,
            llm_model: None,
            reasoning_effort: None,
        authz_principal: None,
        policy_generation: None,
        allow_fingerprint: None,
        };
        let req = build_engine_request(&params);
        assert_eq!(req.question_type(), Some("Complex Reasoning"));
    }

    #[test]
    fn builds_keyword_override_and_response_type() {
        let params = QueryExecutionParams {
            query: "staging for NSCLC".into(),
            mode: QueryMode::Mix,
            max_results: None,
            context_only: false,
            prompt_only: false,
            enable_rerank: true,
            rerank_top_k: None,
            mix_weights: None,
            conversation_history: None,
            system_prompt: None,
            question_type: None,
            hl_keywords: Some(vec!["staging".into(), "NSCLC".into()]),
            ll_keywords: Some(vec!["TNM".into()]),
            response_type: Some("Bullet Points".into()),
            allowed_document_ids: None,
            data_tenant_id: None,
            workspace_id: None,
            llm_provider: None,
            llm_model: None,
            reasoning_effort: None,
        authz_principal: None,
        policy_generation: None,
        allow_fingerprint: None,
        };
        let req = build_engine_request(&params);
        assert!(req.has_keyword_override());
        assert_eq!(req.response_type_or_default(), "Bullet Points");
        assert_eq!(
            req.keyword_override_lists(),
            Some((vec!["staging".into(), "NSCLC".into()], vec!["TNM".into()]))
        );
    }

    #[test]
    fn reject_bypass_for_context() {
        assert!(QueryExecutionParams::reject_bypass(QueryMode::Bypass).is_err());
        assert!(QueryExecutionParams::reject_bypass(QueryMode::Mix).is_ok());
    }
}
