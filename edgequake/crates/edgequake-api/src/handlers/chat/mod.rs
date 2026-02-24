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
//!  │       └── Neither? ──► None → use sota_engine's default              │
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
use edgequake_core::types::{
    ConversationMode, MessageContext, MessageContextEntity, MessageContextRelationship,
    MessageSource,
};
use edgequake_query::QueryMode;

// Re-export DTOs from chat_types module
pub use crate::handlers::chat_types::*;

// ============================================================================
// Helper Functions
// ============================================================================

pub mod completion;
pub mod streaming;

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
        .unwrap_or(QueryMode::Hybrid)
}

/// Convert an ISO 639-1 language code to its full English name.
/// Used to build a clear language directive for the LLM prompt.
fn language_code_to_name(code: &str) -> &'static str {
    match code.to_lowercase().as_str() {
        "en" => "English",
        "zh" | "zh-cn" | "zh-tw" | "zh-hans" | "zh-hant" => "Chinese",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        "pt" | "pt-br" => "Portuguese",
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

fn build_sources(context: &edgequake_query::QueryContext) -> Vec<SourceReference> {
    let mut sources = Vec::new();
    let mut ref_counter = 1usize;

    for chunk in &context.chunks {
        sources.push(SourceReference {
            source_type: "chunk".to_string(),
            id: chunk.id.clone(),
            score: chunk.score,
            rerank_score: None,
            snippet: Some(chunk.content.chars().take(200).collect()),
            reference_id: Some(ref_counter),
            document_id: chunk.document_id.clone(),
            file_path: None,
            start_line: chunk.start_line,
            end_line: chunk.end_line,
            chunk_index: chunk.chunk_index,
        });
        ref_counter += 1;
    }

    for entity in &context.entities {
        sources.push(SourceReference {
            source_type: "entity".to_string(),
            id: entity.name.clone(),
            score: entity.score,
            rerank_score: None,
            snippet: Some(entity.description.chars().take(200).collect()),
            reference_id: Some(ref_counter),
            // Source tracking for citations (LightRAG parity)
            document_id: entity.source_document_id.clone(),
            file_path: entity.source_file_path.clone(),
            start_line: None,
            end_line: None,
            chunk_index: None,
        });
        ref_counter += 1;
    }

    for rel in &context.relationships {
        sources.push(SourceReference {
            source_type: "relationship".to_string(),
            id: format!("{}->{}", rel.source, rel.target),
            score: rel.score,
            rerank_score: None,
            snippet: Some(format!(
                "{} {} {}",
                rel.source, rel.relation_type, rel.target
            )),
            reference_id: Some(ref_counter),
            // Source tracking for citations (LightRAG parity)
            document_id: rel.source_document_id.clone(),
            file_path: rel.source_file_path.clone(),
            start_line: None,
            end_line: None,
            chunk_index: None,
        });
        ref_counter += 1;
    }

    sources
}

fn sources_to_message_context(sources: &[SourceReference]) -> MessageContext {
    MessageContext {
        sources: sources
            .iter()
            .filter(|s| s.source_type == "chunk")
            .map(|s| MessageSource {
                id: s.id.clone(),
                title: s.file_path.clone().or_else(|| s.document_id.clone()),
                content: Some(s.snippet.clone().unwrap_or_default()),
                score: s.score,
                document_id: s.document_id.clone(),
            })
            .collect(),
        entities: sources
            .iter()
            .filter(|s| s.source_type == "entity")
            .map(|s| MessageContextEntity {
                name: s.id.clone(),
                entity_type: "UNKNOWN".to_string(), // Not available in SourceReference
                description: s.snippet.clone(),
                score: s.score,
                source_document_id: s.document_id.clone(),
                source_file_path: s.file_path.clone(),
                source_chunk_ids: Vec::new(), // Not available in SourceReference
            })
            .collect(),
        relationships: sources
            .iter()
            .filter(|s| s.source_type == "relationship")
            .map(|s| {
                // Parse the relationship ID which is in "SOURCE->TARGET" format
                let parts: Vec<&str> = s.id.split("->").collect();
                let (source, target) = if parts.len() >= 2 {
                    (parts[0].trim().to_string(), parts[1].trim().to_string())
                } else {
                    (s.id.clone(), "UNKNOWN".to_string())
                };
                // Try to extract relation type from snippet ("SOURCE RELATION_TYPE TARGET")
                let relation_type = s
                    .snippet
                    .as_ref()
                    .map(|snippet| {
                        let words: Vec<&str> = snippet.split_whitespace().collect();
                        if words.len() >= 3 {
                            words[1..words.len() - 1].join("_").to_uppercase()
                        } else {
                            "RELATED_TO".to_string()
                        }
                    })
                    .unwrap_or_else(|| "RELATED_TO".to_string());

                MessageContextRelationship {
                    source,
                    target,
                    relation_type,
                    description: s.snippet.clone(),
                    score: s.score,
                    source_document_id: s.document_id.clone(),
                    source_file_path: s.file_path.clone(),
                }
            })
            .collect(),
    }
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
        let event = ChatStreamEvent::Context { sources: vec![] };
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
        }];

        let context = sources_to_message_context(&sources);
        assert_eq!(context.sources.len(), 1);
        // With no file_path or document_id, title should be None (not "chunk")
        assert_eq!(context.sources[0].title, None);
    }
}
