//! End-to-end tests for ProviderFactory environment-based selection.
//!
//! These tests verify that ProviderFactory correctly auto-detects and creates
//! providers based on environment variables. Tests must run serially due to
//! shared environment state.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - E2E validation
//! @iteration OODA Loop #3 - Phase 5A

use edgequake_llm::{ProviderFactory, ProviderType};
use serial_test::serial;

/// Test that Ollama is auto-detected when OLLAMA_HOST is set.
#[tokio::test]
#[serial]
async fn test_provider_auto_detection_ollama() {
    // Clean environment to avoid interference
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OPENAI_API_KEY");

    // Set Ollama host (auto-detection should pick this up)
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");

    // Create providers via auto-detection
    let (llm, embedding) =
        ProviderFactory::from_env().expect("Failed to create providers from environment");

    // Verify Ollama was selected
    assert_eq!(
        llm.name(),
        "ollama",
        "Expected Ollama provider, got {}",
        llm.name()
    );
    assert_eq!(
        embedding.name(),
        "ollama",
        "Expected Ollama embedding provider"
    );

    // Verify embeddinggemma dimension (768)
    assert_eq!(
        embedding.dimension(),
        768,
        "embeddinggemma:latest should have 768 dimensions"
    );

    // Cleanup
    std::env::remove_var("OLLAMA_HOST");
}

/// Test that OpenAI is auto-detected when OPENAI_API_KEY is set.
#[tokio::test]
#[serial]
async fn test_provider_auto_detection_openai() {
    // Clean environment
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OLLAMA_HOST");

    // Set OpenAI API key
    std::env::set_var("OPENAI_API_KEY", "sk-test-key-for-testing");

    // Create providers
    let (llm, embedding) = ProviderFactory::from_env().expect("Failed to create providers");

    // Verify OpenAI was selected
    assert_eq!(llm.name(), "openai", "Expected OpenAI provider");
    assert_eq!(embedding.name(), "openai", "Expected OpenAI embedding");

    // Verify text-embedding-3-small dimension (1536)
    assert_eq!(
        embedding.dimension(),
        1536,
        "text-embedding-3-small should have 1536 dimensions"
    );

    // Cleanup
    std::env::remove_var("OPENAI_API_KEY");
}

/// Test that Mock provider is used when no provider env vars are set.
#[tokio::test]
#[serial]
async fn test_provider_auto_detection_mock_fallback() {
    // Clear all provider environment variables
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("OLLAMA_MODEL");
    std::env::remove_var("LMSTUDIO_HOST");
    std::env::remove_var("LMSTUDIO_MODEL");
    std::env::remove_var("OPENAI_API_KEY");

    // Create providers (should fallback to Mock)
    let (llm, embedding) = ProviderFactory::from_env().expect("Failed to create mock providers");

    // Verify Mock was selected
    assert_eq!(llm.name(), "mock", "Expected Mock provider fallback");
    assert_eq!(embedding.name(), "mock", "Expected Mock embedding");

    // Verify Mock dimension (1536, compatible with OpenAI)
    assert_eq!(
        embedding.dimension(),
        1536,
        "Mock provider should have 1536 dimensions (OpenAI-compatible)"
    );
}

/// Test that explicit EDGEQUAKE_LLM_PROVIDER overrides auto-detection.
#[tokio::test]
#[serial]
async fn test_explicit_provider_override() {
    // Set multiple provider env vars (conflicting signals)
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    std::env::set_var("LMSTUDIO_HOST", "http://localhost:1234");
    std::env::set_var("OPENAI_API_KEY", "sk-test");

    // Explicit override should win
    std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");

    let (llm, embedding) =
        ProviderFactory::from_env().expect("Failed to create providers with explicit override");

    // Verify Mock was selected (override worked)
    assert_eq!(
        llm.name(),
        "mock",
        "Expected Mock provider from explicit override"
    );
    assert_eq!(embedding.dimension(), 1536);

    // Cleanup
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("LMSTUDIO_HOST");
    std::env::remove_var("OPENAI_API_KEY");
}

/// Test auto-detection priority chain: explicit > Ollama > LM Studio > OpenAI > Mock.
#[tokio::test]
#[serial]
async fn test_provider_priority_chain() {
    // Test 1: Ollama has priority over LM Studio and OpenAI
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    std::env::set_var("LMSTUDIO_HOST", "http://localhost:1234");
    std::env::set_var("OPENAI_API_KEY", "sk-test");

    let (llm, _) = ProviderFactory::from_env().unwrap();
    assert_eq!(
        llm.name(),
        "ollama",
        "Ollama should have priority over LM Studio and OpenAI"
    );

    // Test 2: LM Studio selected when Ollama not present
    std::env::remove_var("OLLAMA_HOST");
    std::env::set_var("LMSTUDIO_HOST", "http://localhost:1234");
    let (llm, _) = ProviderFactory::from_env().unwrap();
    assert_eq!(
        llm.name(),
        "lmstudio",
        "LM Studio should be selected when Ollama not present"
    );

    // Test 3: OpenAI selected when neither Ollama nor LM Studio present
    std::env::remove_var("LMSTUDIO_HOST");
    let (llm, _) = ProviderFactory::from_env().unwrap();
    assert_eq!(llm.name(), "openai", "OpenAI should be selected");

    // Test 4: Mock fallback when none present
    std::env::remove_var("OPENAI_API_KEY");
    let (llm, _) = ProviderFactory::from_env().unwrap();
    assert_eq!(llm.name(), "mock", "Mock should be fallback");

    // Cleanup
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("LMSTUDIO_HOST");
    std::env::remove_var("OPENAI_API_KEY");
}

/// Test that ProviderType::create() works for all provider types.
#[tokio::test]
async fn test_explicit_provider_creation() {
    // Mock doesn't require env vars
    let (llm, embedding) =
        ProviderFactory::create(ProviderType::Mock).expect("Failed to create Mock provider");
    assert_eq!(llm.name(), "mock");
    assert_eq!(embedding.dimension(), 1536);

    // OpenAI requires API key
    std::env::remove_var("OPENAI_API_KEY");
    let result = ProviderFactory::create(ProviderType::OpenAI);
    assert!(
        result.is_err(),
        "OpenAI creation should fail without API key"
    );

    // Ollama can be created (will use defaults)
    let result = ProviderFactory::create(ProviderType::Ollama);
    assert!(
        result.is_ok(),
        "Ollama creation should succeed with defaults"
    );

    // LM Studio can be created (will use defaults)
    let result = ProviderFactory::create(ProviderType::LMStudio);
    assert!(
        result.is_ok(),
        "LM Studio creation should succeed with defaults"
    );
    let (llm, embedding) = result.unwrap();
    assert_eq!(llm.name(), "lmstudio");
    assert_eq!(embedding.dimension(), 768); // nomic-embed-text-v1.5
}

/// Test embedding dimension detection via ProviderFactory helper.
#[tokio::test]
#[serial]
async fn test_embedding_dimension_detection() {
    // Test Mock dimension
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("LMSTUDIO_HOST");
    std::env::remove_var("OPENAI_API_KEY");

    let dim = ProviderFactory::embedding_dimension().expect("Failed to detect dimension");
    assert_eq!(dim, 1536, "Mock provider dimension");

    // Test Ollama dimension (if available)
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    let dim = ProviderFactory::embedding_dimension().expect("Failed to detect Ollama dimension");
    assert_eq!(dim, 768, "Ollama provider dimension");

    // Test LM Studio dimension
    std::env::remove_var("OLLAMA_HOST");
    std::env::set_var("LMSTUDIO_HOST", "http://localhost:1234");
    let dim = ProviderFactory::embedding_dimension().expect("Failed to detect LM Studio dimension");
    assert_eq!(dim, 768, "LM Studio provider dimension");

    // Cleanup
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("LMSTUDIO_HOST");
}

/// Test LM Studio auto-detection when LMSTUDIO_HOST is set.
#[tokio::test]
#[serial]
async fn test_provider_auto_detection_lmstudio() {
    // Clean environment to avoid interference
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("OLLAMA_MODEL");

    // Set LM Studio host (auto-detection should pick this up)
    std::env::set_var("LMSTUDIO_HOST", "http://localhost:1234");

    // Create providers via auto-detection
    let (llm, embedding) =
        ProviderFactory::from_env().expect("Failed to create providers from environment");

    // Verify LM Studio was selected
    assert_eq!(
        llm.name(),
        "lmstudio",
        "Expected LM Studio provider, got {}",
        llm.name()
    );
    assert_eq!(
        embedding.name(),
        "lmstudio",
        "Expected LM Studio embedding provider"
    );

    // Verify nomic-embed-text-v1.5 dimension (768)
    assert_eq!(
        embedding.dimension(),
        768,
        "nomic-embed-text-v1.5 should have 768 dimensions"
    );

    // Cleanup
    std::env::remove_var("LMSTUDIO_HOST");
}
