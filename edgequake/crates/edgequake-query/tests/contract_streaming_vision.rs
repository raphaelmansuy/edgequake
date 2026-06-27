//! P-G11 contract tests (RC-16 / SPEC-021): streaming vision parity + backpressure.
//!
//! Acceptance (plan-19 §4 P-G11):
//! - A streaming query carrying `images` uses the vision-capable LLM path
//!   (`chat` with image attachments), not the text-only `stream`/`complete`.
//! - A slow consumer does not cause unbounded LLM buffering: the bounded
//!   channel + sequential `send().await` apply natural backpressure.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use edgequake_llm::traits::{
    ChatMessage, CompletionOptions, ImageData, LLMProvider, LLMResponse, ToolDefinition,
};
use edgequake_query::engine::QueryRequest;
use edgequake_query::{QueryEngine, QueryEngineConfig, QueryMode};
use edgequake_storage::traits::{GraphStorage, VectorStorage};
use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};
use futures::StreamExt;

/// Records LLM calls so the contract test can assert which path was taken.
struct RecordingProvider {
    chat_calls: Arc<AtomicUsize>,
    stream_calls: Arc<AtomicUsize>,
    complete_calls: Arc<AtomicUsize>,
    saw_images: Arc<AtomicBool>,
    response: String,
}

impl RecordingProvider {
    fn new(response: &str) -> Self {
        Self {
            chat_calls: Arc::new(AtomicUsize::new(0)),
            stream_calls: Arc::new(AtomicUsize::new(0)),
            complete_calls: Arc::new(AtomicUsize::new(0)),
            saw_images: Arc::new(AtomicBool::new(false)),
            response: response.to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for RecordingProvider {
    fn name(&self) -> &str {
        "recording"
    }
    fn model(&self) -> &str {
        "recording-model"
    }
    fn max_context_length(&self) -> usize {
        8192
    }
    async fn complete(&self, _prompt: &str) -> edgequake_llm::Result<LLMResponse> {
        self.complete_calls.fetch_add(1, Ordering::SeqCst);
        Ok(LLMResponse::new(self.response.clone(), "recording-model"))
    }
    async fn complete_with_options(
        &self,
        prompt: &str,
        _options: &CompletionOptions,
    ) -> edgequake_llm::Result<LLMResponse> {
        self.complete(prompt).await
    }
    async fn chat(
        &self,
        messages: &[ChatMessage],
        _options: Option<&CompletionOptions>,
    ) -> edgequake_llm::Result<LLMResponse> {
        self.chat_calls.fetch_add(1, Ordering::SeqCst);
        // Detect whether any user message carries images (the vision path).
        let has_images = messages.iter().any(|m| {
            m.images
                .as_ref()
                .map(|imgs| !imgs.is_empty())
                .unwrap_or(false)
        });
        if has_images {
            self.saw_images.store(true, Ordering::SeqCst);
        }
        Ok(LLMResponse::new(self.response.clone(), "recording-model"))
    }
    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        _tools: &[ToolDefinition],
        _tool_choice: Option<edgequake_llm::traits::ToolChoice>,
        options: Option<&CompletionOptions>,
    ) -> edgequake_llm::Result<LLMResponse> {
        self.chat(messages, options).await
    }
    async fn stream(
        &self,
        _prompt: &str,
    ) -> edgequake_llm::Result<futures::stream::BoxStream<'static, edgequake_llm::Result<String>>>
    {
        self.stream_calls.fetch_add(1, Ordering::SeqCst);
        let s = self.response.clone();
        Ok(futures::stream::once(async move { Ok(s) }).boxed())
    }
    fn supports_streaming(&self) -> bool {
        true
    }
}

// Also need an embedding provider — reuse MockProvider for embeddings.

fn make_engine(
    vector: Arc<MemoryVectorStorage>,
    graph: Arc<MemoryGraphStorage>,
    llm: Arc<dyn LLMProvider>,
    embed: Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
) -> QueryEngine {
    QueryEngine::with_mock_keywords(
        QueryEngineConfig::default(),
        vector as Arc<dyn VectorStorage>,
        graph as Arc<dyn GraphStorage>,
        embed,
        llm,
    )
}

#[tokio::test]
async fn streaming_query_with_images_uses_vision_chat_path() {
    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("test", dim));
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    // Seed one chunk so context is non-empty (otherwise the streaming path
    // short-circuits to the apology before reaching the LLM).
    use edgequake_storage::traits::VectorStorage;
    vector
        .upsert(&[(
            "chunk-1".to_string(),
            vec![0.1f32; dim],
            serde_json::json!({
                "type": "chunk",
                "content": "GraphRAG uses graphs for retrieval.",
                "document_id": "doc-1",
            }),
        )])
        .await
        .unwrap();

    let mock = Arc::new(edgequake_llm::MockProvider::default());
    let recorder = Arc::new(RecordingProvider::new("VISION_ANSWER"));

    let engine = make_engine(
        vector,
        graph,
        recorder.clone() as Arc<dyn LLMProvider>,
        mock as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    );

    let mut req = QueryRequest::new("Describe this image.");
    req.mode = Some(QueryMode::Local);
    req.images = Some(vec![ImageData::new("iVBORw0KGgo=", "image/png")]);

    let mut stream = engine
        .query_stream(req)
        .await
        .expect("streaming query must start");

    // Drain the stream.
    let mut collected = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(t) => collected.push_str(&t),
            Err(_) => break,
        }
    }

    assert!(
        !collected.is_empty(),
        "vision streaming must produce a non-empty answer"
    );
    assert_eq!(
        recorder.chat_calls.load(Ordering::SeqCst),
        1,
        "vision streaming must use the chat() path exactly once"
    );
    assert_eq!(
        recorder.stream_calls.load(Ordering::SeqCst),
        0,
        "vision streaming must NOT use the text-only stream() path"
    );
    assert!(
        recorder.saw_images.load(Ordering::SeqCst),
        "the chat() call must carry the attached images"
    );
}

#[tokio::test]
async fn streaming_query_without_images_uses_text_stream_path() {
    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("test", dim));
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    use edgequake_storage::traits::VectorStorage;
    vector
        .upsert(&[(
            "chunk-1".to_string(),
            vec![0.1f32; dim],
            serde_json::json!({
                "type": "chunk",
                "content": "GraphRAG uses graphs for retrieval.",
                "document_id": "doc-1",
            }),
        )])
        .await
        .unwrap();

    let mock = Arc::new(edgequake_llm::MockProvider::default());
    let recorder = Arc::new(RecordingProvider::new("TEXT_ANSWER"));

    let engine = make_engine(
        vector,
        graph,
        recorder.clone() as Arc<dyn LLMProvider>,
        mock as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    );

    let mut req = QueryRequest::new("What is GraphRAG?");
    req.mode = Some(QueryMode::Naive);

    let mut stream = engine.query_stream(req).await.unwrap();
    while stream.next().await.is_some() {}

    assert_eq!(
        recorder.stream_calls.load(Ordering::SeqCst),
        1,
        "text-only streaming must use stream()"
    );
    assert_eq!(
        recorder.chat_calls.load(Ordering::SeqCst),
        0,
        "text-only streaming must NOT use chat()"
    );
}
