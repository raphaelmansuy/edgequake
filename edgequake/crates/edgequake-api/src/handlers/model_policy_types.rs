//! SPEC-175 W2: wire types and request validation for model policy endpoints.
//!
//! Validation is pure: it turns a request body into [`PolicyRow`]s or a stable error
//! code. The handlers then check authorization and persist.

use std::collections::HashSet;

use crate::services::model_policy_catalog::CatalogItem;
use edgequake_core::model_policy::{tenant_policy_mode, PolicyRow};
use edgequake_core::{Usage, CONFIGURABLE_USAGES};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};

const MAX_PROVIDER_LEN: usize = 64;
const MAX_MODEL_LEN: usize = 256;

/// One policy row on the wire. `enabled` defaults to true.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct PolicyRowDto {
    pub provider: String,
    pub model: String,
    pub usage: String,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
}

fn enabled_default() -> bool {
    true
}

impl PolicyRowDto {
    /// Build a wire row from a stored row (used for responses).
    pub fn from_row(row: &PolicyRow) -> Self {
        Self {
            provider: row.provider.clone(),
            model: row.model.clone(),
            usage: row.usage.as_str().to_string(),
            enabled: row.enabled,
        }
    }
}

/// Body for `PUT .../model-policy`: replaces every row of that scope.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PutPolicyRequest {
    pub rows: Vec<PolicyRowDto>,
}

/// Tenant ceiling. `constrained_usages` lists usages with at least one ceiling row.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TenantPolicyResponse {
    pub tenant_id: String,
    pub constrained_usages: Vec<String>,
    pub rows: Vec<PolicyRowDto>,
}

/// Workspace narrowing. `can_manage` tells the UI whether to show the editor.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkspacePolicyResponse {
    pub workspace_id: String,
    pub can_manage: bool,
    pub rows: Vec<PolicyRowDto>,
}

/// Query string for `GET /models/eligible`. Exactly one scope.
#[derive(Debug, Clone, Deserialize)]
pub struct EligibleQuery {
    pub workspace_id: Option<uuid::Uuid>,
    pub tenant_id: Option<uuid::Uuid>,
    /// `deployment` selects the fleet offer. Any other value is `scope_required`.
    pub scope: Option<String>,
    pub usage: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EligibleModelDto {
    pub provider: String,
    pub provider_display_name: String,
    pub model: String,
    pub display_name: String,
    pub supports_vision: bool,
    pub supports_streaming: bool,
    pub embedding_dimension: Option<i32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EligibleModelsResponse {
    pub usage: String,
    /// `workspace`, `tenant`, or `deployment`.
    pub scope: String,
    /// `constrained` when the tenant ceiling restricts this usage.
    pub policy_mode: String,
    /// `constrained` when this scope has narrowed its parent offer.
    pub offer_mode: String,
    pub models: Vec<EligibleModelDto>,
}

/// Query string for `GET /models/embedding-support`. Exactly one scope.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingSupportQuery {
    pub workspace_id: Option<uuid::Uuid>,
    pub tenant_id: Option<uuid::Uuid>,
    /// `deployment` selects the fleet offer. Any other value is `scope_required`.
    pub scope: Option<String>,
    /// Embedding pair to judge. Both are required.
    pub provider: String,
    pub model: String,
    /// Width to probe at; the model card's width is used when omitted.
    pub dimension: Option<usize>,
}

/// Answer to "may this workspace embed images with this pair, here?" (SPEC-180 08).
///
/// `ok = false` carries the same code and words create would return, so the model step can
/// explain itself before the user reaches Review.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EmbeddingSupportResponse {
    pub provider: String,
    pub model: String,
    /// The pair is eligible for the embedding usage at this scope.
    pub eligible: bool,
    /// The catalog says the model reads images.
    pub multimodal: bool,
    /// A live image embed succeeded. `null` when the pair needs no such probe.
    pub embeds_images: Option<bool>,
    /// Stable code when `ok` is false (`embedding_auth`, `embedding_model_not_multimodal`, …).
    pub code: Option<String>,
    /// One sentence naming the fix.
    pub message: Option<String>,
}

/// Parse a usage name, rejecting unknown names and rerank (not configurable in W2).
pub fn parse_configurable_usage(name: &str) -> ApiResult<Usage> {
    let usage = Usage::from_wire(name.trim())
        .ok_or_else(|| ApiError::ValidationError(format!("unknown usage '{name}'")))?;
    if usage == Usage::Rerank {
        return Err(ApiError::unprocessable_coded(
            "usage_not_configurable",
            "rerank policy is not configurable in this release",
        ));
    }
    Ok(usage)
}

/// Validate a full replace body. Rejects bad names, empty or over-long pairs, and
/// duplicate `(provider, model, usage)` rows.
pub fn parse_policy_rows(rows: &[PolicyRowDto]) -> ApiResult<Vec<PolicyRow>> {
    let mut seen: HashSet<(String, String, &'static str)> = HashSet::new();
    let mut parsed = Vec::with_capacity(rows.len());
    for dto in rows {
        let row = parse_row(dto)?;
        let key = (row.provider.clone(), row.model.clone(), row.usage.as_str());
        if !seen.insert(key) {
            return Err(ApiError::ValidationError(format!(
                "duplicate policy row for {}/{} ({})",
                row.provider,
                row.model,
                row.usage.as_str()
            )));
        }
        parsed.push(row);
    }
    Ok(parsed)
}

fn parse_row(dto: &PolicyRowDto) -> ApiResult<PolicyRow> {
    let usage = parse_configurable_usage(&dto.usage)?;
    let provider = dto.provider.trim().to_lowercase();
    let model = dto.model.trim().to_string();
    if provider.is_empty() || provider.chars().count() > MAX_PROVIDER_LEN {
        return Err(ApiError::ValidationError(format!(
            "provider must be 1-{MAX_PROVIDER_LEN} characters"
        )));
    }
    if model.is_empty() || model.chars().count() > MAX_MODEL_LEN {
        return Err(ApiError::ValidationError(format!(
            "model must be 1-{MAX_MODEL_LEN} characters"
        )));
    }
    Ok(PolicyRow {
        provider,
        model,
        usage,
        enabled: dto.enabled,
    })
}

// ---------------------------------------------------------------------------
// Tenant catalog (SPEC-175 W4): the ceiling editor's choices
// ---------------------------------------------------------------------------

/// One registered deployment model. Discovery-only models are never listed: they can
/// never be eligible, so they cannot be a ceiling choice.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
pub struct CatalogEntryDto {
    pub provider: String,
    pub provider_display_name: String,
    pub model: String,
    pub display_name: String,
    /// `healthy` (last probe ok), `probe_failed` (last probe failed), `unknown` (never probed).
    pub health: String,
    /// Configurable usages this model can serve, in `CONFIGURABLE_USAGES` order.
    pub usages: Vec<String>,
}

/// `GET /tenants/{id}/model-catalog`: registered models and the tenant ceiling mode.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TenantCatalogResponse {
    pub policy_mode: String,
    pub entries: Vec<CatalogEntryDto>,
}

/// Pure: the registered catalog as the ceiling editor reads it.
pub fn catalog_view(catalog: &[CatalogItem], ceiling: &[PolicyRow]) -> TenantCatalogResponse {
    TenantCatalogResponse {
        policy_mode: tenant_policy_mode(ceiling).as_str().to_string(),
        entries: catalog
            .iter()
            .filter(|item| item.entry.facts.registered)
            .map(catalog_entry_dto)
            .collect(),
    }
}

fn catalog_entry_dto(item: &CatalogItem) -> CatalogEntryDto {
    let entry = &item.entry;
    CatalogEntryDto {
        provider: entry.provider.clone(),
        provider_display_name: item.provider_display_name.clone(),
        model: entry.model.clone(),
        display_name: item.display_name.clone(),
        health: health_label(entry.facts.probe).to_string(),
        usages: CONFIGURABLE_USAGES
            .iter()
            .filter(|usage| entry.facts.supports(**usage))
            .map(|usage| usage.as_str().to_string())
            .collect(),
    }
}

fn health_label(probe: Option<bool>) -> &'static str {
    match probe {
        Some(true) => "healthy",
        Some(false) => "probe_failed",
        None => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto(provider: &str, model: &str, usage: &str, enabled: bool) -> PolicyRowDto {
        PolicyRowDto {
            provider: provider.into(),
            model: model.into(),
            usage: usage.into(),
            enabled,
        }
    }

    #[test]
    fn normalizes_provider_case_and_trims_model() {
        let rows = parse_policy_rows(&[dto(" OpenAI ", "  gpt-5-nano ", "query", true)]).unwrap();
        assert_eq!(rows[0].provider, "openai");
        assert_eq!(rows[0].model, "gpt-5-nano");
        assert_eq!(rows[0].usage, Usage::Query);
    }

    #[test]
    fn rerank_is_rejected_with_stable_code() {
        let err = parse_policy_rows(&[dto("openai", "x", "rerank", true)]).unwrap_err();
        assert_eq!(err.code(), "usage_not_configurable");
        assert_eq!(
            err.status_code(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn unknown_usage_is_a_validation_error() {
        let err = parse_policy_rows(&[dto("openai", "x", "chat", true)]).unwrap_err();
        assert_eq!(
            err.status_code(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn blank_provider_or_model_is_rejected() {
        assert!(parse_policy_rows(&[dto("  ", "x", "query", true)]).is_err());
        assert!(parse_policy_rows(&[dto("openai", "   ", "query", true)]).is_err());
    }

    #[test]
    fn duplicate_pair_for_same_usage_is_rejected() {
        let body = [
            dto("openai", "gpt-5-nano", "query", true),
            dto("OPENAI", "gpt-5-nano", "query", false),
        ];
        assert!(parse_policy_rows(&body).is_err());
    }

    #[test]
    fn same_pair_for_different_usages_is_allowed() {
        let body = [
            dto("openai", "gpt-5-nano", "query", true),
            dto("openai", "gpt-5-nano", "extract", true),
        ];
        assert_eq!(parse_policy_rows(&body).unwrap().len(), 2);
    }

    #[test]
    fn enabled_defaults_to_true_on_the_wire() {
        let row: PolicyRowDto =
            serde_json::from_str(r#"{"provider":"openai","model":"m","usage":"query"}"#).unwrap();
        assert!(row.enabled);
    }
}

#[cfg(test)]
mod catalog_view_tests {
    use super::*;
    use crate::services::model_policy_catalog::CatalogItem;
    use edgequake_core::model_policy::{CatalogEntry, ModelFacts};

    fn item(model: &str, registered: bool, probe: Option<bool>, chat: bool) -> CatalogItem {
        CatalogItem {
            entry: CatalogEntry {
                provider: "openai".into(),
                model: model.into(),
                facts: ModelFacts {
                    chat,
                    vision: false,
                    embedding: false,
                    registered,
                    probe,
                },
            },
            display_name: format!("{model} (display)"),
            provider_display_name: "OpenAI".into(),
            supports_vision: false,
            supports_streaming: true,
            embedding_dimension: None,
        }
    }

    fn ceiling_row() -> PolicyRow {
        PolicyRow {
            provider: "openai".into(),
            model: "gpt-4.1-mini".into(),
            usage: Usage::Query,
            enabled: true,
        }
    }

    #[test]
    fn discovery_only_models_are_not_listed() {
        let catalog = [
            item("registered", true, None, true),
            item("dynamic", false, None, true),
        ];
        let view = catalog_view(&catalog, &[]);
        assert_eq!(view.entries.len(), 1);
        assert_eq!(view.entries[0].model, "registered");
    }

    #[test]
    fn health_maps_probe_states() {
        let catalog = [
            item("ok", true, Some(true), true),
            item("bad", true, Some(false), true),
            item("never", true, None, true),
        ];
        let view = catalog_view(&catalog, &[]);
        let health: Vec<&str> = view.entries.iter().map(|e| e.health.as_str()).collect();
        assert_eq!(health, ["healthy", "probe_failed", "unknown"]);
    }

    #[test]
    fn usages_follow_configurable_order_and_capability() {
        let catalog = [item("chat-only", true, None, true)];
        let usages = catalog_view(&catalog, &[]).entries[0].usages.clone();
        assert_eq!(
            usages,
            ["extract", "query", "summary", "keyword", "decision"]
        );
        assert!(!usages.iter().any(|u| u == "rerank"));
    }

    #[test]
    fn mode_is_constrained_only_when_the_ceiling_has_rows() {
        let catalog = [item("m", true, None, true)];
        assert_eq!(catalog_view(&catalog, &[]).policy_mode, "unconstrained");
        assert_eq!(
            catalog_view(&catalog, &[ceiling_row()]).policy_mode,
            "constrained"
        );
    }
}
