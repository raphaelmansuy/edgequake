//! First-principles LLM credential prerequisites.
//!
//! **Goal:** only select providers that can authenticate in this runtime.
//!
//! **Ladder (query-time):**
//! 1. Request override — if credentials for that provider are configured.
//! 2. Workspace override — same gate; creation/auth failures fall through.
//! 3. Server default — startup `from_env()` provider (no override).
//! 4. Runtime auth rejection on override — retry without override (see `query_execution`).

use edgequake_llm::model_config::ProviderConfig;
use edgequake_llm::ModelsConfig;

use crate::provider_types::ConfigRequirement;

/// How a provider authenticates at runtime (SPEC-043 §011).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialKind {
    /// Static API key env var (OpenAI, Anthropic, …).
    StaticApiKey,
    /// OAuth2 identity ladder (Vertex AI).
    OAuth2Identity,
    /// No auth (Ollama, LM Studio, mock).
    LocalNoAuth,
    /// AWS credential chain (Bedrock).
    AwsChain,
    /// Unknown / provider-specific passthrough.
    Passthrough,
}

impl CredentialKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StaticApiKey => "api_key",
            Self::OAuth2Identity => "oauth2_identity",
            Self::LocalNoAuth => "local",
            Self::AwsChain => "aws_chain",
            Self::Passthrough => "passthrough",
        }
    }
}

/// Credential kind for a provider id.
pub fn credential_kind_for(provider_name: &str) -> CredentialKind {
    match provider_name.to_ascii_lowercase().as_str() {
        "mock" | "ollama" | "lmstudio" | "lm-studio" | "lm_studio" => CredentialKind::LocalNoAuth,
        "vertexai" | "vertex" => CredentialKind::OAuth2Identity,
        "bedrock" | "aws-bedrock" => CredentialKind::AwsChain,
        "vscode-copilot" | "copilot" => CredentialKind::Passthrough,
        "openai" | "anthropic" | "gemini" | "google" | "mistral" | "xai" | "openrouter"
        | "nvidia" | "cohere" | "jina" | "huggingface" | "hf" | "azure" => {
            CredentialKind::StaticApiKey
        }
        _ => CredentialKind::Passthrough,
    }
}

/// True when the provider's runtime prerequisites are satisfied (key env non-empty, etc.).
///
/// This does **not** prove the key is valid — only that configuration exists.
/// Invalid keys are handled at execution time via auth fallback.
pub fn provider_credentials_configured(provider: &ProviderConfig) -> bool {
    match provider.name.as_str() {
        "mock" | "ollama" | "lmstudio" => true,
        "vertexai" => vertex_auth_configured_sync(),
        _ => {
            if let Some(env) = provider.api_key_env.as_ref().filter(|s| !s.is_empty()) {
                env_non_empty(env)
            } else {
                true
            }
        }
    }
}

/// Lookup provider in [`ModelsConfig`] and apply the credential gate.
pub fn llm_provider_credentials_configured(config: &ModelsConfig, provider_name: &str) -> bool {
    config
        .providers
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(provider_name))
        .map(provider_credentials_configured)
        .unwrap_or_else(|| llm_provider_credentials_configured_by_name(provider_name))
}

/// Fallback when models.toml has no entry (tests, unknown ids).
pub fn llm_provider_credentials_configured_by_name(provider_name: &str) -> bool {
    match provider_name.to_ascii_lowercase().as_str() {
        "mock" | "ollama" | "lmstudio" => true,
        "openai" => env_non_empty("OPENAI_API_KEY"),
        "mistral" => env_non_empty("MISTRAL_API_KEY"),
        "anthropic" => env_non_empty("ANTHROPIC_API_KEY"),
        "google" | "gemini" => env_non_empty("GOOGLE_API_KEY") || env_non_empty("GEMINI_API_KEY"),
        "vertexai" | "vertex" => vertex_auth_configured_sync(),
        "nvidia" => env_non_empty("NVIDIA_API_KEY"),
        "cohere" => env_non_empty("COHERE_API_KEY"),
        "jina" => env_non_empty("JINA_API_KEY"),
        "huggingface" | "hf" => env_non_empty("HF_TOKEN") || env_non_empty("HUGGINGFACE_TOKEN"),
        "vscode-copilot" | "copilot" => true,
        _ => true,
    }
}

/// Structured config requirements for settings / health APIs.
pub fn provider_config_requirements(provider: &ProviderConfig) -> Vec<ConfigRequirement> {
    let mut reqs = Vec::new();

    if let Some(env) = provider.api_key_env.as_ref().filter(|s| !s.is_empty()) {
        reqs.push(ConfigRequirement {
            env_var: env.clone(),
            required: true,
            description: format!("{} API key", provider.display_name),
            satisfied: env_non_empty(env),
        });
    }

    if let Some(env) = provider.base_url_env.as_ref().filter(|s| !s.is_empty()) {
        reqs.push(ConfigRequirement {
            env_var: env.clone(),
            required: false,
            description: format!("{} base URL override", provider.display_name),
            satisfied: std::env::var(env).is_ok(),
        });
    }

    match provider.name.as_str() {
        "vertexai" => reqs.extend(vertex_config_requirements()),
        "ollama" => {
            reqs.push(ConfigRequirement {
                env_var: "OLLAMA_HOST".to_string(),
                required: false,
                description: "Ollama server URL (default: http://localhost:11434)".to_string(),
                satisfied: true,
            });
        }
        "lmstudio" => {
            reqs.push(ConfigRequirement {
                env_var: "LMSTUDIO_HOST".to_string(),
                required: false,
                description: "LM Studio server URL (default: http://localhost:1234)".to_string(),
                satisfied: true,
            });
        }
        "azure" if !reqs.iter().any(|r| r.env_var == "AZURE_OPENAI_ENDPOINT") => {
            reqs.push(ConfigRequirement {
                env_var: "AZURE_OPENAI_ENDPOINT".to_string(),
                required: true,
                description: "Azure OpenAI endpoint URL".to_string(),
                satisfied: std::env::var("AZURE_OPENAI_ENDPOINT").is_ok(),
            });
        }
        _ => {}
    }

    reqs
}

/// Human-readable health error for cloud providers (never "API key" for Vertex).
pub fn provider_credentials_health_error(provider: &ProviderConfig) -> String {
    match provider.name.as_str() {
        "vertexai" => vertex_auth_health_error(),
        _ => {
            let env_hint = provider
                .api_key_env
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or("API key");
            format!("{env_hint} not configured")
        }
    }
}

// ── Vertex AI identity auth (SPEC-043 §011) ─────────────────────────────────

/// Sync gate: project + at least one auth source (env/file/gcloud).
pub fn vertex_auth_configured_sync() -> bool {
    env_non_empty("GOOGLE_CLOUD_PROJECT")
        && (env_non_empty("GOOGLE_ACCESS_TOKEN")
            || service_account_credentials_file_exists()
            || adc_well_known_file_exists()
            || gcloud_adc_token_available())
}

/// Async gate: includes GCE/GKE/Cloud Run metadata server.
pub async fn vertex_auth_configured() -> bool {
    vertex_auth_configured_sync() || vertex_metadata_available().await
}

/// Config requirements for Vertex AI (identity auth, not API key).
pub fn vertex_config_requirements() -> Vec<ConfigRequirement> {
    vec![
        ConfigRequirement {
            env_var: "GOOGLE_CLOUD_PROJECT".to_string(),
            required: true,
            description: "Google Cloud project ID".to_string(),
            satisfied: env_non_empty("GOOGLE_CLOUD_PROJECT"),
        },
        ConfigRequirement {
            env_var: "GOOGLE_CLOUD_REGION".to_string(),
            required: false,
            description: "GCP region (default: us-central1)".to_string(),
            satisfied: std::env::var("GOOGLE_CLOUD_REGION").is_ok()
                || std::env::var("GOOGLE_CLOUD_LOCATION").is_ok(),
        },
        ConfigRequirement {
            env_var: "GOOGLE_ACCESS_TOKEN".to_string(),
            required: false,
            description: "Short-lived OAuth2 token (CI/debug)".to_string(),
            satisfied: env_non_empty("GOOGLE_ACCESS_TOKEN"),
        },
        ConfigRequirement {
            env_var: "GOOGLE_APPLICATION_CREDENTIALS".to_string(),
            required: false,
            description: "Service account JSON or WIF config path".to_string(),
            satisfied: service_account_credentials_file_exists(),
        },
        ConfigRequirement {
            env_var: "ADC".to_string(),
            required: false,
            description: "Application Default Credentials (gcloud auth application-default login)"
                .to_string(),
            satisfied: adc_well_known_file_exists() || gcloud_adc_token_available(),
        },
    ]
}

/// Error message when Vertex identity is missing or cannot mint a token.
pub fn vertex_auth_health_error() -> String {
    if !env_non_empty("GOOGLE_CLOUD_PROJECT") {
        return "GOOGLE_CLOUD_PROJECT not set".to_string();
    }
    if service_account_credentials_file_exists() && !gcloud_adc_token_available() {
        return "Service account key set; run `gcloud auth application-default login \
                --impersonate-service-account=...` or deploy on GCP with an attached service account"
            .to_string();
    }
    "No Vertex identity configured (ADC, service account, or GOOGLE_ACCESS_TOKEN)".to_string()
}

/// Live probe: can we obtain a token right now?
pub async fn probe_vertex_auth_live() -> Result<(), String> {
    if env_non_empty("GOOGLE_ACCESS_TOKEN") {
        return Ok(());
    }
    if vertex_metadata_available().await {
        return Ok(());
    }
    if gcloud_adc_token_available() {
        return Ok(());
    }
    if service_account_credentials_file_exists() {
        return Err(
            "Service account key set; run `gcloud auth application-default login \
             --impersonate-service-account=...` or deploy on GCP with an attached service account"
                .to_string(),
        );
    }
    Err(vertex_auth_health_error())
}

fn env_non_empty(name: &str) -> bool {
    std::env::var(name)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

fn service_account_credentials_file_exists() -> bool {
    std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .ok()
        .filter(|p| !p.trim().is_empty())
        .map(|p| std::path::Path::new(p.trim()).is_file())
        .unwrap_or(false)
}

fn adc_well_known_file_exists() -> bool {
    std::env::var("HOME")
        .ok()
        .map(|home| {
            std::path::Path::new(&home)
                .join(".config/gcloud/application_default_credentials.json")
                .is_file()
        })
        .unwrap_or(false)
}

fn gcloud_adc_token_available() -> bool {
    use std::process::Command;

    Command::new("gcloud")
        .args(["auth", "application-default", "print-access-token"])
        .output()
        .map(|out| out.status.success() && !out.stdout.is_empty())
        .unwrap_or(false)
}

async fn vertex_metadata_available() -> bool {
    reqwest::Client::new()
        .get("http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token")
        .header("Metadata-Flavor", "Google")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::ModelsConfig;

    fn save_env(key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    fn restore_env(key: &str, prev: Option<String>) {
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn openai_requires_non_empty_key_env() {
        let prev = save_env("OPENAI_API_KEY");
        std::env::set_var("OPENAI_API_KEY", "");
        assert!(!llm_provider_credentials_configured_by_name("openai"));
        restore_env("OPENAI_API_KEY", prev);
    }

    #[test]
    fn mock_always_configured() {
        assert!(llm_provider_credentials_configured_by_name("mock"));
    }

    #[test]
    fn vertex_requires_project_and_accepts_explicit_token() {
        let prev_project = save_env("GOOGLE_CLOUD_PROJECT");
        let prev_token = save_env("GOOGLE_ACCESS_TOKEN");

        std::env::remove_var("GOOGLE_CLOUD_PROJECT");
        std::env::remove_var("GOOGLE_ACCESS_TOKEN");
        assert!(!vertex_auth_configured_sync());

        std::env::set_var("GOOGLE_CLOUD_PROJECT", "my-project");
        std::env::set_var("GOOGLE_ACCESS_TOKEN", "tok");
        assert!(vertex_auth_configured_sync());

        restore_env("GOOGLE_CLOUD_PROJECT", prev_project);
        restore_env("GOOGLE_ACCESS_TOKEN", prev_token);
    }

    #[test]
    fn vertex_health_error_never_mentions_api_key() {
        let prev_project = save_env("GOOGLE_CLOUD_PROJECT");
        std::env::remove_var("GOOGLE_CLOUD_PROJECT");
        let msg = vertex_auth_health_error();
        assert!(!msg.to_ascii_lowercase().contains("api key"));
        assert!(msg.contains("GOOGLE_CLOUD_PROJECT"));
        restore_env("GOOGLE_CLOUD_PROJECT", prev_project);
    }

    #[test]
    fn vertex_credential_kind_is_oauth2() {
        assert_eq!(
            credential_kind_for("vertexai"),
            CredentialKind::OAuth2Identity
        );
        assert_eq!(credential_kind_for("openai"), CredentialKind::StaticApiKey);
    }

    #[test]
    fn uses_models_config_when_present() {
        let config = ModelsConfig::builtin_defaults();
        let openai = config.providers.iter().find(|p| p.name == "openai");
        if let Some(p) = openai {
            let prev = save_env("OPENAI_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
            assert!(!provider_credentials_configured(p));
            restore_env("OPENAI_API_KEY", prev);
        }
    }

    #[test]
    fn vertex_config_requirements_include_project() {
        let reqs = vertex_config_requirements();
        assert!(reqs
            .iter()
            .any(|r| r.env_var == "GOOGLE_CLOUD_PROJECT" && r.required));
        assert!(!reqs
            .iter()
            .any(|r| r.description.to_ascii_lowercase().contains("api key")));
    }
}
