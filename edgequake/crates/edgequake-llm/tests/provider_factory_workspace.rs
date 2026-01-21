//! E2E tests for ProviderFactory workspace-specific provider creation.
//!
//! These tests verify that ProviderFactory correctly creates providers
//! based on provider name strings, which is critical for workspace-specific
//! provider selection.
//!
//! @implements SPEC-032: Workspace-specific provider creation verification
//! @implements OODA-220: ProviderFactory provider creation tests

use edgequake_llm::ProviderFactory;
use serial_test::serial;

// ============================================================================
// Helper Functions
// ============================================================================

fn clean_provider_env() {
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("OLLAMA_MODEL");
    std::env::remove_var("LMSTUDIO_HOST");
    std::env::remove_var("LMSTUDIO_MODEL");
    std::env::remove_var("OPENAI_API_KEY");
}

// ============================================================================
// create_embedding_provider Tests
// ============================================================================

/// Test: create_embedding_provider with "ollama" creates Ollama provider.
#[test]
#[serial]
fn test_create_embedding_provider_ollama() {
    clean_provider_env();

    let provider = ProviderFactory::create_embedding_provider("ollama", "nomic-embed-text", 768)
        .expect("Should create Ollama embedding provider");

    assert_eq!(provider.name(), "ollama");
    assert_eq!(provider.dimension(), 768);
}

/// Test: create_embedding_provider with "OLLAMA" (uppercase) creates Ollama provider.
#[test]
#[serial]
fn test_create_embedding_provider_ollama_uppercase() {
    clean_provider_env();

    let provider = ProviderFactory::create_embedding_provider("OLLAMA", "nomic-embed-text", 768)
        .expect("Should create Ollama embedding provider");

    assert_eq!(provider.name(), "ollama");
}

/// Test: create_embedding_provider with "lmstudio" creates LM Studio provider.
#[test]
#[serial]
fn test_create_embedding_provider_lmstudio() {
    clean_provider_env();

    let provider = ProviderFactory::create_embedding_provider(
        "lmstudio",
        "text-embedding-nomic-embed-text-v1.5",
        768,
    )
    .expect("Should create LM Studio embedding provider");

    assert_eq!(provider.name(), "lmstudio");
    assert_eq!(provider.dimension(), 768);
}

/// Test: create_embedding_provider with "mock" creates Mock provider.
#[test]
#[serial]
fn test_create_embedding_provider_mock() {
    clean_provider_env();

    let provider = ProviderFactory::create_embedding_provider("mock", "mock-embedding", 1536)
        .expect("Should create Mock embedding provider");

    assert_eq!(provider.name(), "mock");
}

/// Test: create_embedding_provider with "openai" requires API key.
#[test]
#[serial]
fn test_create_embedding_provider_openai_requires_key() {
    clean_provider_env();

    let result =
        ProviderFactory::create_embedding_provider("openai", "text-embedding-3-small", 1536);

    assert!(result.is_err(), "OpenAI should fail without API key");
    if let Err(err) = result {
        assert!(
            err.to_string().contains("OPENAI_API_KEY"),
            "Error should mention API key"
        );
    }
}

/// Test: create_embedding_provider with invalid provider name fails.
#[test]
#[serial]
fn test_create_embedding_provider_invalid() {
    clean_provider_env();

    let result = ProviderFactory::create_embedding_provider("invalid_provider", "some-model", 768);

    assert!(result.is_err(), "Invalid provider should fail");
    if let Err(err) = result {
        assert!(err.to_string().contains("Unknown embedding provider"));
    }
}

// ============================================================================
// create_llm_provider Tests
// ============================================================================

/// Test: create_llm_provider with "ollama" creates Ollama provider.
#[test]
#[serial]
fn test_create_llm_provider_ollama() {
    clean_provider_env();

    let provider = ProviderFactory::create_llm_provider("ollama", "gemma3:12b")
        .expect("Should create Ollama LLM provider");

    assert_eq!(provider.name(), "ollama");
}

/// Test: create_llm_provider with "lmstudio" creates LM Studio provider.
#[test]
#[serial]
fn test_create_llm_provider_lmstudio() {
    clean_provider_env();

    let provider = ProviderFactory::create_llm_provider("lmstudio", "gemma-3n-e4b-it")
        .expect("Should create LM Studio LLM provider");

    assert_eq!(provider.name(), "lmstudio");
}

/// Test: create_llm_provider with "mock" creates Mock provider.
#[test]
#[serial]
fn test_create_llm_provider_mock() {
    clean_provider_env();

    let provider = ProviderFactory::create_llm_provider("mock", "mock-model")
        .expect("Should create Mock LLM provider");

    assert_eq!(provider.name(), "mock");
}

/// Test: create_llm_provider with "openai" requires API key.
#[test]
#[serial]
fn test_create_llm_provider_openai_requires_key() {
    clean_provider_env();

    let result = ProviderFactory::create_llm_provider("openai", "gpt-4o-mini");

    assert!(result.is_err(), "OpenAI should fail without API key");
    if let Err(err) = result {
        assert!(
            err.to_string().contains("OPENAI_API_KEY"),
            "Error should mention API key"
        );
    }
}

/// Test: create_llm_provider with invalid provider name fails.
#[test]
#[serial]
fn test_create_llm_provider_invalid() {
    clean_provider_env();

    let result = ProviderFactory::create_llm_provider("unknown_llm", "some-model");

    assert!(result.is_err(), "Invalid provider should fail");
    if let Err(err) = result {
        assert!(err.to_string().contains("Unknown LLM provider"));
    }
}

// ============================================================================
// Provider Name Consistency Tests
// ============================================================================

/// Test: Provider names are consistent between embedding and LLM.
#[test]
#[serial]
fn test_provider_name_consistency() {
    clean_provider_env();

    // Ollama
    let ollama_embed = ProviderFactory::create_embedding_provider("ollama", "model", 768).unwrap();
    let ollama_llm = ProviderFactory::create_llm_provider("ollama", "model").unwrap();
    assert_eq!(ollama_embed.name(), ollama_llm.name());

    // LM Studio
    let lms_embed = ProviderFactory::create_embedding_provider("lmstudio", "model", 768).unwrap();
    let lms_llm = ProviderFactory::create_llm_provider("lmstudio", "model").unwrap();
    assert_eq!(lms_embed.name(), lms_llm.name());

    // Mock
    let mock_embed = ProviderFactory::create_embedding_provider("mock", "model", 768).unwrap();
    let mock_llm = ProviderFactory::create_llm_provider("mock", "model").unwrap();
    assert_eq!(mock_embed.name(), mock_llm.name());
}

/// Test: Provider type case insensitivity.
#[test]
#[serial]
fn test_provider_case_insensitivity() {
    clean_provider_env();

    let providers = ["ollama", "OLLAMA", "Ollama", "OlLaMa"];

    for provider_name in providers {
        let result = ProviderFactory::create_embedding_provider(provider_name, "model", 768);
        assert!(
            result.is_ok(),
            "Provider '{}' should be valid",
            provider_name
        );
        assert_eq!(result.unwrap().name(), "ollama");
    }
}

/// Test: Different models can be specified for same provider.
#[test]
#[serial]
fn test_different_models_same_provider() {
    clean_provider_env();

    // Create two Ollama providers with different models
    let provider1 = ProviderFactory::create_embedding_provider("ollama", "nomic-embed-text", 768)
        .expect("Should create provider 1");

    let provider2 = ProviderFactory::create_embedding_provider("ollama", "all-minilm", 384)
        .expect("Should create provider 2");

    // Both should be Ollama providers
    assert_eq!(provider1.name(), "ollama");
    assert_eq!(provider2.name(), "ollama");
}
