//! Environment variable helpers for deterministic model/provider resolution.

/// Compatibility aliases for teams migrating from LightRAG-style naming.
pub const LLM_PROVIDER_ALIASES: &[&str] = &["MODEL_PROVIDER", "CHAT_PROVIDER"];
pub const LLM_MODEL_ALIASES: &[&str] = &["CHAT_MODEL", "LLM_MODEL"];
pub const EMBEDDING_PROVIDER_ALIASES: &[&str] = &["EMBEDDING_PROVIDER"];
pub const EMBEDDING_MODEL_ALIASES: &[&str] = &["EMBEDDING_MODEL"];
pub const EMBEDDING_DIMENSION_ALIASES: &[&str] = &["EMBEDDING_DIMENSION"];

/// Read an environment variable and return `None` when it is absent or empty.
pub fn non_empty_env_var(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

/// Return the first present, non-empty environment variable from `keys`.
pub fn first_non_empty_env_var(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| non_empty_env_var(key))
}

/// Normalize compatibility aliases into EdgeQuake's canonical env vars.
///
/// Canonical variables always win. Aliases are only copied when the canonical
/// variable is unset or empty, which makes the precedence deterministic.
pub fn apply_model_env_aliases() {
    normalize_env_alias("EDGEQUAKE_LLM_PROVIDER", LLM_PROVIDER_ALIASES);
    normalize_env_alias("EDGEQUAKE_LLM_MODEL", LLM_MODEL_ALIASES);
    normalize_env_alias("EDGEQUAKE_EMBEDDING_PROVIDER", EMBEDDING_PROVIDER_ALIASES);
    normalize_env_alias("EDGEQUAKE_EMBEDDING_MODEL", EMBEDDING_MODEL_ALIASES);
    normalize_env_alias("EDGEQUAKE_EMBEDDING_DIMENSION", EMBEDDING_DIMENSION_ALIASES);
}

fn normalize_env_alias(canonical_key: &str, alias_keys: &[&str]) {
    if non_empty_env_var(canonical_key).is_some() {
        return;
    }

    if let Some(alias_value) = first_non_empty_env_var(alias_keys) {
        std::env::set_var(canonical_key, alias_value);
    }
}
