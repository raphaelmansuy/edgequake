# Show HN: EdgeQuake – LLM-agnostic RAG with trait-based provider abstraction

**HackerNews Post**

---

## Title

Show HN: EdgeQuake – LLM-agnostic RAG with trait-based provider abstraction

## URL

https://github.com/raphaelmansuy/edgequake

## Text

Hey HN,

I've been working on EdgeQuake, a Rust Graph-RAG framework. One design decision I'd love feedback on: **trait-based LLM provider abstraction**.

**The problem:**

Most RAG applications are tightly coupled to a specific LLM provider (usually OpenAI). When you want to:

- Switch to a cheaper model
- Use local models for privacy
- Support enterprise Azure requirements
- Run tests without API costs

...you're looking at a significant refactor.

**The solution:**

We use Rust traits to abstract the LLM interface:

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    async fn complete(&self, prompt: &str) -> Result<LLMResponse>;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn dimensions(&self) -> usize;
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}
```

Any provider implementing these traits works with EdgeQuake. The factory auto-detects from environment:

```rust
let (llm, embedding) = ProviderFactory::from_env()?;
```

**Detection priority:**

1. `EDGEQUAKE_LLM_PROVIDER` (explicit override)
2. `OLLAMA_HOST` → Ollama
3. `LMSTUDIO_HOST` → LM Studio
4. `OPENAI_API_KEY` → OpenAI
5. Fallback → Mock

**Providers implemented:**

- OpenAI (gpt-4o, gpt-4o-mini)
- Ollama (llama3.2, qwen2.5, etc.)
- LM Studio (OpenAI-compatible API)
- Azure OpenAI (enterprise)
- Mock (deterministic responses for testing)

**Why traits (vs runtime polymorphism):**

```rust
// Compile-time guarantee that P has complete()
async fn process<P: LLMProvider>(provider: &P, doc: &str) -> Result<()> {
    let response = provider.complete(doc).await?;
    // ...
}
```

If a provider doesn't implement the required methods, it fails at compile time. No runtime surprises.

**The cost benefit:**

| Environment | Provider    | Cost/1M tokens |
| ----------- | ----------- | -------------- |
| Development | Ollama      | $0             |
| CI/Testing  | Mock        | $0             |
| Production  | gpt-4o-mini | $0.75          |
| Quality     | gpt-4o      | $20            |

**Trade-offs:**

- More boilerplate than Python's duck typing
- Each new provider requires a trait implementation
- Async traits require the `async_trait` macro (until Rust stabilizes native async traits)

**Questions for HN:**

1. We prioritize Ollama over OpenAI in auto-detection (local-first). Does this make sense?
2. Would a fallback chain (try Ollama → if down, try OpenAI) be useful?
3. Any providers you'd want to see added?

Code: https://github.com/raphaelmansuy/edgequake

---

## HN Comment Preparation

**Q: Why not just use OpenAI-compatible API mode for everything?**
A: We do support that (LM Studio works this way). But OpenAI's SDK has specific features (JSON mode, function calling) that don't translate perfectly. The trait abstraction lets us handle provider-specific quirks without leaking them to application code.

**Q: What about streaming responses?**
A: The trait has `complete_streaming()` returning `BoxStream<'_, Result<StreamChunk>>`. Streaming is provider-specific, so each implementation handles it appropriately.

**Q: How do you handle different embedding dimensions?**
A: `EmbeddingProvider::dimensions()` returns the expected dimension. Storage layer validates consistency. If you switch providers with different dimensions, you need to re-embed (or use dimension-agnostic similarity measures).

**Q: Is this over-engineered for most use cases?**
A: For a single-provider prototype, yes. For production with cost optimization, testing, and enterprise requirements, it pays off quickly. The abstraction cost is minimal — adding a new provider is ~100 lines of implementation.
