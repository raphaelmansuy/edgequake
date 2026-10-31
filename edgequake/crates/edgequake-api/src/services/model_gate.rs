//! SPEC-179: the one judge for a model that is about to run.
//!
//! Callers pass the pair they resolved. This module loads nothing new: it
//! uses [`ensure_resolved_pair`]. An operator lock skips the offer. The
//! `mock` provider is the test double, not a production fallback.

use edgequake_core::{Usage, Workspace};

use crate::error::{ApiError, ApiResult};
use crate::services::model_policy_enforcement::{ensure_resolved_pair, operator_lock_set};
use crate::state::AppState;

/// Judge the pair a workspace is about to run.
pub async fn judge_running_pair(
    state: &AppState,
    workspace: &Workspace,
    usage: Usage,
    provider: &str,
    model: &str,
) -> ApiResult<()> {
    if operator_lock_set(usage) || provider.eq_ignore_ascii_case("mock") {
        return Ok(());
    }
    ensure_resolved_pair(
        state,
        Some(workspace),
        usage,
        None,
        None,
        Some(provider),
        Some(model),
    )
    .await
}

/// SPEC-180: a usage the workspace profile never runs is not judged (LAW-180-3).
/// A normal workspace has no extract, summary or keyword step, so a stale extract pair
/// cannot block its ingest.
pub fn usage_not_applicable(
    caps: &edgequake_core::rag_profile::ProfileCapabilities,
    usage: Usage,
) -> bool {
    match usage {
        Usage::Extract => !caps.judges_extract_usage,
        Usage::Summary => !caps.judges_summary_usage,
        Usage::Keyword => !caps.queries_graph,
        _ => false,
    }
}

/// The pair a workspace will run for one usage. Summary with no role inherits extract.
pub async fn judge_workspace_usage(
    state: &AppState,
    workspace_id: uuid::Uuid,
    usage: Usage,
) -> ApiResult<()> {
    if state.optional_pg_pool().is_none() {
        return Ok(());
    }
    let Ok(Some(ws)) = state.workspace_service.get_workspace(workspace_id).await else {
        return Ok(());
    };
    if usage_not_applicable(&ws.capabilities(), usage) {
        return Ok(());
    }
    let tenant = state
        .workspace_service
        .get_tenant(ws.tenant_id)
        .await
        .ok()
        .flatten();
    let (provider, model) = match usage {
        Usage::Vlm => {
            let cfg = crate::services::vlm_provider_resolver::resolve_workspace_vlm_config(
                &ws,
                tenant.as_ref(),
            );
            (cfg.provider, cfg.model)
        }
        Usage::Embedding => {
            let choice = edgequake_core::resolve_embedding_choice(
                None,
                None,
                None,
                Some(&ws),
                tenant.as_ref(),
            );
            (choice.provider, choice.model)
        }
        Usage::Summary => {
            if edgequake_core::role_config_from_workspace(&ws, edgequake_core::LlmRole::Summary)
                .is_none()
            {
                return Ok(());
            }
            let role = edgequake_core::resolve_role_llm(&ws, edgequake_core::LlmRole::Summary);
            (role.provider, role.model)
        }
        Usage::Query => {
            let role = edgequake_core::resolve_role_llm(&ws, edgequake_core::LlmRole::Query);
            (role.provider, role.model)
        }
        Usage::Extract => {
            let role = edgequake_core::resolve_extract_role_llm(&ws);
            (role.provider, role.model)
        }
        Usage::Keyword => {
            let role = edgequake_core::resolve_role_llm(&ws, edgequake_core::LlmRole::Keyword);
            (role.provider, role.model)
        }
        Usage::Decision | Usage::Rerank => return Ok(()),
    };
    if provider.trim().is_empty() || model.trim().is_empty() {
        if operator_lock_set(usage) {
            return Ok(());
        }
        return Err(ApiError::unprocessable_coded(
            "model_not_eligible",
            "no model is allowed for this request",
        ));
    }
    judge_running_pair(state, &ws, usage, &provider, &model).await
}

/// Extract and embedding, the two clients an ingest pipeline builds.
///
/// A missing workspace is not this function's error. The pipeline factory
/// already reports that. No database means there is no offer to apply.
pub async fn judge_ingest(state: &AppState, workspace_id: &str) -> ApiResult<()> {
    if state.optional_pg_pool().is_none() {
        return Ok(());
    }
    let Some(uuid) = crate::middleware::resolve_workspace_uuid(Some(workspace_id)) else {
        return Ok(());
    };
    let Ok(Some(ws)) = state.workspace_service.get_workspace(uuid).await else {
        return Ok(());
    };
    if !usage_not_applicable(&ws.capabilities(), Usage::Extract) {
        let extract = edgequake_core::resolve_extract_role_llm(&ws);
        judge_running_pair(
            state,
            &ws,
            Usage::Extract,
            &extract.provider,
            &extract.model,
        )
        .await?;
    }
    let tenant = state
        .workspace_service
        .get_tenant(ws.tenant_id)
        .await
        .ok()
        .flatten();
    let embedding =
        edgequake_core::resolve_embedding_choice(None, None, None, Some(&ws), tenant.as_ref());
    if !embedding.provider.is_empty() && !embedding.model.is_empty() {
        judge_running_pair(
            state,
            &ws,
            Usage::Embedding,
            &embedding.provider,
            &embedding.model,
        )
        .await?;
    }
    Ok(())
}

pub fn failure_message(err: &ApiError) -> String {
    format!("{}: {err}. Fix this under Workspace → Models.", err.code())
}

/// `If-Match` absent proceeds. A present value must equal the stored revision.
pub fn if_match_ok(header: Option<&str>, current: i64) -> ApiResult<()> {
    let Some(header) = header.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let sent: i64 = header
        .trim_matches('"')
        .parse()
        .map_err(|_| ApiError::Coded {
            code: "stale_revision",
            message: "If-Match is not a revision".into(),
            status: axum::http::StatusCode::CONFLICT,
        })?;
    if sent != current {
        return Err(ApiError::Coded {
            code: "stale_revision",
            message: "the offer changed; reload and try again".into(),
            status: axum::http::StatusCode::CONFLICT,
        });
    }
    Ok(())
}

/// An embedding model needs a positive dimension. Other models do not.
pub fn require_embedding_dimension(
    supports_embedding: bool,
    dimension: Option<i32>,
) -> ApiResult<()> {
    if supports_embedding && dimension.filter(|value| *value > 0).is_none() {
        return Err(ApiError::unprocessable_coded(
            "embedding_dimension_required",
            "an embedding model needs its dimension",
        ));
    }
    Ok(())
}

/// A failed refresh keeps the previous cache. A successful one replaces it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshPlan {
    pub replace: bool,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

pub fn refresh_plan(previous: &[String], listed: &[String], ok: bool) -> RefreshPlan {
    if !ok {
        return RefreshPlan {
            replace: false,
            added: Vec::new(),
            removed: Vec::new(),
        };
    }
    let listed_set: std::collections::BTreeSet<&str> = listed.iter().map(String::as_str).collect();
    let previous_set: std::collections::BTreeSet<&str> =
        previous.iter().map(String::as_str).collect();
    RefreshPlan {
        replace: true,
        added: listed
            .iter()
            .filter(|id| !previous_set.contains(id.as_str()))
            .cloned()
            .collect(),
        removed: previous
            .iter()
            .filter(|id| !listed_set.contains(id.as_str()))
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_refresh_does_not_replace_the_cache() {
        let plan = refresh_plan(&["a".into()], &["b".into()], false);
        assert!(!plan.replace);
        assert!(plan.added.is_empty());
        assert!(plan.removed.is_empty());
    }

    #[test]
    fn successful_refresh_reports_the_diff() {
        let plan = refresh_plan(&["a".into(), "b".into()], &["b".into(), "c".into()], true);
        assert!(plan.replace);
        assert_eq!(plan.added, vec!["c".to_string()]);
        assert_eq!(plan.removed, vec!["a".to_string()]);
    }

    #[test]
    fn embedding_without_a_dimension_is_rejected() {
        assert!(require_embedding_dimension(true, None).is_err());
        assert!(require_embedding_dimension(true, Some(0)).is_err());
        assert!(require_embedding_dimension(true, Some(1536)).is_ok());
        assert!(require_embedding_dimension(false, None).is_ok());
    }

    #[test]
    fn if_match_absent_proceeds_and_a_mismatch_conflicts() {
        assert!(if_match_ok(None, 3).is_ok());
        assert!(if_match_ok(Some("3"), 3).is_ok());
        assert!(if_match_ok(Some("\"3\""), 3).is_ok());
        let err = if_match_ok(Some("2"), 3).unwrap_err();
        assert_eq!(err.code(), "stale_revision");
    }
}
