//! Engine-level regression: chat options + early tokens + completed cache replay.
use super::*;
use async_trait::async_trait;
use edgequake_llm::traits::{
    ChatMessage, CompletionOptions, LLMProvider, LLMResponse, StreamChunk, ToolChoice,
    ToolDefinition,
};
use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};
use tokio::sync::Notify;

struct StreamingProvider {
    calls: AtomicUsize,
    options: Mutex<Option<CompletionOptions>>,
    gate: Arc<Notify>,
    startup_error: bool,
}

#[async_trait]
impl LLMProvider for StreamingProvider {
    fn name(&self) -> &str {
        "mistral"
    }
    fn model(&self) -> &str {
        "mistral-small-latest"
    }
    fn max_context_length(&self) -> usize {
        8192
    }
    async fn complete(&self, _: &str) -> edgequake_llm::Result<LLMResponse> {
        panic!("structured RAG must not use blocking complete")
    }
    async fn complete_with_options(
        &self,
        _: &str,
        _: &CompletionOptions,
    ) -> edgequake_llm::Result<LLMResponse> {
        panic!("structured RAG must not use blocking complete")
    }
    async fn chat(
        &self,
        _: &[ChatMessage],
        _: Option<&CompletionOptions>,
    ) -> edgequake_llm::Result<LLMResponse> {
        assert!(
            self.startup_error,
            "structured RAG must not use blocking chat"
        );
        Ok(LLMResponse::new("fallback answer", self.model()))
    }
    fn supports_tool_streaming(&self) -> bool {
        true
    }
    async fn chat_with_tools_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        choice: Option<ToolChoice>,
        options: Option<&CompletionOptions>,
    ) -> edgequake_llm::Result<
        futures::stream::BoxStream<'static, edgequake_llm::Result<StreamChunk>>,
    > {
        assert!(tools.is_empty());
        assert!(matches!(choice, Some(ToolChoice::Auto(value)) if value == "none"));
        assert!(messages
            .iter()
            .any(|message| message.content.contains("fixture evidence")));
        assert!(messages
            .last()
            .unwrap()
            .content
            .ends_with("Summarize the fixture."));
        assert!(matches!(
            messages[0].role,
            edgequake_llm::traits::ChatRole::System
        ));
        assert!(matches!(
            messages[1].role,
            edgequake_llm::traits::ChatRole::User
        ));
        if edgequake_llm::provider_prompt_cache_enabled() {
            assert!(options.unwrap().prompt_cache_key.is_some());
        }
        *self.options.lock().unwrap() = options.cloned();
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.startup_error {
            return Err(edgequake_llm::LlmError::NotSupported(
                "structured streaming unavailable".into(),
            ));
        }
        let gate = self.gate.clone();
        Ok(
            futures::stream::once(async { Ok(StreamChunk::Content("first ".into())) })
                .chain(futures::stream::once(async move {
                    gate.notified().await;
                    Ok(StreamChunk::Content("second".into()))
                }))
                .chain(futures::stream::once(async {
                    Ok(StreamChunk::Finished {
                        reason: "stop".into(),
                        ttft_ms: None,
                        usage: None,
                    })
                }))
                .boxed(),
        )
    }
}

fn setup(
    startup_error: bool,
) -> (
    QueryEngine,
    Arc<StreamingProvider>,
    crate::types::QueryRequest,
    QueryContext,
) {
    let provider = Arc::new(StreamingProvider {
        calls: AtomicUsize::new(0),
        options: Mutex::new(None),
        gate: Arc::new(Notify::new()),
        startup_error,
    });
    let engine = QueryEngine::with_mock_keywords(
        crate::engine_impl::QueryEngineConfig::default(),
        Arc::new(MemoryVectorStorage::new("stream-test", 1536)),
        Arc::new(MemoryGraphStorage::new("stream-test")),
        Arc::new(edgequake_llm::MockProvider::default()),
        provider.clone(),
    )
    .with_answer_cache();
    let mut request = crate::types::QueryRequest::new("Summarize the fixture.");
    request.reasoning_effort = Some(" low ".into());
    let context = QueryContext {
        chunks: vec![crate::context::RetrievedChunk::new(
            "fixture-chunk",
            "fixture evidence",
            1.0,
        )],
        ..Default::default()
    };
    (engine, provider, request, context)
}

#[tokio::test]
async fn rag_stream_preserves_options_delivers_early_and_replays_completed_cache() {
    let (engine, provider, request, context) = setup(false);
    let (_, _, mut stream) = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        engine.stream_answer_from_context(&request, context.clone(), QueryMode::Naive, None),
    )
    .await
    .expect("must start before provider generation finishes")
    .unwrap();
    assert_eq!(stream.next().await.unwrap().unwrap(), "first ");
    assert_eq!(
        provider
            .options
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .reasoning_effort
            .as_deref(),
        Some("low")
    );
    provider.gate.notify_one();
    assert_eq!(stream.next().await.unwrap().unwrap(), "second");
    assert!(stream.next().await.is_none());
    let (_, _, mut cached) = engine
        .stream_answer_from_context(&request, context, QueryMode::Naive, None)
        .await
        .unwrap();
    assert_eq!(cached.next().await.unwrap().unwrap(), "first second");
    assert!(cached.next().await.is_none());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn structured_stream_startup_failure_keeps_chat_fallback() {
    let (engine, provider, request, context) = setup(true);
    let (_, _, mut stream) = engine
        .stream_answer_from_context(&request, context, QueryMode::Naive, None)
        .await
        .unwrap();
    assert_eq!(stream.next().await.unwrap().unwrap(), "fallback answer");
    assert!(stream.next().await.is_none());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}
