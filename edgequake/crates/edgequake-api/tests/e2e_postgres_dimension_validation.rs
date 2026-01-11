//! E2E tests for PostgreSQL dimension validation.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Dimension validation
//! @iteration OODA Loop #4 - Phase 6C

use edgequake_api::state::AppState;
use serial_test::serial;

#[cfg(feature = "postgres")]
mod postgres_tests {
    use super::*;

    /// Helper: Check if PostgreSQL is available
    fn is_postgres_available() -> bool {
        std::env::var("DATABASE_URL").is_ok()
    }

    /// Test that fresh PostgreSQL storage doesn't error (no dimension mismatch).
    #[tokio::test]
    #[serial]
    async fn test_fresh_postgres_no_error() {
        if !is_postgres_available() {
            eprintln!("⚠️  Skipping: DATABASE_URL not set");
            return;
        }

        // Setup: Use Mock provider (1536-dim)
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OPENAI_API_KEY");

        let database_url = std::env::var("DATABASE_URL").unwrap();

        // Create AppState with fresh storage
        let result = AppState::new_postgres(database_url, "").await;

        // Should succeed (no existing vectors to conflict with)
        assert!(result.is_ok(), "Fresh storage should not error");

        // Cleanup: Clear storage for next test
        if result.is_ok() {
            // Delete all vectors using raw SQL (safer than relying on clear())
            if let Ok(pool) = sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect(&std::env::var("DATABASE_URL").unwrap())
                .await
            {
                let _ = sqlx::query("TRUNCATE TABLE eq_default_vectors CASCADE")
                    .execute(&pool)
                    .await;
            }
        }
    }

    /// Test that dimension mismatch is detected and fails.
    #[tokio::test]
    #[serial]
    async fn test_postgres_dimension_mismatch_error() {
        if !is_postgres_available() {
            eprintln!("⚠️  Skipping: DATABASE_URL not set");
            return;
        }

        let database_url = std::env::var("DATABASE_URL").unwrap();

        // Step 1: Create storage with OpenAI dimensions (1536)
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::set_var("OPENAI_API_KEY", "sk-test-key-for-testing");

        let state1 = AppState::new_postgres(database_url.clone(), "sk-test-key")
            .await
            .expect("Failed to create initial state");

        // Store a test vector (1536 dimensions)
        let test_vector = vec![0.1f32; 1536];
        state1
            .vector_storage
            .upsert(&[(
                "test_doc".to_string(),
                test_vector,
                serde_json::json!({"test": true}),
            )])
            .await
            .expect("Failed to store test vector");

        // Verify storage is not empty
        assert!(!state1.vector_storage.is_empty().await.unwrap());

        // Step 2: Try to create AppState with Ollama (768 dimensions)
        std::env::remove_var("OPENAI_API_KEY");
        std::env::set_var("OLLAMA_HOST", "http://localhost:11434");

        let result = AppState::new_postgres(database_url.clone(), "").await;

        // Should fail with dimension mismatch error
        assert!(
            result.is_err(),
            "Should fail when dimension mismatch detected"
        );

        if let Err(err) = result {
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("Dimension mismatch"),
                "Error should mention dimension mismatch, got: {}",
                err_msg
            );
            assert!(
                err_msg.contains("1536"),
                "Error should mention storage dimension (1536), got: {}",
                err_msg
            );
            assert!(
                err_msg.contains("768"),
                "Error should mention provider dimension (768), got: {}",
                err_msg
            );
        }

        // Cleanup: Clear storage
        if let Ok(pool) = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
        {
            let _ = sqlx::query("TRUNCATE TABLE eq_default_vectors CASCADE")
                .execute(&pool)
                .await;
        }
        std::env::remove_var("OLLAMA_HOST");
    }

    /// Test that validation passes when dimensions match.
    #[tokio::test]
    #[serial]
    async fn test_postgres_dimension_match_success() {
        if !is_postgres_available() {
            eprintln!("⚠️  Skipping: DATABASE_URL not set");
            return;
        }

        let database_url = std::env::var("DATABASE_URL").unwrap();

        // Step 1: Create storage with OpenAI (1536-dim)
        std::env::set_var("OPENAI_API_KEY", "sk-test");
        let state1 = AppState::new_postgres(database_url.clone(), "sk-test")
            .await
            .expect("Failed to create state");

        // Store vector
        state1
            .vector_storage
            .upsert(&[("test".to_string(), vec![0.0; 1536], serde_json::json!({}))])
            .await
            .unwrap();

        // Step 2: Create another AppState with same provider
        let result = AppState::new_postgres(database_url.clone(), "sk-test").await;

        // Should succeed (dimensions match: 1536 == 1536)
        assert!(result.is_ok(), "Should succeed when dimensions match");

        // Cleanup
        if let Ok(pool) = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
        {
            let _ = sqlx::query("TRUNCATE TABLE eq_default_vectors CASCADE")
                .execute(&pool)
                .await;
        }
        std::env::remove_var("OPENAI_API_KEY");
    }
}
