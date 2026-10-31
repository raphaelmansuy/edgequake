//! Search projection: hits[] from ContextBundle (SPEC-152).

use chrono::{Duration as ChronoDuration, Utc};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;

use edgequake_llm::traits::LLMProvider;

use crate::error::ApiResult;
use crate::handlers::context_types::{
    ContentGranularity, ContextBundle, ContextRetrievalRequest, ContextRetrievalResponse,
    ModeSelection,
};
use crate::handlers::query_types::DocumentFilter;
use crate::middleware::TenantContext;
use crate::services::query_context::retrieve_context;
use crate::services::retrieval_id_cache::global_retrieval_cache;
use crate::state::AppState;

use super::budget::{apply_budget, BudgetClass};
use super::doc_scope::DocumentScope;
use super::entity_ref::agent_id_for_node;
use super::envelope::EnvelopeBuilder;
use super::errors::{eq_error, ErrorCode};
use super::ids::{is_artifact_type, map_entity_type, title_case_label, truncate_chars};
use super::scores::scores_from_hits;

const DEFAULT_HIT_LIMIT: usize = 8;
const SNIPPET_MAX: usize = 240;

/// Run retrieve+cache then project hits (does not change REST search_context shape).
pub async fn eq_search(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
    llm_override: Option<Arc<dyn LLMProvider>>,
) -> ApiResult<Value> {
    let budget = BudgetClass::parse(args.get("budget").and_then(|v| v.as_str()));
    let limit = args
        .get("limit")
        .or_else(|| args.get("max_results"))
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_HIT_LIMIT as u64)
        .clamp(1, 50) as usize;

    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mode = args
        .get("mode")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // SPEC-162 R7/R8: hard MCP scope (ids ∩ pattern); unknown id → not_found.
    let scope = DocumentScope::resolve(state, tenant_ctx, args).await?;
    if let Some(ref s) = scope {
        if let Some(unknown) = s.unknown_ids.first() {
            return Ok(eq_error(
                ErrorCode::NotFound,
                format!("Document not found: {unknown}"),
                None,
            ));
        }
        if s.filter_result == Some("no_match") {
            let expires_at = Utc::now() + ChronoDuration::minutes(15);
            return Ok(EnvelopeBuilder::new("search", budget)
                .insert("retrieval_id", json!(null))
                .insert("expires_at", json!(expires_at.to_rfc3339()))
                .insert("hits", json!([]))
                .insert("documents_considered", json!([]))
                .insert("filter_result", json!("no_match"))
                .insert(
                    "message",
                    json!(s
                        .message
                        .clone()
                        .unwrap_or_else(|| "No document matches the pattern.".into())),
                )
                .build());
        }
        if s.allowed_ids.is_empty() {
            let expires_at = Utc::now() + ChronoDuration::minutes(15);
            return Ok(EnvelopeBuilder::new("search", budget)
                .insert("retrieval_id", json!(null))
                .insert("expires_at", json!(expires_at.to_rfc3339()))
                .insert("hits", json!([]))
                .insert("documents_considered", json!([]))
                .insert("filter_result", json!("empty"))
                .build());
        }
    }

    let document_filter = scope.as_ref().map(|s| DocumentFilter {
        document_ids: Some(s.allowed_ids.clone()),
        ..Default::default()
    });
    // SPEC-180: chunk-kind filters ride along; a graph workspace refuses them in the service.
    let document_filter = super::profile_gate::with_kind_filters(document_filter, args);
    // A workspace without a graph has no entity hits to project.
    let caps = crate::services::workspace_profile::workspace_capabilities(
        state,
        tenant_ctx.workspace_id.as_deref(),
    )
    .await;

    let full_request = ContextRetrievalRequest {
        query: query.clone(),
        mode: mode.clone(),
        content_granularity: ContentGranularity::Agent,
        max_results: Some(limit.max(12)),
        conversation_history: None,
        document_filter,
        mix_weights: None,
        enable_rerank: true,
        rerank_model: None,
        rerank_top_k: None,
        include_lineage: true,
        include_documents: true,
        include_agent_hints: false,
        include_subgraph: caps.queries_graph, // entity hits need it; fetch defaults it off
    };

    let response = retrieve_context(state, tenant_ctx, full_request, llm_override).await?;
    let expires_at = Utc::now() + ChronoDuration::minutes(15);

    let mut projected = project_search_response(
        &response,
        tenant_ctx.workspace_id.as_deref().unwrap_or("default"),
        budget,
        limit,
        expires_at.to_rfc3339(),
        false,
    );

    if let Some(ref s) = scope {
        // Defense in depth: drop hits outside allowed ids.
        if let Some(hits) = projected.get_mut("hits").and_then(|v| v.as_array_mut()) {
            hits.retain(|h| {
                let doc = h.get("document_id").and_then(|v| v.as_str()).unwrap_or("");
                let docs = h
                    .get("document_ids")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str())
                            .any(|d| s.allowed_ids.iter().any(|a| a == d))
                    })
                    .unwrap_or(false);
                s.allowed_ids.iter().any(|a| a == doc) || docs
            });
        }
        let mut considered = s.allowed_ids.clone();
        considered.sort();
        if let Some(obj) = projected.as_object_mut() {
            obj.insert("documents_considered".into(), json!(considered));
            let hits_empty = obj
                .get("hits")
                .and_then(|v| v.as_array())
                .map(|a| a.is_empty())
                .unwrap_or(true);
            if hits_empty {
                obj.insert("filter_result".into(), json!("empty"));
            }
        }
    }

    Ok(projected)
}

pub fn project_search_response(
    response: &ContextRetrievalResponse,
    workspace: &str,
    budget: BudgetClass,
    limit: usize,
    expires_at: String,
    include_artifacts: bool,
) -> Value {
    let mut hits = build_hits(&response.bundle, workspace, limit, include_artifacts);
    let score_type = scores_from_hits(&hits);
    let cross_document = compute_cross_document(&response.bundle);
    let mut documents_considered: Vec<String> = response
        .bundle
        .documents
        .iter()
        .map(|d| d.document_id.clone())
        .collect();
    documents_considered.sort();

    let (mode_used, mode_reason) = mode_echo(&response.mode_selection);

    // Cap hit snippets
    for hit in &mut hits {
        if let Some(Value::String(s)) = hit.get_mut("snippet") {
            *s = truncate_chars(s, SNIPPET_MAX);
        }
    }

    let env = EnvelopeBuilder::new("search", budget)
        .insert("retrieval_id", json!(response.retrieval_id))
        .insert("expires_at", json!(expires_at))
        .insert("mode_used", json!(mode_used))
        .insert("mode_reason", json!(mode_reason))
        .insert("cross_document", json!(cross_document))
        .insert("score_type", json!(score_type.as_str()))
        .insert("hits", json!(hits))
        .insert("documents_considered", json!(documents_considered))
        .insert(
            "stats",
            json!({
                "total_ms": response.stats.total_time_ms,
                "cached": response.cached,
                "fingerprint": response.retrieval_fingerprint,
            }),
        )
        .build();

    let (capped, over) = apply_budget(env, budget);
    if over {
        return super::errors::eq_error(
            super::errors::ErrorCode::TruncateInvalid,
            "structuredContent exceeded budget after truncation",
            None,
        );
    }
    capped
}

fn build_hits(
    bundle: &ContextBundle,
    workspace: &str,
    limit: usize,
    include_artifacts: bool,
) -> Vec<Value> {
    let mut candidates: Vec<(f64, Value)> = Vec::new();

    for chunk in &bundle.chunks {
        let doc_id = chunk
            .lineage
            .as_ref()
            .and_then(|l| l.document_id.clone())
            .unwrap_or_default();
        let file_name = chunk
            .lineage
            .as_ref()
            .and_then(|l| l.file_path.clone())
            .unwrap_or_default();
        let snippet = truncate_chars(&chunk.content, SNIPPET_MAX);
        candidates.push((
            chunk.score as f64,
            json!({
                "kind": "chunk",
                "id": chunk.id,
                "document_id": doc_id,
                "title": file_name,
                "snippet": snippet,
                "score": chunk.score,
            }),
        ));
    }

    for ent in &bundle.subgraph.entities {
        if !include_artifacts && is_artifact_type(&ent.entity_type) {
            continue;
        }
        // SPEC-162 R3: same field as eq_entity_search (storage / graph_node_id).
        let storage_id = if !ent.graph_node_id.is_empty() {
            ent.graph_node_id.as_str()
        } else {
            ent.name.as_str()
        };
        let id = agent_id_for_node(workspace, storage_id);
        let mut doc_ids = ent.source_document_ids.clone();
        if doc_ids.is_empty() {
            if let Some(ref lin) = ent.lineage {
                doc_ids = lin.source_document_ids.clone();
                if doc_ids.is_empty() {
                    if let Some(ref singular) = lin.source_document_id {
                        doc_ids.push(singular.clone());
                    }
                }
            }
        }
        let doc_id = doc_ids.first().cloned().unwrap_or_default();
        candidates.push((
            ent.score as f64,
            json!({
                "kind": "entity",
                "id": id,
                "document_id": doc_id,
                "document_ids": doc_ids,
                "title": title_case_label(edgequake_storage::EntityId::bare_name_from_graph_node_id(storage_id)),
                "snippet": truncate_chars(&ent.description, SNIPPET_MAX),
                "score": ent.score,
                "type": map_entity_type(&ent.entity_type),
            }),
        ));
    }

    for doc in &bundle.documents {
        let score = 0.5_f64;
        candidates.push((
            score,
            json!({
                "kind": "document",
                "id": doc.document_id,
                "document_id": doc.document_id,
                "title": doc.title,
                "snippet": "",
                "score": score,
            }),
        ));
    }

    candidates.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let ia = a.1.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let ib = b.1.get("id").and_then(|v| v.as_str()).unwrap_or("");
                ia.cmp(ib)
            })
    });
    candidates.into_iter().take(limit).map(|(_, v)| v).collect()
}

fn compute_cross_document(bundle: &ContextBundle) -> bool {
    let mut ids = HashSet::new();
    for c in &bundle.chunks {
        if let Some(id) = c.lineage.as_ref().and_then(|l| l.document_id.as_ref()) {
            ids.insert(id.clone());
        }
    }
    ids.len() > 1
}

fn mode_echo(sel: &ModeSelection) -> (String, String) {
    let used = sel.effective.clone();
    let reason = if sel.adaptive || sel.requested != sel.effective {
        format!(
            "requested={} effective={} adaptive={}{}",
            sel.requested,
            sel.effective,
            sel.adaptive,
            sel.intent
                .as_ref()
                .map(|i| format!(" intent={i}"))
                .unwrap_or_default()
        )
    } else {
        format!("requested {}", sel.requested)
    };
    (used, reason)
}

pub fn build_document_filter(args: &Value) -> Option<DocumentFilter> {
    let ids = args
        .get("document_ids")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty());

    let nested = args.get("document_filter");
    let pattern = args
        .get("document_pattern")
        .or_else(|| nested.and_then(|f| f.get("document_pattern")))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let date_from = args
        .get("date_from")
        .or_else(|| nested.and_then(|f| f.get("date_from")))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let date_to = args
        .get("date_to")
        .or_else(|| nested.and_then(|f| f.get("date_to")))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let nested_ids = nested
        .and_then(|f| f.get("document_ids"))
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty());

    let document_ids = ids.or(nested_ids);
    let filter = DocumentFilter {
        date_from,
        date_to,
        document_pattern: pattern,
        document_ids,
        ..Default::default()
    };
    let filter = super::profile_gate::with_kind_filters(Some(filter), args).unwrap_or_default();
    if filter.is_empty() && !filter.has_kind_filters() {
        None
    } else {
        Some(filter)
    }
}

/// Expose cache TTL remaining as expires_at when available.
#[allow(dead_code)]
pub fn cache_still_valid(retrieval_id: &str) -> bool {
    !global_retrieval_cache().is_expired(retrieval_id)
        && global_retrieval_cache().get(retrieval_id).is_some()
}
