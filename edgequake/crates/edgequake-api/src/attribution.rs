//! Application attribution context for upstream LLM provider calls (SPEC-043).

use std::collections::HashMap;

use edgequake_llm::{
    application_context::ApplicationContext,
    provider_catalog::{AttributionSupport, ProviderCatalog, ProviderDescriptor},
};

/// Merge env defaults, ingress `x-edgequake-*` headers, and propagation headers.
pub fn build_application_context(
    propagation_headers: Option<&HashMap<String, String>>,
    end_user_id: Option<String>,
) -> ApplicationContext {
    let mut ctx = ApplicationContext::from_env();

    if let Some(headers) = propagation_headers {
        if let Ok(ingress) = ApplicationContext::from_ingress_headers(headers) {
            ctx.merge(ingress);
        }
        for key in [
            "x-request-id",
            "x-correlation-id",
            "traceparent",
            "tracestate",
        ] {
            if let Some(value) = headers.get(key) {
                if key == "x-request-id" && ctx.request_id.is_none() {
                    ctx.request_id = Some(value.clone());
                } else {
                    ctx.extra_headers
                        .entry(key.to_string())
                        .or_insert_with(|| value.clone());
                }
            }
        }
        if let Some(tenant) = headers.get("x-tenant-id") {
            ctx.tenant_id.get_or_insert_with(|| tenant.clone());
        }
    }

    if let Some(uid) = end_user_id {
        ctx.end_user_id = Some(uid);
    }

    ctx
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ProviderAttributionInfo {
    pub id: String,
    pub display_name: String,
    pub attribution_support: String,
    pub headers: Vec<String>,
    pub body_fields: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct AttributionSettingsResponse {
    pub effective_context: EffectiveContextResponse,
    pub providers: Vec<ProviderAttributionInfo>,
    pub ingress_headers: Vec<String>,
    pub environment_variables: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct EffectiveContextResponse {
    pub app_id: Option<String>,
    pub app_name: Option<String>,
    pub app_url: Option<String>,
    pub tenant_id: Option<String>,
    pub request_id: Option<String>,
    pub end_user_id: Option<String>,
    pub active: bool,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct HealthAttributionSummary {
    pub app_id: Option<String>,
    pub app_name: Option<String>,
    pub active: bool,
}

pub fn health_attribution_summary() -> HealthAttributionSummary {
    let ctx = ApplicationContext::from_env();
    HealthAttributionSummary {
        app_id: ctx.app_id.clone(),
        app_name: ctx.app_name.clone(),
        active: ctx.has_app_attribution(),
    }
}

pub fn build_attribution_settings_response() -> AttributionSettingsResponse {
    let ctx = ApplicationContext::from_env();
    let mut sources = Vec::new();
    if std::env::var("EDGEQUAKE_APP_ID").is_ok() {
        sources.push("env:EDGEQUAKE_APP_ID".into());
    }
    if std::env::var("EDGEQUAKE_APP_NAME").is_ok() {
        sources.push("env:EDGEQUAKE_APP_NAME".into());
    }
    if std::env::var("EDGEQUAKE_APP_URL").is_ok() {
        sources.push("env:EDGEQUAKE_APP_URL".into());
    }
    if std::env::var("EDGEQUAKE_TENANT_ID").is_ok() {
        sources.push("env:EDGEQUAKE_TENANT_ID".into());
    }

    AttributionSettingsResponse {
        effective_context: EffectiveContextResponse {
            app_id: ctx.app_id.clone(),
            app_name: ctx.app_name.clone(),
            app_url: ctx.app_url.clone(),
            tenant_id: ctx.tenant_id.clone(),
            request_id: ctx.request_id.clone(),
            end_user_id: ctx.end_user_id.clone(),
            active: ctx.has_app_attribution(),
            sources,
        },
        providers: ProviderCatalog::all()
            .iter()
            .filter(|d| {
                d.features.chat && crate::provider_visibility::is_ui_visible_provider_id(d.id)
            })
            .map(provider_attribution_info)
            .collect(),
        ingress_headers: vec![
            "x-edgequake-app-id".into(),
            "x-edgequake-app-name".into(),
            "x-edgequake-app-url".into(),
            "x-edgequake-tenant-id".into(),
            "x-edgequake-request-id".into(),
        ],
        environment_variables: vec![
            "EDGEQUAKE_APP_ID".into(),
            "EDGEQUAKE_APP_NAME".into(),
            "EDGEQUAKE_APP_URL".into(),
            "EDGEQUAKE_TENANT_ID".into(),
        ],
    }
}

fn provider_attribution_info(descriptor: &ProviderDescriptor) -> ProviderAttributionInfo {
    let kind = edgequake_llm::http::attribution::attribution_kind_from_provider_name(descriptor.id);
    let resolved = edgequake_llm::http::attribution::resolve_attribution(
        kind,
        &ApplicationContext {
            app_id: Some("edgequake".into()),
            app_name: Some("EdgeQuake".into()),
            app_url: Some("https://edgequake.local".into()),
            request_id: Some("sample-req".into()),
            end_user_id: Some("user-sample".into()),
            ..Default::default()
        },
    );

    ProviderAttributionInfo {
        id: descriptor.id.to_string(),
        display_name: humanize_provider_id(descriptor.id),
        attribution_support: attribution_support_label(descriptor.attribution).to_string(),
        headers: resolved.headers.keys().cloned().collect(),
        body_fields: resolved.body_fields.keys().cloned().collect(),
    }
}

fn attribution_support_label(support: AttributionSupport) -> &'static str {
    match support {
        AttributionSupport::Full => "full",
        AttributionSupport::Passthrough => "passthrough",
        AttributionSupport::ObservabilityOnly => "observability_only",
        AttributionSupport::None => "none",
    }
}

fn humanize_provider_id(id: &str) -> String {
    match id {
        "openai" => "OpenAI".into(),
        "azure" => "Azure OpenAI".into(),
        "anthropic" => "Anthropic".into(),
        "gemini" => "Google Gemini".into(),
        "vertexai" => "Google Vertex AI".into(),
        "openrouter" => "OpenRouter".into(),
        "mistral" => "Mistral AI".into(),
        "nvidia" => "NVIDIA NIM".into(),
        "cohere" => "Cohere".into(),
        "bedrock" => "AWS Bedrock".into(),
        "xai" => "xAI".into(),
        "huggingface" => "HuggingFace".into(),
        "lmstudio" => "LM Studio".into(),
        "ollama" => "Ollama".into(),
        "vscode-copilot" => "GitHub Copilot".into(),
        "jina" => "Jina AI".into(),
        "mock" => "Mock Provider".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_context_merges_propagation_headers() {
        let mut headers = HashMap::new();
        headers.insert("x-request-id".into(), "req-1".into());
        headers.insert("traceparent".into(), "00-abc".into());
        let ctx = build_application_context(Some(&headers), Some("user-42".into()));
        assert_eq!(ctx.request_id.as_deref(), Some("req-1"));
        assert_eq!(ctx.end_user_id.as_deref(), Some("user-42"));
        assert_eq!(
            ctx.extra_headers.get("traceparent").map(String::as_str),
            Some("00-abc")
        );
    }

    #[test]
    fn attribution_catalog_excludes_mock() {
        let resp = build_attribution_settings_response();
        assert!(resp.providers.iter().any(|p| p.id == "openai"));
        assert!(!resp.providers.iter().any(|p| p.id == "mock"));
    }
}
