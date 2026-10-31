//! SPEC-178: the list, the save, and the run share one verdict.
//!
//! The WebUI picker is a convenience. The server is the authority. A request
//! that names a provider/model pair must be in `eligible_models` for the
//! caller's workspace offer (`Some(offer)`), or it is refused with 422
//! `model_not_eligible` / `capability_mismatch`. An omitted pair still
//! judges the stored assignment when one was saved. A one-sided pair is
//! `422 model_pair_incomplete`.

use std::collections::BTreeSet;

use edgequake_core::model_policy::{eligible_models, CatalogEntry, PolicyRow};
use edgequake_core::{Usage, Workspace};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::services::model_policy_catalog::{load_catalog, CatalogItem};
use crate::services::model_policy_rows::load_rows;
use crate::state::AppState;

/// Why a pair is or is not selectable. Shared by enforcement and the effective view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairVerdict {
    Eligible,
    CapabilityMismatch,
    NotEligible,
}

/// A locked usage ignores the offer. Every other usage is limited to it.
pub fn offer_filter(
    locked: bool,
    offered: &std::collections::BTreeSet<String>,
) -> Option<&std::collections::BTreeSet<String>> {
    if locked {
        None
    } else {
        Some(offered)
    }
}

/// Pure decision over already-loaded facts.
pub fn pair_verdict(
    usage: Usage,
    provider: &str,
    model: &str,
    catalog: &[CatalogEntry],
    ceiling: &[PolicyRow],
    narrowing: &[PolicyRow],
    offered: Option<&std::collections::BTreeSet<String>>,
) -> PairVerdict {
    let provider = provider.trim().to_lowercase();
    let model = model.trim();
    let is_pair = |entry: &&CatalogEntry| entry.provider == provider && entry.model == model;
    if eligible_models(usage, catalog, ceiling, narrowing, offered)
        .iter()
        .any(is_pair)
    {
        return PairVerdict::Eligible;
    }
    match catalog.iter().find(is_pair) {
        Some(entry) if !entry.facts.supports(usage) => PairVerdict::CapabilityMismatch,
        _ => PairVerdict::NotEligible,
    }
}

/// Map a verdict to the request outcome: `Ok`, or a 422 with the matching code.
pub fn check_pair(
    usage: Usage,
    provider: &str,
    model: &str,
    catalog: &[CatalogEntry],
    ceiling: &[PolicyRow],
    narrowing: &[PolicyRow],
    offered: Option<&std::collections::BTreeSet<String>>,
) -> ApiResult<()> {
    match pair_verdict(usage, provider, model, catalog, ceiling, narrowing, offered) {
        PairVerdict::Eligible => Ok(()),
        PairVerdict::CapabilityMismatch => Err(ApiError::unprocessable_coded(
            "capability_mismatch",
            format!("model does not support the {} usage", usage.as_str()),
        )),
        PairVerdict::NotEligible => Err(ApiError::unprocessable_coded(
            "model_not_eligible",
            "model is not selectable for this workspace",
        )),
    }
}

/// Enforce the pair for `usage` in `workspace`. A request with no pair passes. A pair
/// with no workspace scope fails closed.
pub async fn ensure_pair_eligible(
    state: &AppState,
    workspace: Option<&Workspace>,
    usage: Usage,
    provider: Option<&str>,
    model: Option<&str>,
) -> ApiResult<()> {
    let (Some(provider), Some(model)) = (provider, model) else {
        return Ok(());
    };
    let Some(workspace) = workspace else {
        return Err(ApiError::unprocessable_coded(
            "model_not_eligible",
            "a model override needs a workspace scope",
        ));
    };
    let ceiling = load_rows(state, workspace.tenant_id, None).await?;
    let narrowing = load_rows(state, workspace.tenant_id, Some(workspace.workspace_id)).await?;
    let mut catalog = load_catalog(state).await;
    crate::services::model_policy_catalog::append_connection_models(state, &mut catalog).await?;
    let entries: Vec<CatalogEntry> = catalog.into_iter().map(|item| item.entry).collect();
    let offer = crate::services::provider_offer_rows::load_workspace_offer(
        state,
        workspace.tenant_id,
        workspace.workspace_id,
    )
    .await?;
    check_pair(
        usage,
        provider,
        model,
        &entries,
        &ceiling,
        &narrowing,
        Some(&offer.slugs),
    )
}

/// Both names, or neither. One side is not gap-filled.
pub fn ensure_pair_complete(provider: Option<&str>, model: Option<&str>) -> ApiResult<()> {
    let present = |value: Option<&str>| {
        value
            .map(str::trim)
            .filter(|text| {
                !text.is_empty()
                    && !edgequake_core::workspace_model_update::is_clear_override_value(text)
            })
            .is_some()
    };
    match (present(provider), present(model)) {
        (true, true) | (false, false) => Ok(()),
        _ => Err(ApiError::unprocessable_coded(
            "model_pair_incomplete",
            "choose a provider and a model together",
        )),
    }
}

/// A request pair on a pinned usage is `422 usage_locked`. An omitted pair may run the pin.
pub fn refuse_locked_request(usage: Usage, request_set: bool) -> ApiResult<()> {
    if request_set && operator_lock_set(usage) {
        return Err(ApiError::unprocessable_coded(
            "usage_locked",
            "an operator lock chooses this model",
        ));
    }
    Ok(())
}

/// The pair that will run. A pre-resolve check passes no resolved names and may
/// return `None` (inherit). A resolved call with an empty name is refused.
pub fn require_running_pair<'a>(
    request_set: bool,
    resolved_provider: Option<&str>,
    resolved_model: Option<&str>,
    provider: Option<&'a str>,
    model: Option<&'a str>,
) -> ApiResult<Option<(&'a str, &'a str)>> {
    match named_pair(provider, model)? {
        Some(pair) => Ok(Some(pair)),
        None if !request_set && resolved_provider.is_none() && resolved_model.is_none() => Ok(None),
        None => Err(ApiError::unprocessable_coded(
            "model_not_eligible",
            "no model is allowed for this request",
        )),
    }
}

/// True when an operator pin owns this usage, so a request pair must not run.
pub fn operator_lock_set(usage: Usage) -> bool {
    let set = |key: &str| {
        std::env::var(key)
            .ok()
            .is_some_and(|value| !value.trim().is_empty())
    };
    match usage {
        Usage::Extract => {
            set("EDGEQUAKE_EXTRACT_LLM_MODEL") || set("EDGEQUAKE_EXTRACT_LLM_PROVIDER")
        }
        Usage::Keyword => {
            set("EDGEQUAKE_KEYWORD_LLM_MODEL") || set("EDGEQUAKE_KEYWORD_LLM_PROVIDER")
        }
        Usage::Vlm => {
            set("EDGEQUAKE_VISION_PROVIDER_MANUSCRIPT") || set("EDGEQUAKE_VISION_MODEL_MANUSCRIPT")
        }
        _ => false,
    }
}

/// Request pair, then the pair that will run. Empty request plus a stored
/// illegal assignment refuses. No database pool skips the check (unit tests).
pub async fn ensure_resolved_pair(
    state: &AppState,
    workspace: Option<&Workspace>,
    usage: Usage,
    request_provider: Option<&str>,
    request_model: Option<&str>,
    resolved_provider: Option<&str>,
    resolved_model: Option<&str>,
) -> ApiResult<()> {
    ensure_pair_complete(request_provider, request_model)?;
    let request_set = request_provider.is_some_and(|value| !value.trim().is_empty())
        && request_model.is_some_and(|value| !value.trim().is_empty());
    refuse_locked_request(usage, request_set)?;
    if state.optional_pg_pool().is_none() {
        return Ok(());
    }
    let (provider, model) = if request_set {
        (request_provider, request_model)
    } else {
        (resolved_provider, resolved_model)
    };
    // Same exemption as `judge_running_pair`: the test-only mock provider is not in the
    // catalog, and it exists only when a test opted in.
    if provider.is_some_and(crate::provider_visibility::is_mock_provider)
        && crate::provider_visibility::mock_provider_allowed()
    {
        return Ok(());
    }
    match require_running_pair(
        request_set,
        resolved_provider,
        resolved_model,
        provider,
        model,
    )? {
        Some((provider, model)) => {
            ensure_pair_eligible(state, workspace, usage, Some(provider), Some(model)).await
        }
        None => Ok(()),
    }
}

const CHAT_FAMILY: [Usage; 4] = [Usage::Extract, Usage::Query, Usage::Summary, Usage::Keyword];

/// The model fields one workspace save names. `None` means the caller did not send it.
#[derive(Default, Clone, Copy)]
pub struct WorkspaceChoiceFields<'a> {
    pub llm_provider: Option<&'a str>,
    pub llm_model: Option<&'a str>,
    pub embedding_provider: Option<&'a str>,
    pub embedding_model: Option<&'a str>,
    pub embedding_dimension: Option<usize>,
    pub vision_provider: Option<&'a str>,
    pub vision_model: Option<&'a str>,
    pub decision_model: Option<&'a str>,
    pub llm_roles: Option<&'a serde_json::Value>,
}

/// Judge a workspace save. Returns the embedding dimension copied from the card
/// when the body names an embedding pair and does not disagree with the card.
pub async fn ensure_workspace_choice(
    state: &AppState,
    workspace: &Workspace,
    fields: WorkspaceChoiceFields<'_>,
) -> ApiResult<Option<usize>> {
    let WorkspaceChoiceFields {
        llm_provider,
        llm_model,
        embedding_provider,
        embedding_model,
        embedding_dimension,
        vision_provider,
        vision_model,
        decision_model,
        llm_roles,
    } = fields;
    if state.optional_pg_pool().is_none() {
        return Ok(embedding_dimension);
    }
    let mut catalog = load_catalog(state).await;
    crate::services::model_policy_catalog::append_connection_models(state, &mut catalog).await?;
    let entries: Vec<CatalogEntry> = catalog.iter().map(|item| item.entry.clone()).collect();
    let ceiling = load_rows(state, workspace.tenant_id, None).await?;
    let narrowing = load_rows(state, workspace.tenant_id, Some(workspace.workspace_id)).await?;
    let offer = crate::services::provider_offer_rows::load_workspace_offer(
        state,
        workspace.tenant_id,
        workspace.workspace_id,
    )
    .await?;
    // An empty offer means no provider is offered at all, so every set pair is
    // refused below as model_not_eligible. The check is never skipped.
    let offered = Some(&offer.slugs);
    if let Some((provider, model)) = named_pair(llm_provider, llm_model)? {
        ensure_family(&entries, &ceiling, &narrowing, offered, provider, model)?;
    }
    if let Some((provider, model)) = named_pair(vision_provider, vision_model)? {
        check_pair(
            Usage::Vlm,
            provider,
            model,
            &entries,
            &ceiling,
            &narrowing,
            offered,
        )?;
    }
    if let Some(model) = named_one(decision_model) {
        // EC-175-08: a decision tag is not a chat-catalog pair. `tev1:0.8b`
        // is absent from `eligible_models` on purpose (it lives on the decision
        // host). Judging it there refuses every create that pins the default
        // engine, including "Use organization defaults". Name shape is SPEC-160;
        // inherit words are cleared later by `apply_decision_metadata`.
        judge_decision_model(model)?;
    }
    if let Some(roles) = llm_roles.and_then(|value| value.as_object()) {
        for (key, value) in roles {
            let Ok(usage) = crate::handlers::model_policy_types::parse_configurable_usage(key)
            else {
                continue;
            };
            let provider = value.get("provider").and_then(|item| item.as_str());
            let model = value.get("model").and_then(|item| item.as_str());
            if let Some((provider, model)) = named_pair(provider, model)? {
                check_pair(
                    usage, provider, model, &entries, &ceiling, &narrowing, offered,
                )?;
            }
        }
    }
    let mut dimension = embedding_dimension;
    if let Some((provider, model)) = named_pair(embedding_provider, embedding_model)? {
        check_pair(
            Usage::Embedding,
            provider,
            model,
            &entries,
            &ceiling,
            &narrowing,
            offered,
        )?;
        dimension = embedding_dimension_from_card(&catalog, provider, model, embedding_dimension)?;
    }
    Ok(dimension)
}

/// Tenant defaults. Every set pair must be eligible; an empty offer refuses it.
///
/// Returns the embedding dimension copied from the model card when the body names an
/// embedding pair. The card is authoritative: a body dimension that disagrees is refused.
pub async fn ensure_tenant_choice(
    state: &AppState,
    tenant_id: Uuid,
    llm_provider: Option<&str>,
    llm_model: Option<&str>,
    embedding: (Option<&str>, Option<&str>, Option<usize>),
    vision_provider: Option<&str>,
    vision_model: Option<&str>,
) -> ApiResult<Option<usize>> {
    let (embedding_provider, embedding_model, embedding_dimension) = embedding;
    if state.optional_pg_pool().is_none() {
        return Ok(embedding_dimension);
    }
    let offer = crate::services::provider_offer_rows::load_tenant_offer(state, tenant_id).await?;
    let ceiling = load_rows(state, tenant_id, None).await?;
    let mut catalog = load_catalog(state).await;
    crate::services::model_policy_catalog::append_connection_models(state, &mut catalog).await?;
    let entries: Vec<CatalogEntry> = catalog.iter().map(|item| item.entry.clone()).collect();
    let offered = Some(&offer.slugs);
    if let Some((provider, model)) = named_pair(llm_provider, llm_model)? {
        ensure_family(&entries, &ceiling, &[], offered, provider, model)?;
    }
    let mut dimension = embedding_dimension;
    if let Some((provider, model)) = named_pair(embedding_provider, embedding_model)? {
        check_pair(
            Usage::Embedding,
            provider,
            model,
            &entries,
            &ceiling,
            &[],
            offered,
        )?;
        dimension = embedding_dimension_from_card(&catalog, provider, model, embedding_dimension)?;
    }
    if let Some((provider, model)) = named_pair(vision_provider, vision_model)? {
        check_pair(
            Usage::Vlm,
            provider,
            model,
            &entries,
            &ceiling,
            &[],
            offered,
        )?;
    }
    Ok(dimension)
}

/// Fleet defaults. Same rules as a tenant choice against the global offer.
pub async fn ensure_deployment_choice(
    state: &AppState,
    llm_provider: Option<&str>,
    llm_model: Option<&str>,
    embedding_provider: Option<&str>,
    embedding_model: Option<&str>,
    vision_provider: Option<&str>,
    vision_model: Option<&str>,
) -> ApiResult<()> {
    if state.optional_pg_pool().is_none() {
        return Ok(());
    }
    let slugs = crate::services::provider_offer_rows::load_global_offer(state).await?;
    let llm = named_pair(llm_provider, llm_model)?;
    let embedding = named_pair(embedding_provider, embedding_model)?;
    let vision = named_pair(vision_provider, vision_model)?;
    if slugs.is_empty() {
        if llm.is_some() || embedding.is_some() || vision.is_some() {
            return Err(ApiError::unprocessable_coded(
                "model_not_eligible",
                "no provider is offered",
            ));
        }
        return Ok(());
    }
    let mut catalog = load_catalog(state).await;
    crate::services::model_policy_catalog::append_connection_models(state, &mut catalog).await?;
    let entries: Vec<CatalogEntry> = catalog.into_iter().map(|item| item.entry).collect();
    let offered = Some(&slugs);
    if let Some((provider, model)) = llm {
        ensure_family(&entries, &[], &[], offered, provider, model)?;
    }
    if let Some((provider, model)) = embedding {
        check_pair(
            Usage::Embedding,
            provider,
            model,
            &entries,
            &[],
            &[],
            offered,
        )?;
    }
    if let Some((provider, model)) = vision {
        check_pair(Usage::Vlm, provider, model, &entries, &[], &[], offered)?;
    }
    Ok(())
}

fn ensure_family(
    entries: &[CatalogEntry],
    ceiling: &[PolicyRow],
    narrowing: &[PolicyRow],
    offered: Option<&BTreeSet<String>>,
    provider: &str,
    model: &str,
) -> ApiResult<()> {
    for usage in CHAT_FAMILY {
        check_pair(usage, provider, model, entries, ceiling, narrowing, offered)?;
    }
    Ok(())
}

/// A decision-engine tag. Not filtered by the provider offer or the chat catalog.
fn judge_decision_model(model: &str) -> ApiResult<()> {
    if is_decision_inherit(model) {
        return Ok(());
    }
    edgequake_pipeline::extractor::decision::settings::check_model_name(model)
        .map_err(|err| ApiError::bad_request_coded("invalid_decision_model", err.to_string()))?;
    Ok(())
}

fn is_decision_inherit(model: &str) -> bool {
    matches!(
        model.trim().to_ascii_lowercase().as_str(),
        "" | "inherit" | "none"
    )
}

fn named_one(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|text| {
        !text.is_empty() && !edgequake_core::workspace_model_update::is_clear_override_value(text)
    })
}

fn named_pair<'a>(
    provider: Option<&'a str>,
    model: Option<&'a str>,
) -> ApiResult<Option<(&'a str, &'a str)>> {
    ensure_pair_complete(provider, model)?;
    match (named_one(provider), named_one(model)) {
        (Some(provider), Some(model)) => Ok(Some((provider, model))),
        _ => Ok(None),
    }
}

fn embedding_dimension_from_card(
    catalog: &[CatalogItem],
    provider: &str,
    model: &str,
    body: Option<usize>,
) -> ApiResult<Option<usize>> {
    let provider = provider.trim().to_lowercase();
    // SPEC-180 08: the multimodal model is served at 768, 1536 or 3072. The card holds the
    // default; the body may pick another width of the finite set.
    if crate::embeddings::is_vertex_multimodal(&provider, model) {
        let width = body.filter(|value| *value != 0).unwrap_or(1536);
        edgequake_core::rag_profile::check_multimodal_dimension(width)?;
        return Ok(Some(width));
    }
    let card = catalog
        .iter()
        .find(|item| item.entry.provider == provider && item.entry.model == model.trim());
    let Some(dim) = card.and_then(|item| item.embedding_dimension) else {
        return Err(ApiError::unprocessable_coded(
            "model_not_eligible",
            "embedding model has no dimension",
        ));
    };
    let dim = dim as usize;
    if let Some(body) = body.filter(|value| *value != 0) {
        if body != dim {
            return Err(ApiError::unprocessable_coded(
                "embedding_dimension_mismatch",
                "the dimension does not match this embedding model",
            ));
        }
    }
    Ok(Some(dim))
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_core::model_policy::ModelFacts;

    fn chat(provider: &str, model: &str) -> CatalogEntry {
        CatalogEntry {
            provider: provider.into(),
            model: model.into(),
            facts: ModelFacts {
                chat: true,
                registered: true,
                ..ModelFacts::default()
            },
        }
    }

    fn code(result: ApiResult<()>) -> Option<&'static str> {
        result.err().map(|e| e.code())
    }

    fn row(provider: &str, model: &str, usage: Usage, enabled: bool) -> PolicyRow {
        PolicyRow {
            provider: provider.into(),
            model: model.into(),
            usage,
            enabled,
        }
    }

    #[test]
    fn selectable_pair_passes_and_provider_is_case_insensitive() {
        let cat = [chat("openai", "gpt-5-nano")];
        assert!(check_pair(Usage::Query, " OpenAI ", "gpt-5-nano", &cat, &[], &[], None).is_ok());
    }

    #[test]
    fn unknown_pair_is_model_not_eligible() {
        let cat = [chat("openai", "gpt-5-nano")];
        let got = check_pair(Usage::Query, "openai", "gpt-unknown", &cat, &[], &[], None);
        assert_eq!(code(got), Some("model_not_eligible"));
    }

    #[test]
    fn empty_offer_refuses_every_set_pair() {
        // Regression: an empty offer used to skip the check, so an unoffered pair was stored.
        let cat = [chat("openai", "gpt-5-nano")];
        let empty = std::collections::BTreeSet::new();
        let got = check_pair(
            Usage::Query,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &[],
            Some(&empty),
        );
        assert_eq!(code(got), Some("model_not_eligible"));

        let mut offered = std::collections::BTreeSet::new();
        offered.insert("openai".to_string());
        let ok = check_pair(
            Usage::Query,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &[],
            Some(&offered),
        );
        assert!(ok.is_ok());
    }

    #[test]
    fn pair_outside_ceiling_is_model_not_eligible() {
        let cat = [chat("openai", "gpt-5-nano"), chat("ollama", "gemma3")];
        let ceiling = [row("openai", "gpt-5-nano", Usage::Query, true)];
        let got = check_pair(Usage::Query, "ollama", "gemma3", &cat, &ceiling, &[], None);
        assert_eq!(code(got), Some("model_not_eligible"));
    }

    #[test]
    fn pair_disabled_by_workspace_is_model_not_eligible() {
        let cat = [chat("openai", "gpt-5-nano")];
        let narrowing = [row("openai", "gpt-5-nano", Usage::Query, false)];
        let got = check_pair(
            Usage::Query,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &narrowing,
            None,
        );
        assert_eq!(code(got), Some("model_not_eligible"));
    }

    #[test]
    fn known_model_without_the_usage_capability_is_capability_mismatch() {
        let cat = [chat("openai", "gpt-5-nano")];
        // A chat model cannot serve embedding.
        let got = check_pair(
            Usage::Embedding,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &[],
            None,
        );
        assert_eq!(code(got), Some("capability_mismatch"));
    }

    #[test]
    fn discovery_only_model_is_model_not_eligible() {
        let mut entry = chat("ollama", "qwen-local");
        entry.facts.registered = false;
        let got = check_pair(
            Usage::Query,
            "ollama",
            "qwen-local",
            &[entry],
            &[],
            &[],
            None,
        );
        assert_eq!(code(got), Some("model_not_eligible"));
    }

    #[test]
    fn eligible_omits_slug_outside_the_offer() {
        let cat = [chat("openai", "gpt-5-nano"), chat("acme", "local")];
        let offered = std::collections::BTreeSet::from(["openai".to_string()]);
        let got = check_pair(
            Usage::Query,
            "acme",
            "local",
            &cat,
            &[],
            &[],
            Some(&offered),
        );
        assert_eq!(code(got), Some("model_not_eligible"));
        assert!(check_pair(
            Usage::Query,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &[],
            Some(&offered),
        )
        .is_ok());
    }

    #[test]
    fn lock_ignores_withdrawn_grant() {
        let cat = [chat("openai", "gpt-5-nano")];
        let offered = std::collections::BTreeSet::from(["acme".to_string()]);
        let locked = offer_filter(true, &offered);
        assert!(locked.is_none());
        assert!(check_pair(
            Usage::Extract,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &[],
            locked,
        )
        .is_ok());
        let unlocked = offer_filter(false, &offered);
        let got = check_pair(
            Usage::Extract,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &[],
            unlocked,
        );
        assert_eq!(code(got), Some("model_not_eligible"));
    }

    #[test]
    fn model_pair_incomplete_is_422() {
        let got = ensure_pair_complete(Some("openai"), None);
        assert_eq!(code(got), Some("model_pair_incomplete"));
        assert!(ensure_pair_complete(None, None).is_ok());
        assert!(ensure_pair_complete(Some(""), Some("none")).is_ok());
    }

    #[test]
    fn request_pair_while_locked_is_usage_locked() {
        std::env::set_var("EDGEQUAKE_EXTRACT_LLM_MODEL", "pin-model");
        assert_eq!(
            code(refuse_locked_request(Usage::Extract, true)),
            Some("usage_locked")
        );
        assert!(refuse_locked_request(Usage::Query, true).is_ok());
        assert!(refuse_locked_request(Usage::Extract, false).is_ok());
        std::env::remove_var("EDGEQUAKE_EXTRACT_LLM_MODEL");
        std::env::remove_var("EDGEQUAKE_EXTRACT_LLM_PROVIDER");
    }

    #[test]
    fn empty_resolved_pair_is_422() {
        let kept = require_running_pair(false, None, None, None, None).unwrap();
        assert!(kept.is_none());
        let got = require_running_pair(false, Some(""), Some(""), Some(""), Some(""));
        assert_eq!(code(got.map(|_| ())), Some("model_not_eligible"));
    }

    #[test]
    fn family_save_rejects_pair_missing_from_extract() {
        let cat = [chat("openai", "gpt-5-nano"), chat("ollama", "gemma3")];
        let ceiling = [
            row("openai", "gpt-5-nano", Usage::Query, true),
            row("ollama", "gemma3", Usage::Extract, true),
        ];
        let got = ensure_family(&cat, &ceiling, &[], None, "openai", "gpt-5-nano");
        assert_eq!(code(got), Some("model_not_eligible"));
        assert!(ensure_family(&cat, &[], &[], None, "openai", "gpt-5-nano").is_ok());
    }

    #[test]
    fn embedding_save_copies_card_dimension_and_rejects_a_mismatch() {
        let card = CatalogItem {
            entry: CatalogEntry {
                provider: "openai".into(),
                model: "text-embedding-3-small".into(),
                facts: ModelFacts {
                    embedding: true,
                    registered: true,
                    ..ModelFacts::default()
                },
            },
            display_name: "Small".into(),
            provider_display_name: "OpenAI".into(),
            supports_vision: false,
            supports_streaming: false,
            embedding_dimension: Some(1536),
        };
        assert_eq!(
            embedding_dimension_from_card(
                std::slice::from_ref(&card),
                "OpenAI",
                "text-embedding-3-small",
                None
            )
            .unwrap(),
            Some(1536)
        );
        let mismatch =
            embedding_dimension_from_card(&[card], "openai", "text-embedding-3-small", Some(768));
        assert_eq!(
            mismatch.err().map(|err| err.code()),
            Some("embedding_dimension_mismatch")
        );
    }

    #[test]
    fn embedding_only_model_is_capability_mismatch_for_query() {
        let mut embed = chat("openai", "text-embedding-3-small");
        embed.facts.chat = false;
        embed.facts.embedding = true;
        let got = check_pair(
            Usage::Query,
            "openai",
            "text-embedding-3-small",
            &[embed],
            &[],
            &[],
            None,
        );
        assert_eq!(code(got), Some("capability_mismatch"));
    }

    #[test]
    fn decision_tag_is_accepted_when_absent_from_the_chat_catalog() {
        // EC-175-08: tev1 is kept off the chat catalog. Create still sends it
        // as the decision engine default, and that save must not be
        // model_not_eligible.
        let catalog = [chat("mistral", "mistral-large-latest")];
        let offered = std::collections::BTreeSet::from(["mistral".to_string()]);
        let in_chat_catalog = eligible_models(Usage::Decision, &catalog, &[], &[], Some(&offered))
            .iter()
            .any(|entry| entry.model == "tev1:0.8b");
        assert!(!in_chat_catalog);
        assert!(judge_decision_model("tev1:0.8b").is_ok());
        assert!(judge_decision_model("inherit").is_ok());
        assert_eq!(
            judge_decision_model("a b").err().map(|err| err.code()),
            Some("invalid_decision_model")
        );
    }

    #[test]
    fn clear_llm_model_is_not_422() {
        assert!(named_pair(Some(""), Some("none")).unwrap().is_none());
        assert!(named_pair(Some("openai"), Some("gpt-5-nano"))
            .unwrap()
            .is_some());
    }
}
