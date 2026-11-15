//! Rapid-MLX client.
//!
//! The published `edgequake-llm` 0.10.10 factory has no `rapid-mlx` id. Its
//! public [`LocalOpenAiIdentity`] is the extension point for a local
//! OpenAI-shaped server, so this module owns that identity.
//!
//! It lives in `edgequake-pdf` because PDF vision calls
//! `ProviderFactory::create_llm_provider` directly when no client is injected,
//! and `edgequake-api` already depends on this crate. One identity then serves
//! vision, boot, and workspace clients without a crate cycle.
//!
//! Rapid-MLX is keyless unless the server was started with `--api-key`. The
//! unauthenticated process probe is `GET {host}/health`. Chat, models, and
//! embeddings stay under `/v1`.

use std::sync::Arc;

use edgequake_llm::{
    ApplicationContext, EmbeddingProvider, LLMProvider, LocalOpenAiIdentity, LocalOpenAiProvider,
    LocalOpenAiRuntimeConfig, Result,
};

/// Canonical id. Aliases `rapid_mlx` and `rapidmlx` collapse to this.
pub const PROVIDER_ID: &str = "rapid-mlx";

/// Loopback host without `/v1`. The client appends `/v1`.
pub const DEFAULT_HOST: &str = "http://127.0.0.1:8000";

/// Host the container uses when Rapid-MLX runs on the Docker host.
pub const DOCKER_HOST: &str = "http://host.docker.internal:8000";

pub const RAPID_MLX_IDENTITY: LocalOpenAiIdentity = LocalOpenAiIdentity {
    id: PROVIDER_ID,
    display_name: "Rapid-MLX",
    default_host: DEFAULT_HOST,
    host_envs: &["RAPID_MLX_HOST", "RAPID_MLX_BASE_URL"],
    key_envs: &["RAPID_MLX_API_KEY"],
    model_env: "RAPID_MLX_MODEL",
    embedding_model_env: "RAPID_MLX_EMBEDDING_MODEL",
    timeout_env: "RAPID_MLX_TIMEOUT_SECONDS",
    placeholder_key: "rapid-mlx",
    default_model: "default",
    default_timeout_secs: 600,
    default_context: 128_000,
    max_output_tokens: 4096,
};

/// True for `rapid-mlx`, `rapid_mlx`, and `rapidmlx`.
pub fn is_rapid_mlx(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "rapid-mlx" | "rapid_mlx" | "rapidmlx"
    )
}

/// Host and optional key from the environment, then the compiled default.
pub fn runtime() -> LocalOpenAiRuntimeConfig {
    RAPID_MLX_IDENTITY.resolve_runtime()
}

/// `GET` URL for the unauthenticated process probe.
pub fn health_url() -> String {
    format!("{}/health", runtime().host.trim_end_matches('/'))
}

/// Process-default chat client. `RAPID_MLX_MODEL` wins, then `EDGEQUAKE_LLM_MODEL`.
pub fn chat_from_env() -> Result<Arc<dyn LLMProvider>> {
    chat_named("", None)
}

/// Chat client for an explicit model. An empty model uses the env resolution above.
pub fn chat_named(model: &str, ctx: Option<ApplicationContext>) -> Result<Arc<dyn LLMProvider>> {
    let chat = chat_model(model);
    let embed = embedding_model("");
    Ok(Arc::new(build(&chat, &embed, ctx)?))
}

/// Process-default embedding client on the same host.
pub fn embedding_from_env() -> Result<Arc<dyn EmbeddingProvider>> {
    embedding_named("")
}

/// Embedding client. An empty model uses `RAPID_MLX_EMBEDDING_MODEL`, then
/// `EDGEQUAKE_EMBEDDING_MODEL`, then the chat model.
pub fn embedding_named(model: &str) -> Result<Arc<dyn EmbeddingProvider>> {
    let chat = chat_model("");
    let embed = embedding_model(model);
    Ok(Arc::new(build(&chat, &embed, None)?))
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn chat_model(explicit: &str) -> String {
    let explicit = explicit.trim();
    if !explicit.is_empty() {
        return explicit.to_string();
    }
    runtime()
        .default_model
        .or_else(|| non_empty_env("EDGEQUAKE_LLM_MODEL"))
        .unwrap_or_else(|| RAPID_MLX_IDENTITY.default_model.to_string())
}

fn embedding_model(explicit: &str) -> String {
    let explicit = explicit.trim();
    if !explicit.is_empty() {
        return explicit.to_string();
    }
    non_empty_env(RAPID_MLX_IDENTITY.embedding_model_env)
        .or_else(|| non_empty_env("EDGEQUAKE_EMBEDDING_MODEL"))
        .unwrap_or_else(|| chat_model(""))
}

fn build(chat: &str, embed: &str, ctx: Option<ApplicationContext>) -> Result<LocalOpenAiProvider> {
    let cfg = runtime();
    let timeout = std::env::var(RAPID_MLX_IDENTITY.timeout_env)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(RAPID_MLX_IDENTITY.default_timeout_secs);
    let mut builder = LocalOpenAiProvider::builder(RAPID_MLX_IDENTITY)
        .host(cfg.host)
        .model(chat)
        .embedding_model(embed)
        .timeout_seconds(timeout);
    if let Some(key) = cfg.api_key {
        builder = builder.api_key(key);
    }
    let mut provider = builder.build()?;
    if let Some(ctx) = ctx {
        provider = provider.with_application_context(ctx);
    }
    Ok(provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_cleared_env(f: impl FnOnce()) {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let keys = [
            "RAPID_MLX_HOST",
            "RAPID_MLX_BASE_URL",
            "RAPID_MLX_API_KEY",
            "RAPID_MLX_MODEL",
            "RAPID_MLX_EMBEDDING_MODEL",
            "EDGEQUAKE_LLM_MODEL",
            "EDGEQUAKE_EMBEDDING_MODEL",
        ];
        let saved: Vec<_> = keys
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        for key in keys {
            std::env::remove_var(key);
        }
        f();
        for (key, value) in saved {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }

    #[test]
    fn aliases_collapse() {
        assert!(is_rapid_mlx("rapid-mlx"));
        assert!(is_rapid_mlx(" RAPID_MLX "));
        assert!(is_rapid_mlx("rapidmlx"));
        assert!(!is_rapid_mlx("omlx"));
        assert!(!is_rapid_mlx("vllm-mlx"));
    }

    #[test]
    fn keyless_default_strips_v1_and_reads_an_optional_key() {
        with_cleared_env(|| {
            assert_eq!(runtime().host, DEFAULT_HOST);
            assert!(runtime().api_key.is_none());
            assert_eq!(health_url(), "http://127.0.0.1:8000/health");

            std::env::set_var("RAPID_MLX_HOST", "http://localhost:8000/v1");
            std::env::set_var("RAPID_MLX_API_KEY", "secret");
            std::env::set_var("RAPID_MLX_MODEL", "qwen3.5-4b-4bit");
            let cfg = runtime();
            assert_eq!(cfg.host, "http://localhost:8000");
            assert_eq!(cfg.api_key.as_deref(), Some("secret"));
            assert_eq!(health_url(), "http://localhost:8000/health");

            let chat = chat_from_env().expect("chat client");
            assert_eq!(chat.name(), PROVIDER_ID);
            assert_eq!(chat.model(), "qwen3.5-4b-4bit");
        });
    }

    #[test]
    fn explicit_model_wins_and_generic_model_env_is_the_fallback() {
        with_cleared_env(|| {
            std::env::set_var("EDGEQUAKE_LLM_MODEL", "from-generic");
            std::env::set_var("EDGEQUAKE_EMBEDDING_MODEL", "embed-generic");
            let chat = chat_named("workspace-model", None).expect("named");
            assert_eq!(chat.model(), "workspace-model");

            let fallback = chat_from_env().expect("fallback");
            assert_eq!(fallback.model(), "from-generic");

            let embed = embedding_named("").expect("embed");
            assert_eq!(embed.model(), "embed-generic");
        });
    }
}
