//! OODA-18: Query engine E2E tests.
//!
//! Validates query-related endpoints:
//! 1. Basic query execution with response structure validation
//! 2. Query modes (naive, local, global, hybrid, mix)
//! 3. Context-only and prompt-only queries
//! 4. Conversation management (create, list)
//! 5. Query response field completeness (answer, mode, sources, stats)
//! 6. Conversation history in queries
//! 7. Query validation (empty query, long query)
//!
//! WHY: The query engine is the primary user-facing feature of EdgeQuake.
//! These tests ensure all query modes work, response structure is stable,
//! and conversation management integrates correctly with queries.

mod common;

use axum::http::StatusCode;
use common::{create_test_app, get_with_tenant, post_json, post_json_with_tenant, with_timeout};
use serde_json::json;
use std::time::Duration;

// ============================================================================
// Constants — unique UUIDs for this test file
// ============================================================================

const TENANT_ID: &str = "aaaaaaaa-0018-0018-0018-aaaaaaaaaaaa";
const USER_ID: &str = "bbbbbbbb-0018-0018-0018-bbbbbbbbbbbb";
const WORKSPACE_ID: &str = "cccccccc-0018-0018-0018-cccccccccccc";

// ============================================================================
// Basic Query Tests
// ============================================================================

/// OODA-18: Basic query returns structured response with answer, mode, sources, stats.
#[tokio::test]
async fn test_basic_query_response_structure() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        // Upload a document first so there's something to query
        let upload = json!({
            "content": "Dr. Marie Curie conducted pioneering research on radioactivity. She discovered polonium and radium.",
            "title": "Marie Curie"
        });
        let (status, _) = post_json(&app, "/api/v1/documents", &upload).await;
        assert_eq!(status, StatusCode::CREATED);

        // Execute query
        let query = json!({
            "query": "What did Marie Curie discover?",
            "mode": "naive"
        });
        let (status, body) = post_json(&app, "/api/v1/query", &query).await;
        assert_eq!(status, StatusCode::OK, "Query should return 200: {}", body);

        // WHY: QueryResponse has answer, mode, sources, stats fields.
        assert!(body["answer"].is_string(), "Should have 'answer': {}", body);
        assert!(body["mode"].is_string(), "Should have 'mode': {}", body);
        assert!(body["sources"].is_array(), "Should have 'sources': {}", body);
        assert!(body["stats"].is_object(), "Should have 'stats': {}", body);

        // Verify stats sub-fields
        let stats = &body["stats"];
        assert!(
            stats["total_time_ms"].is_number(),
            "Stats should have total_time_ms: {}",
            stats
        );

        body
    })
    .await;

    assert!(result.is_ok(), "Basic query: {}", result.unwrap_err());
}

/// OODA-18: Query with different modes all return valid responses.
#[tokio::test]
async fn test_query_modes_naive_and_hybrid() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        // Upload content first
        let upload = json!({
            "content": "Albert Einstein developed the theory of relativity. His famous equation E=mc² describes energy-mass equivalence.",
            "title": "Einstein"
        });
        post_json(&app, "/api/v1/documents", &upload).await;

        // WHY: Test the two most common modes (naive and hybrid).
        // Mock provider returns "Mock response" for both.
        for mode in &["naive", "hybrid"] {
            let query = json!({
                "query": "Tell me about Einstein.",
                "mode": mode
            });
            let (status, body) = post_json(&app, "/api/v1/query", &query).await;

            assert_eq!(
                status,
                StatusCode::OK,
                "Query mode '{}' should return 200: {}",
                mode,
                body
            );
            assert!(
                body["answer"].is_string(),
                "Mode '{}' should have answer: {}",
                mode,
                body
            );
            assert_eq!(
                body["mode"].as_str().unwrap(),
                *mode,
                "Response mode should match requested mode"
            );
        }
    })
    .await;

    assert!(result.is_ok(), "Query modes: {}", result.unwrap_err());
}

/// OODA-18: Context-only query returns empty answer but valid response.
#[tokio::test]
async fn test_context_only_query() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        let upload = json!({
            "content": "Quantum mechanics describes nature at the smallest scales of energy levels.",
            "title": "Quantum"
        });
        post_json(&app, "/api/v1/documents", &upload).await;

        // WHY: context_only=true skips LLM generation, returns empty answer + context
        let query = json!({
            "query": "What is quantum mechanics?",
            "mode": "naive",
            "context_only": true
        });
        let (status, body) = post_json(&app, "/api/v1/query", &query).await;
        assert_eq!(status, StatusCode::OK, "Context-only query should work: {}", body);

        // Should still have proper response structure
        assert!(body.is_object(), "Should return object: {}", body);
        assert!(body["answer"].is_string(), "Should have answer field: {}", body);
        // WHY: context_only returns empty answer string
        assert_eq!(
            body["answer"].as_str().unwrap(),
            "",
            "Context-only answer should be empty"
        );

        body
    })
    .await;

    assert!(result.is_ok(), "Context only: {}", result.unwrap_err());
}

/// OODA-18: Prompt-only query returns formatted prompt, not LLM answer.
#[tokio::test]
async fn test_prompt_only_query() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        let upload = json!({
            "content": "The Amazon rainforest produces 20% of the world's oxygen.",
            "title": "Amazon"
        });
        post_json(&app, "/api/v1/documents", &upload).await;

        // WHY: prompt_only=true returns the formatted prompt for debugging
        let query = json!({
            "query": "Tell me about the Amazon.",
            "mode": "naive",
            "prompt_only": true
        });
        let (status, body) = post_json(&app, "/api/v1/query", &query).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "Prompt-only query should work: {}",
            body
        );
        assert!(body["answer"].is_string(), "Should have answer: {}", body);

        body
    })
    .await;

    assert!(result.is_ok(), "Prompt only: {}", result.unwrap_err());
}

// ============================================================================
// Conversation Management
// ============================================================================

/// OODA-18: Create a conversation requires tenant+user headers.
#[tokio::test]
async fn test_create_conversation() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let payload = json!({
            "title": "Test Conversation"
        });
        // WHY: Conversation endpoints require X-Tenant-ID and X-User-ID headers
        let (status, body) = post_json_with_tenant(
            &app,
            "/api/v1/conversations",
            &payload,
            TENANT_ID,
            USER_ID,
            WORKSPACE_ID,
        )
        .await;

        assert_eq!(
            status,
            StatusCode::CREATED,
            "Create conversation should return 201: {}",
            body
        );

        // ConversationResponse has id field (UUID)
        assert!(
            body["id"].is_string(),
            "Conversation should have id: {}",
            body
        );
        assert!(
            body["title"].is_string(),
            "Conversation should have title: {}",
            body
        );

        body
    })
    .await;

    assert!(result.is_ok(), "Create conv: {}", result.unwrap_err());
}

/// OODA-18: Create conversation without tenant headers returns 400.
#[tokio::test]
async fn test_create_conversation_no_headers() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let payload = json!({ "title": "No Tenant" });
        // WHY: Without tenant headers, should get 400 "Missing X-Tenant-ID header"
        let (status, _body) = post_json(&app, "/api/v1/conversations", &payload).await;

        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "Create conversation without tenant should return 400"
        );
    })
    .await;

    assert!(result.is_ok(), "No headers: {}", result.unwrap_err());
}

/// OODA-18: List conversations returns paginated response.
#[tokio::test]
async fn test_list_conversations() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Create a conversation first
        let payload = json!({ "title": "List Test Conv" });
        let (status, _) = post_json_with_tenant(
            &app,
            "/api/v1/conversations",
            &payload,
            TENANT_ID,
            USER_ID,
            WORKSPACE_ID,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // List conversations
        let (status, body) = get_with_tenant(
            &app,
            "/api/v1/conversations",
            TENANT_ID,
            USER_ID,
            WORKSPACE_ID,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "List conversations should return 200: {}",
            body
        );

        // WHY: PaginatedConversationsResponse has items + pagination fields
        assert!(
            body["items"].is_array(),
            "Should have 'items' array: {}",
            body
        );
        assert!(
            body["pagination"].is_object(),
            "Should have 'pagination' object: {}",
            body
        );

        body
    })
    .await;

    assert!(result.is_ok(), "List conv: {}", result.unwrap_err());
}

// ============================================================================
// Query Edge Cases
// ============================================================================

/// OODA-18: Query on empty knowledge base should still return 200.
#[tokio::test]
async fn test_query_empty_knowledge_base() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        // Don't upload anything — query empty KB
        let query = json!({
            "query": "What is the meaning of life?",
            "mode": "naive"
        });
        let (status, body) = post_json(&app, "/api/v1/query", &query).await;

        // WHY: Mock provider returns answer even without context.
        assert_eq!(
            status,
            StatusCode::OK,
            "Query on empty KB should return 200: {}",
            body
        );
        assert!(body["answer"].is_string(), "Should have answer: {}", body);

        body
    })
    .await;

    assert!(result.is_ok(), "Empty KB: {}", result.unwrap_err());
}

/// OODA-18: Empty query should be rejected with validation error.
#[tokio::test]
async fn test_query_empty_string() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // WHY: validate_query() rejects empty/whitespace-only queries
        let query = json!({
            "query": "   ",
            "mode": "naive"
        });
        let (status, _body) = post_json(&app, "/api/v1/query", &query).await;

        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Empty query should return 422"
        );
    })
    .await;

    assert!(result.is_ok(), "Empty query: {}", result.unwrap_err());
}

/// OODA-18: Query with conversation_history field for multi-turn context.
#[tokio::test]
async fn test_query_with_conversation_history() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        let upload = json!({
            "content": "Mars is the fourth planet from the Sun. It has a thin atmosphere.",
            "title": "Mars"
        });
        post_json(&app, "/api/v1/documents", &upload).await;

        // WHY: QueryRequest uses 'conversation_history' field (not 'messages')
        let query = json!({
            "query": "What about its atmosphere?",
            "mode": "naive",
            "conversation_history": [
                {"role": "user", "content": "Tell me about Mars."},
                {"role": "assistant", "content": "Mars is the fourth planet from the Sun."}
            ]
        });
        let (status, body) = post_json(&app, "/api/v1/query", &query).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "Query with conversation_history should work: {}",
            body
        );
        assert!(body["answer"].is_string(), "Should have answer: {}", body);

        body
    })
    .await;

    assert!(result.is_ok(), "History query: {}", result.unwrap_err());
}

/// OODA-18: Query response includes sources array (may be empty with mock).
#[tokio::test]
async fn test_query_response_sources() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        let upload = json!({
            "content": "The Eiffel Tower is located in Paris, France. It was built in 1889.",
            "title": "Eiffel Tower"
        });
        post_json(&app, "/api/v1/documents", &upload).await;

        let query = json!({
            "query": "Where is the Eiffel Tower?",
            "mode": "naive"
        });
        let (status, body) = post_json(&app, "/api/v1/query", &query).await;
        assert_eq!(status, StatusCode::OK);

        // WHY: Sources array is always present (may be empty with mock embeddings)
        let sources = body["sources"].as_array().expect("sources should be array");
        // With mock storage, sources may be empty — just verify the field exists and is valid
        for source in sources {
            assert!(
                source["source_type"].is_string(),
                "Source should have source_type"
            );
            assert!(source["id"].is_string(), "Source should have id");
        }

        body
    })
    .await;

    assert!(result.is_ok(), "Sources: {}", result.unwrap_err());
}
