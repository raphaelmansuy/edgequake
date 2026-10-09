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

/// `local` when the URL host is loopback, localhost, or RFC1918/ULA. Otherwise `cloud`.
///
/// Link-local and metadata addresses stay `cloud` so SSRF still denies them.
pub fn locality_for_url(raw: &str) -> &'static str {
    let Ok(url) = url::Url::parse(raw) else {
        return "cloud";
    };
    let Some(host) = url.host_str() else {
        return "cloud";
    };
    if host.eq_ignore_ascii_case("localhost") {
        return "local";
    }
    let trimmed = host.trim_matches(|c| c == '[' || c == ']');
    if let Ok(ip) = trimmed.parse::<std::net::IpAddr>() {
        if crate::ssrf::is_private_or_loopback(ip) && !crate::ssrf::is_metadata_or_link_local(ip) {
            return "local";
        }
    }
    "cloud"
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

    #[test]
    fn locality_follows_the_host() {
        assert_eq!(locality_for_url("http://127.0.0.1:9050"), "local");
        assert_eq!(locality_for_url("http://localhost:11434"), "local");
        assert_eq!(locality_for_url("http://10.1.2.3:8080"), "local");
        assert_eq!(locality_for_url("https://api.openai.com/v1"), "cloud");
        assert_eq!(locality_for_url("http://169.254.169.254/"), "cloud");
    }
}
