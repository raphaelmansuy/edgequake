//! SPEC-180 test doubles shared by the normal-RAG e2e tests.

#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use edgequake_llm::traits::{
    ChatMessage, CompletionOptions, EmbeddingProvider, LLMProvider, LLMResponse,
};

/// Embedding width the mock workspaces use.
pub const DIM: usize = 1536;

/// A provider that counts chat calls and embeds with a deterministic bag-of-words hash.
///
/// Two texts that share words get a high cosine, so dense retrieval tests have a
/// meaningful signal without a real model. Any chat call is a LAW-180-3 violation for a
/// normal workspace, so [`Self::chat_calls`] must stay zero.
#[derive(Default)]
pub struct CountingProvider {
    chat_calls: AtomicUsize,
    embed_texts: AtomicUsize,
}

impl CountingProvider {
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn chat_calls(&self) -> usize {
        self.chat_calls.load(Ordering::SeqCst)
    }

    pub fn embedded_texts(&self) -> usize {
        self.embed_texts.load(Ordering::SeqCst)
    }

    fn bump(&self) {
        self.chat_calls.fetch_add(1, Ordering::SeqCst);
    }
}

/// Deterministic L2-normalised bag-of-words vector.
pub fn hash_embedding(text: &str) -> Vec<f32> {
    let mut v = vec![0f32; DIM];
    for word in text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 1)
    {
        let h = word.bytes().fold(1469598103934665603u64, |a, b| {
            (a ^ b as u64).wrapping_mul(1099511628211)
        });
        v[(h % DIM as u64) as usize] += 1.0;
    }
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        v[0] = 1.0;
    } else {
        v.iter_mut().for_each(|x| *x /= norm);
    }
    v
}

#[async_trait]
impl LLMProvider for CountingProvider {
    fn name(&self) -> &str {
        "mock"
    }
    fn model(&self) -> &str {
        "mock-llm"
    }
    fn max_context_length(&self) -> usize {
        8192
    }
    async fn complete(&self, _prompt: &str) -> edgequake_llm::Result<LLMResponse> {
        self.bump();
        Ok(LLMResponse::new(
            "{\"entities\":[],\"relationships\":[]}",
            "mock-llm",
        ))
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
        _messages: &[ChatMessage],
        _options: Option<&CompletionOptions>,
    ) -> edgequake_llm::Result<LLMResponse> {
        self.complete("").await
    }
}

#[async_trait]
impl EmbeddingProvider for CountingProvider {
    fn name(&self) -> &str {
        "mock"
    }
    fn model(&self) -> &str {
        "mock-embedding"
    }
    fn dimension(&self) -> usize {
        DIM
    }
    fn max_tokens(&self) -> usize {
        8192
    }
    async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
        self.embed_texts.fetch_add(texts.len(), Ordering::SeqCst);
        Ok(texts.iter().map(|t| hash_embedding(t)).collect())
    }
}

/// Run an async test body on a runtime whose threads have a large stack.
///
/// The ingest futures are deep in debug builds, and the default 2 MiB test-thread stack
/// overflows once an upload, a worker and a delete share one test.
pub fn block_on_big_stack<F>(body: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    const STACK: usize = 64 * 1024 * 1024;
    let runner = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .thread_stack_size(STACK)
                .enable_all()
                .build()
                .expect("test runtime")
                .block_on(body)
        })
        .expect("spawn test thread");
    if let Err(panic) = runner.join() {
        std::panic::resume_unwind(panic);
    }
}
