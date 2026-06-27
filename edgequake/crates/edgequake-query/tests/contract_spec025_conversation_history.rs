//! SPEC-025 5.1 — conversation history wired into query engine.

use edgequake_query::{
    conversation_context::{format_conversation_history, query_with_conversation_context},
    types::{ConversationMessage, QueryRequest},
};

#[test]
fn format_conversation_history_renders_roles() {
    let history = vec![
        ConversationMessage {
            role: "user".into(),
            content: "What is EdgeQuake?".into(),
        },
        ConversationMessage {
            role: "assistant".into(),
            content: "A knowledge graph RAG framework.".into(),
        },
    ];
    let section = format_conversation_history(&history, 6).expect("section");
    assert!(section.contains("User: What is EdgeQuake?"));
    assert!(section.contains("Assistant: A knowledge graph RAG framework."));
}

#[test]
fn keyword_query_context_prepends_history() {
    let history = vec![ConversationMessage {
        role: "user".into(),
        content: "Who built EdgeQuake?".into(),
    }];
    let q = query_with_conversation_context("What about query modes?", &history, 4);
    assert!(q.contains("Who built EdgeQuake?"));
    assert!(q.contains("Current question: What about query modes?"));
}

#[test]
fn query_request_carries_history_field() {
    let req = QueryRequest::new("follow up").with_conversation_history(vec![ConversationMessage {
        role: "user".into(),
        content: "prior".into(),
    }]);
    assert_eq!(req.conversation_history.len(), 1);
}
