//! UI provider visibility rules (SPEC-043).
//!
//! Mock and other internal-only providers must never appear in WebUI pickers or
//! settings catalogs. Tests and server runtime may still use Mock via env/factory.

use std::collections::HashSet;

use edgequake_llm::model_config::ProviderConfig;

/// Provider IDs that must never appear in user-facing API responses or WebUI.
pub const UI_HIDDEN_PROVIDER_IDS: &[&str] = &["mock", "mock-imagegen"];

/// Returns true when the provider is the internal mock/test integration.
pub fn is_mock_provider(provider_id: &str) -> bool {
    matches!(
        provider_id.trim().to_lowercase().as_str(),
        "mock" | "mock-imagegen"
    )
}

/// Whether a provider ID may be shown in pickers, settings, and catalog APIs.
pub fn is_ui_visible_provider_id(provider_id: &str) -> bool {
    !is_mock_provider(provider_id)
}

/// Whether a configured provider may be shown in the UI.
pub fn is_ui_visible_provider(provider: &ProviderConfig) -> bool {
    provider.enabled && is_ui_visible_provider_id(&provider.name)
}

/// Filter enabled providers for UI, respecting allowlist and mock exclusion.
pub fn filter_ui_providers<'a>(
    providers: &'a [ProviderConfig],
    allowed: &Option<HashSet<String>>,
) -> Vec<&'a ProviderConfig> {
    crate::model_catalog::filter_providers(providers, allowed)
        .into_iter()
        .filter(|p| is_ui_visible_provider_id(&p.name))
        .collect()
}

/// Chat-capable provider IDs from edgequake-llm that EdgeQuake should expose via models.toml.
pub fn expected_chat_provider_ids() -> Vec<&'static str> {
    edgequake_llm::provider_catalog::ProviderCatalog::list_llm_providers()
        .into_iter()
        .filter(|id| is_ui_visible_provider_id(id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_is_hidden_from_ui() {
        assert!(is_mock_provider("mock"));
        assert!(is_mock_provider("MOCK"));
        assert!(is_ui_visible_provider_id("openai"));
        assert!(!is_ui_visible_provider_id("mock"));
    }

    #[test]
    fn filter_ui_providers_excludes_mock() {
        use edgequake_llm::ProviderConfig;
        let providers = vec![
            ProviderConfig {
                name: "openai".into(),
                enabled: true,
                ..ProviderConfig::default()
            },
            ProviderConfig {
                name: "mock".into(),
                enabled: true,
                ..ProviderConfig::default()
            },
        ];
        let filtered = filter_ui_providers(&providers, &None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "openai");
    }
}
