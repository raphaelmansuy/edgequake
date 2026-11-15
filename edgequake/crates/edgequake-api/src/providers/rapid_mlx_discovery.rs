//! Rapid-MLX model list via `GET {host}/v1/models`.
//!
//! The published discovery service does not know this id. Registration is the
//! extension point. A failure, including a missing optional key, returns an
//! empty list so a stopped local server does not fail catalog refresh.

use async_trait::async_trait;
use chrono::Utc;
use edgequake_llm::discovery::{
    DiscoveredModel, DiscoverySource, DiscoveryStrategy, ModelDiscoveryProvider,
};
use edgequake_llm::model_config::{ModelCapabilities, ModelType};
use edgequake_llm::parse_openai_models_list;

pub struct RapidMlxDiscovery;

impl RapidMlxDiscovery {
    async fn fetch(host: &str, api_key: Option<&str>) -> Option<Vec<DiscoveredModel>> {
        let url = format!("{}/v1/models", host.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .ok()?;
        let mut req = client.get(&url);
        if let Some(key) = api_key.map(str::trim).filter(|key| !key.is_empty()) {
            req = req.bearer_auth(key);
        }
        let resp = req.send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body: serde_json::Value = resp.json().await.ok()?;
        let now = Utc::now();
        let models = parse_openai_models_list(&body)
            .into_iter()
            .map(|id| DiscoveredModel {
                id: id.clone(),
                name: id,
                provider: "rapid-mlx".into(),
                context_length: 0,
                max_output_tokens: 0,
                capabilities: ModelCapabilities {
                    supports_function_calling: true,
                    supports_streaming: true,
                    supports_system_message: true,
                    ..Default::default()
                },
                source: DiscoverySource::DynamicApi,
                discovered_at: now,
                available: true,
                model_type: ModelType::Llm,
                ..Default::default()
            })
            .collect::<Vec<_>>();
        if models.is_empty() {
            None
        } else {
            Some(models)
        }
    }
}

#[async_trait]
impl ModelDiscoveryProvider for RapidMlxDiscovery {
    fn provider_id(&self) -> &str {
        "rapid-mlx"
    }

    fn discovery_strategy(&self) -> DiscoveryStrategy {
        DiscoveryStrategy::Dynamic
    }

    async fn discover_models(&self) -> edgequake_llm::Result<Vec<DiscoveredModel>> {
        let cfg = edgequake_pdf::rapid_mlx::runtime();
        match Self::fetch(&cfg.host, cfg.api_key.as_deref()).await {
            Some(models) => Ok(models),
            None => Ok(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_is_dynamic() {
        let discovery = RapidMlxDiscovery;
        assert_eq!(discovery.provider_id(), "rapid-mlx");
        assert_eq!(discovery.discovery_strategy(), DiscoveryStrategy::Dynamic);
    }
}
