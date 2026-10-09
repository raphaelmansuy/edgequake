//! SPEC-163: locality is a property of the inference server, not a brand name.

/// Native local inference servers (timeouts, concurrency, admission).
pub fn is_local_provider(provider_name: &str) -> bool {
    matches!(
        provider_name.trim().to_ascii_lowercase().as_str(),
        "ollama"
            | "lmstudio"
            | "lm-studio"
            | "lm_studio"
            | "omlx"
            | "mtplx"
            | "llamacpp"
            | "llama-server"
            | "vllm-mlx"
            | "vllm_mlx"
            | "mlx-lm"
            | "mlx_lm"
            | "openai-compatible"
            | "openai_compatible"
            | "mock"
    )
}

/// Local servers that need the slow extraction / sync-upload profile.
/// Excludes `mock` (in-process, fast).
pub fn is_slow_local_provider(provider_name: &str) -> bool {
    matches!(
        provider_name.trim().to_ascii_lowercase().as_str(),
        "ollama"
            | "lmstudio"
            | "lm-studio"
            | "lm_studio"
            | "omlx"
            | "mtplx"
            | "llamacpp"
            | "llama-server"
            | "vllm-mlx"
            | "vllm_mlx"
            | "mlx-lm"
            | "mlx_lm"
    )
}

/// Default HTTP health path for a local kind.
pub fn local_health_path(provider_name: &str) -> &'static str {
    match provider_name.trim().to_ascii_lowercase().as_str() {
        "ollama" => "/api/version",
        _ => "/v1/models",
    }
}

/// Default loopback base URL for a local kind.
pub fn default_local_base_url(provider_name: &str) -> Option<&'static str> {
    match provider_name.trim().to_ascii_lowercase().as_str() {
        "ollama" => Some("http://127.0.0.1:11434"),
        "lmstudio" | "lm-studio" | "lm_studio" => Some("http://127.0.0.1:1234"),
        "omlx" => Some("http://127.0.0.1:9050"),
        "mtplx" => Some("http://127.0.0.1:9060"),
        "llamacpp" | "llama-server" => Some("http://127.0.0.1:8081"),
        "vllm-mlx" | "vllm_mlx" => Some("http://127.0.0.1:8082"),
        "mlx-lm" | "mlx_lm" => Some("http://127.0.0.1:8083"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omlx_is_slow_local() {
        assert!(is_slow_local_provider("omlx"));
        assert!(is_local_provider("mlx-lm"));
        assert!(!is_slow_local_provider("openai"));
        assert!(!is_slow_local_provider("mock"));
        assert!(is_local_provider("mock"));
    }
}
