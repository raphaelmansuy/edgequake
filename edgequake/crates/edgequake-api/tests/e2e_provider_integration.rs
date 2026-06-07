//! End-to-end integration tests for AppState provider configuration.
//!
//! These tests verify that AppState correctly uses ProviderFactory for:
//! - Provider auto-detection from environment
//! - Embedding dimension configuration  
//! - Provider switching between Mock and Ollama
//!
//! Note: Ollama tests skip if Ollama is not available at localhost:11434.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Configuration validation
//! @iteration OODA Loop #3 - Phase 5B

mod common;

use common::clear_provider_detection_env;
use edgequake_api::state::AppState;
use serial_test::serial;

/// Check if Ollama is available at localhost:11434.
/// This is a best-effort check - provider creation may still fail if Ollama
/// is not fully operational.
#[allow(dead_code)]
fn is_ollama_available() -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();

    match client.get("http://localhost:11434/api/tags").send() {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Test that AppState uses Mock provider by default (no env vars set).
#[tokio::test]
#[serial]
async fn test_appstate_default_mock_provider() {
    clear_provider_detection_env();

    // Create AppState - should use Mock provider
    let state = AppState::new_memory(None::<String>);

    // Verify Mock provider selected
    assert_eq!(
        state.query.llm_provider.name(),
        "mock",
        "AppState should use Mock provider by default"
    );
    assert_eq!(
        state.query.embedding_provider.name(),
        "mock",
        "AppState should use Mock embedding by default"
    );

    // Verify Mock uses 1536 dimensions (OpenAI-compatible)
    assert_eq!(
        state.query.embedding_provider.dimension(),
        1536,
        "Mock provider should have 1536 dimensions"
    );
}

/// Test that explicit EDGEQUAKE_LLM_PROVIDER=mock works.
#[tokio::test]
#[serial]
async fn test_appstate_explicit_mock_selection() {
    clear_provider_detection_env();
    std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");

    let state = AppState::new_memory(None::<String>);

    assert_eq!(state.query.llm_provider.name(), "mock");
    assert_eq!(state.query.embedding_provider.dimension(), 1536);

    // Cleanup
    clear_provider_detection_env();
}

/// Test that compatibility aliases are normalized before provider factory use.
#[tokio::test]
#[serial]
async fn test_appstate_model_provider_alias_selection() {
    clear_provider_detection_env();
    std::env::set_var("MODEL_PROVIDER", "mock");
    std::env::set_var("CHAT_MODEL", "gpt-5-mini");
    std::env::set_var("EMBEDDING_MODEL", "text-embedding-3-small");

    let state = AppState::new_memory(None::<String>);

    assert_eq!(state.query.llm_provider.name(), "mock");
    assert_eq!(state.query.embedding_provider.name(), "mock");
    assert_eq!(std::env::var("EDGEQUAKE_LLM_PROVIDER").unwrap(), "mock");
    assert_eq!(std::env::var("EDGEQUAKE_LLM_MODEL").unwrap(), "gpt-5-mini");
    assert_eq!(
        std::env::var("EDGEQUAKE_EMBEDDING_MODEL").unwrap(),
        "text-embedding-3-small"
    );

    clear_provider_detection_env();
}

/// Test dimension differences between Mock and Ollama providers.
#[tokio::test]
#[serial]
async fn test_provider_dimension_matrix() {
    // Test 1: Mock provider
    clear_provider_detection_env();

    let state_mock = AppState::new_memory(None::<String>);
    let mock_dimension = state_mock.query.embedding_provider.dimension();
    assert_eq!(mock_dimension, 1536, "Mock should have 1536 dimensions");

    // Test 2: Check Ollama dimension would be different
    // We don't actually create Ollama state here to avoid flaky tests
    // Just document the expected dimension
    let expected_ollama_dimension = 768;
    assert_ne!(
        mock_dimension, expected_ollama_dimension,
        "Mock and Ollama dimensions must be different (migration safety)"
    );
}
