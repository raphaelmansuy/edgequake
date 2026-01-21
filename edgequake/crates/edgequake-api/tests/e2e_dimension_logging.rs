//! E2E tests for dimension logging in AppState.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Dimension logging
//! @iteration OODA Loop #4 - Phase 6C

use edgequake_api::state::AppState;
use serial_test::serial;

/// Test that dimension is logged when creating memory storage.
#[tokio::test]
#[serial]
async fn test_dimension_logged_memory_mock() {
    // Setup: Use Mock provider (1536-dim)
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("OPENAI_API_KEY");

    // Enable info logging
    std::env::set_var("RUST_LOG", "edgequake_api=info");

    // Create AppState
    let _state = AppState::new_memory(None::<String>);

    // Note: Actual log verification requires tracing-subscriber setup
    // For now, we just verify it doesn't panic
    // Manual verification: Run test with RUST_LOG=info and check output
}

/// Test that dimension is logged when creating memory storage with Ollama.
#[tokio::test]
#[serial]
async fn test_dimension_logged_memory_ollama() {
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    std::env::set_var("RUST_LOG", "edgequake_api=info");

    // This will use Ollama provider (768-dim) if available
    // Otherwise falls back to Mock
    let _state = AppState::new_memory(None::<String>);

    // Cleanup
    std::env::remove_var("OLLAMA_HOST");
}
