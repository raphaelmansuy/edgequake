//! Unified chat completions handler.
//!
//! This module provides a unified endpoint for chat interactions that handles
//! conversation creation, message persistence, and LLM streaming in a single
//! atomic operation. This is the preferred API for client applications.
//!
//! # WHY: Query Provider Resolution vs Pipeline Provider Resolution
//!
//! The chat handler resolves providers PER-REQUEST. This is SEPARATE from the
//! pipeline's document-extraction providers (see processor.rs). Users often see
//! Ollama logs interleaved with their OpenAI chat query logs and assume their
//! query used Ollama. In reality, Ollama logs come from background pipeline
//! tasks running concurrently.
//!
//! ```text
//!  ┌──────────────────────────────────────────────────────────────────────┐
//!  │  QUERY PROVIDER RESOLUTION (this module)                            │
//!  │                                                                      │
//!  │  UI sends: { provider: "openai", model: "gpt-5-nano" }             │
//!  │       │                                                              │
//!  │       ▼                                                              │
//!  │  WorkspaceProviderResolver::resolve_llm_provider_with_workspace      │
//!  │       │                                                              │
//!  │       ├── Has request.provider + request.model?                      │
//!  │       │   └── YES ──► create_safe_llm_provider() → source=Request   │
//!  │       │                                                              │
//!  │       ├── Has workspace.llm_provider?                                │
//!  │       │   └── YES ──► create_safe_llm_provider() → source=Workspace │
//!  │       │                                                              │
//!  │       └── Neither? ──► None → use engine_impl's default              │
//!  │                                                                      │
//!  │  Result: llm_override = Arc<dyn LLMProvider>                        │
//!  │  Used for: answer generation + keyword extraction (query-time only)  │
//!  └──────────────────────────────────────────────────────────────────────┘
//!
//!  ┌──────────────────────────────────────────────────────────────────────┐
//!  │  PIPELINE PROVIDER (processor.rs - background task, NOT this module) │
//!  │                                                                      │
//!  │  Worker picks up document task with workspace_id                     │
//!  │       │                                                              │
//!  │       ▼                                                              │
//!  │  get_workspace_pipeline_strict(workspace_id)                        │
//!  │       │                                                              │
//!  │       ├── Creates llm + embedding from workspace DB config           │
//!  │       │   └── SUCCESS ──► workspace-specific Pipeline               │
//!  │       │                                                              │
//!  │       └── FAILURE ──► Task fails (strict mode) or falls back to     │
//!  │                       server default pipeline (Ollama from env)      │
//!  │                                                                      │
//!  │  Result: Pipeline with LLMExtractor + EmbeddingProvider             │
//!  │  Used for: entity extraction from documents (background ingestion)   │
//!  └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Implements
//!
//! - **FEAT0501**: Unified chat endpoint with streaming SSE responses
//! - **FEAT0502**: Server-initiated message persistence
//! - **FEAT0503**: Automatic conversation creation and management
//! - **FEAT0504**: Multi-mode query support (local/global/hybrid/naive)
//!
//! ## Use Cases
//!
//! - **UC2101**: User sends a chat message and receives streamed response
//! - **UC2102**: System creates conversation automatically on first message
//! - **UC2103**: User views source citations in chat response
//! - **UC2104**: System persists assistant response after streaming completes
//!
//! ## Enforces
//!
//! - **BR0501**: All messages must be persisted with proper roles
//! - **BR0502**: Streaming must accumulate tokens before persistence
//! - **BR0503**: Source tracking must include document IDs for citations
//! - **BR0504**: Query mode defaults to hybrid when not specified
//!
//! Key benefits:
//! - Server-initiated persistence (no client-side message saving)
//! - Transactional integrity for message storage
//! - Single API call instead of multiple round-trips
//! - Automatic conversation management

use crate::handlers::query::SourceReference;
use edgequake_core::types::ConversationMode;

#[cfg(test)]
use edgequake_core::types::MessageContext;
use edgequake_query::QueryMode;

// Re-export DTOs from chat_types module
pub use crate::handlers::chat_types::*;

// ============================================================================
// Helper Functions
// ============================================================================

pub mod completion;
pub mod streaming;
pub mod validation;

pub use completion::*;
pub use streaming::*;

fn parse_mode(mode: &Option<String>) -> ConversationMode {
    mode.as_ref()
        .and_then(|m| match m.to_lowercase().as_str() {
            "local" => Some(ConversationMode::Local),
            "global" => Some(ConversationMode::Global),
            "hybrid" => Some(ConversationMode::Hybrid),
            "naive" | "simple" => Some(ConversationMode::Naive),
            _ => None,
        })
        .unwrap_or(ConversationMode::Hybrid)
}

fn parse_query_mode(mode: &Option<String>) -> QueryMode {
    mode.as_ref()
        .and_then(|m| QueryMode::parse(m))
        .unwrap_or(QueryMode::Mix)
}

/// Convert an ISO 639-1 language code to its full English name.
/// Used to build a clear language directive for the LLM prompt.
///
/// WHY: Region-tagged locales like "fr-FR" must be normalized to bare codes
/// ("fr") before matching. Browsers often send "fr-FR" instead of "fr", and
/// without stripping the region the match falls through to "English", causing
/// the LLM to respond in English regardless of the user's language setting.
fn language_code_to_name(code: &str) -> &'static str {
    // Strip region suffix: "fr-FR" → "fr", "zh-TW" → "zh"
    let base = code.split('-').next().unwrap_or(code);
    match base.to_lowercase().as_str() {
        "en" => "English",
        "zh" => "Chinese",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        "pt" => "Portuguese",
        "it" => "Italian",
        "ja" => "Japanese",
        "ko" => "Korean",
        "ru" => "Russian",
        "ar" => "Arabic",
        "hi" => "Hindi",
        "nl" => "Dutch",
        "sv" => "Swedish",
        "pl" => "Polish",
        "tr" => "Turkish",
        "vi" => "Vietnamese",
        "th" => "Thai",
        "uk" => "Ukrainian",
        "cs" => "Czech",
        "ro" => "Romanian",
        _ => "English", // fallback
    }
}

/// Enrich the user query with a response language directive.
///
/// WHY: The system prompt says "respond in the same language as the user query"
/// but that fails when the user's UI is in Chinese yet they type in English.
/// By appending an explicit language directive to the query text (not stored in
/// the message), we ensure the LLM responds in the user's preferred language.
fn enrich_query_with_language(query: &str, language: &Option<String>) -> String {
    match language {
        Some(lang) if !lang.is_empty() => {
            let lang_name = language_code_to_name(lang);
            format!("{query}\n\n[IMPORTANT: You MUST respond in {lang_name}]")
        }
        _ => query.to_string(),
    }
}

pub(crate) fn build_sources(
    context: &edgequake_query::QueryContext,
    granularity: crate::handlers::context_types::ContentGranularity,
) -> Vec<SourceReference> {
    crate::services::build_sources_from_context(context, true, None, false, granularity)
}

#[cfg(test)]
fn sources_to_message_context(sources: &[SourceReference]) -> MessageContext {
    crate::services::message_context_from_subgraph(
        &crate::handlers::context_types::SubgraphBundle::default(),
        sources,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_parse_mode() {
        assert_eq!(
            parse_mode(&Some("local".to_string())),
            ConversationMode::Local
        );
        assert_eq!(
            parse_mode(&Some("GLOBAL".to_string())),
            ConversationMode::Global
        );
        assert_eq!(
            parse_mode(&Some("hybrid".to_string())),
            ConversationMode::Hybrid
        );
        assert_eq!(
            parse_mode(&Some("naive".to_string())),
            ConversationMode::Naive
        );
        assert_eq!(
            parse_mode(&Some("simple".to_string())),
            ConversationMode::Naive
        );
        assert_eq!(parse_mode(&None), ConversationMode::Hybrid);
        assert_eq!(
            parse_mode(&Some("invalid".to_string())),
            ConversationMode::Hybrid
        );
    }

    #[test]
    fn test_chat_stream_event_serialization() {
        let event = ChatStreamEvent::Conversation {
            conversation_id: Uuid::nil(),
            user_message_id: Uuid::nil(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"conversation\""));

        let event = ChatStreamEvent::Token {
            content: "hello".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"token\""));
        assert!(json.contains("\"content\":\"hello\""));

        let event = ChatStreamEvent::Done {
            assistant_message_id: Uuid::nil(),
            tokens_used: 100,
            duration_ms: 500,
            llm_provider: None,
            llm_model: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"done\""));
        assert!(json.contains("\"tokens_used\":100"));
    }

    #[test]
    fn test_chat_completion_request_defaults() {
        let json = r#"{"message": "hello world"}"#;
        let request: Result<ChatCompletionRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.message, "hello world");
        assert!(req.stream); // default_stream() returns true
        assert!(req.conversation_id.is_none());
    }

    #[test]
    fn test_chat_completion_request_with_conversation() {
        let json = r#"{
            "message": "test",
            "conversation_id": "00000000-0000-0000-0000-000000000001",
            "mode": "global",
            "stream": false
        }"#;
        let request: Result<ChatCompletionRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert!(!req.stream);
        assert_eq!(req.mode, Some("global".to_string()));
        assert!(req.conversation_id.is_some());
    }

    #[test]
    fn test_chat_stream_event_context() {
        let event = ChatStreamEvent::Context {
            sources: vec![],
            query_mode: None,
            retrieval_time_ms: None,
            subgraph: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"context\""));
        assert!(json.contains("\"sources\":[]"));
    }

    #[test]
    fn test_chat_stream_event_error() {
        let event = ChatStreamEvent::Error {
            message: "Something went wrong".to_string(),
            code: "INTERNAL_ERROR".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("Something went wrong"));
        assert!(json.contains("INTERNAL_ERROR"));
    }

    #[test]
    fn test_sources_to_message_context_uses_file_path_for_title() {
        let sources = vec![SourceReference {
            source_type: "chunk".to_string(),
            id: "doc-123-chunk-0".to_string(),
            score: 0.95,
            rerank_score: None,
            snippet: Some("Test content".to_string()),
            reference_id: Some(1),
            document_id: Some("doc-123".to_string()),
            file_path: Some("research_paper.pdf".to_string()),
            start_line: None,
            end_line: None,
            chunk_index: Some(0),
            entity_type: None,
            degree: None,
            source_chunk_ids: None,
            page_start: None,
            page_end: None,
        }];

        let context = sources_to_message_context(&sources);
        assert_eq!(context.sources.len(), 1);
        // Title should be the file_path, NOT "chunk"
        assert_eq!(
            context.sources[0].title,
            Some("research_paper.pdf".to_string())
        );
    }

    #[test]
    fn test_sources_to_message_context_fallback_to_document_id() {
        let sources = vec![SourceReference {
            source_type: "chunk".to_string(),
            id: "doc-456-chunk-0".to_string(),
            score: 0.8,
            rerank_score: None,
            snippet: Some("Content".to_string()),
            reference_id: Some(1),
            document_id: Some("doc-456".to_string()),
            file_path: None,
            start_line: None,
            end_line: None,
            chunk_index: Some(0),
            entity_type: None,
            degree: None,
            source_chunk_ids: None,
            page_start: None,
            page_end: None,
        }];

        let context = sources_to_message_context(&sources);
        assert_eq!(context.sources.len(), 1);
        // Should fall back to document_id, NOT "chunk"
        assert_eq!(context.sources[0].title, Some("doc-456".to_string()));
    }

    #[test]
    fn test_sources_to_message_context_no_chunk_title() {
        // Verify the old bug is fixed - source_type should never be used as title
        let sources = vec![SourceReference {
            source_type: "chunk".to_string(),
            id: "doc-789-chunk-0".to_string(),
            score: 0.7,
            rerank_score: None,
            snippet: Some("Some text".to_string()),
            reference_id: Some(1),
            document_id: None,
            file_path: None,
            start_line: None,
            end_line: None,
            chunk_index: Some(0),
            entity_type: None,
            degree: None,
            source_chunk_ids: None,
            page_start: None,
            page_end: None,
        }];

        let context = sources_to_message_context(&sources);
        assert_eq!(context.sources.len(), 1);
        // With no file_path or document_id, title should be None (not "chunk")
        assert_eq!(context.sources[0].title, None);
    }

    // ── Fix #207: language_code_to_name — region-tagged locale normalization ─

    #[test]
    fn test_language_code_to_name_bare_codes() {
        // Core supported languages with bare ISO 639-1 codes
        assert_eq!(language_code_to_name("en"), "English");
        assert_eq!(language_code_to_name("fr"), "French");
        assert_eq!(language_code_to_name("de"), "German");
        assert_eq!(language_code_to_name("es"), "Spanish");
        assert_eq!(language_code_to_name("zh"), "Chinese");
        assert_eq!(language_code_to_name("pt"), "Portuguese");
        assert_eq!(language_code_to_name("ja"), "Japanese");
        assert_eq!(language_code_to_name("ko"), "Korean");
        assert_eq!(language_code_to_name("ar"), "Arabic");
        assert_eq!(language_code_to_name("ru"), "Russian");
    }

    /// WHY: This is the core regression for issue #207. Browsers send "fr-FR",
    /// "zh-TW", "pt-BR" etc. Before the fix, these fell through to the "_" arm
    /// and the LLM was told to respond in English regardless of user preference.
    #[test]
    fn test_language_code_to_name_region_tagged_codes() {
        // Most common region-tagged locales that browsers send
        assert_eq!(
            language_code_to_name("fr-FR"),
            "French",
            "fr-FR must resolve to French"
        );
        assert_eq!(
            language_code_to_name("fr-BE"),
            "French",
            "fr-BE (Belgian French)"
        );
        assert_eq!(
            language_code_to_name("fr-CA"),
            "French",
            "fr-CA (Canadian French)"
        );
        assert_eq!(
            language_code_to_name("de-DE"),
            "German",
            "de-DE must resolve to German"
        );
        assert_eq!(
            language_code_to_name("de-AT"),
            "German",
            "de-AT (Austrian German)"
        );
        assert_eq!(language_code_to_name("es-ES"), "Spanish");
        assert_eq!(
            language_code_to_name("es-MX"),
            "Spanish",
            "es-MX (Mexican Spanish)"
        );
        assert_eq!(
            language_code_to_name("zh-TW"),
            "Chinese",
            "zh-TW must resolve to Chinese"
        );
        assert_eq!(
            language_code_to_name("zh-CN"),
            "Chinese",
            "zh-CN must resolve to Chinese"
        );
        assert_eq!(
            language_code_to_name("pt-BR"),
            "Portuguese",
            "pt-BR must resolve to Portuguese"
        );
        assert_eq!(language_code_to_name("pt-PT"), "Portuguese");
        assert_eq!(language_code_to_name("en-US"), "English");
        assert_eq!(language_code_to_name("en-GB"), "English");
        assert_eq!(language_code_to_name("ja-JP"), "Japanese");
        assert_eq!(language_code_to_name("ko-KR"), "Korean");
        assert_eq!(language_code_to_name("ar-SA"), "Arabic");
        assert_eq!(language_code_to_name("ru-RU"), "Russian");
    }

    #[test]
    fn test_language_code_to_name_case_insensitive() {
        // WHY: i18next may emit mixed-case codes in some browser configurations
        assert_eq!(language_code_to_name("FR"), "French");
        assert_eq!(language_code_to_name("FR-FR"), "French");
        assert_eq!(language_code_to_name("ZH"), "Chinese");
        assert_eq!(language_code_to_name("DE-DE"), "German");
    }

    #[test]
    fn test_language_code_to_name_unknown_falls_back_to_english() {
        // Unknown codes must fall back to English (safe default for LLM prompt)
        assert_eq!(language_code_to_name("xx"), "English");
        assert_eq!(language_code_to_name("xx-XX"), "English");
        assert_eq!(language_code_to_name(""), "English");
    }

    #[test]
    fn test_enrich_query_with_language_region_tagged() {
        // WHY: The enrichment function must produce the correct language directive
        // even when the frontend sends a region-tagged locale like "fr-FR".
        let enriched = enrich_query_with_language("Qu'est-ce que la RAG?", &Some("fr-FR".into()));
        assert!(
            enriched.contains("[IMPORTANT: You MUST respond in French]"),
            "fr-FR must produce French directive, got: {enriched}"
        );
    }

    #[test]
    fn test_enrich_query_with_language_bare_code() {
        let enriched = enrich_query_with_language("Was ist RAG?", &Some("de".into()));
        assert!(enriched.contains("[IMPORTANT: You MUST respond in German]"));
        assert!(enriched.starts_with("Was ist RAG?"));
    }

    #[test]
    fn test_enrich_query_no_language_unchanged() {
        // WHY: When no language is set, query must pass through unmodified
        let q = "What is RAG?";
        assert_eq!(enrich_query_with_language(q, &None), q);
        assert_eq!(enrich_query_with_language(q, &Some("".into())), q);
    }
}
