# Reddit Posts for Article 006

## r/rust Post

**Title:** Trait-based LLM provider abstraction in Rust – how we made provider switching a config change

**Body:**

Hey rustaceans!

Working on EdgeQuake (Graph-RAG in Rust), and wanted to share a pattern we've found useful: **trait-based LLM provider abstraction**.

**The problem:**

Most AI apps are tightly coupled to OpenAI's SDK. Switching providers (for cost, privacy, or enterprise requirements) means touching code everywhere.

**The solution:**

Two core traits:

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn max_context_length(&self) -> usize;

    async fn complete(&self, prompt: &str) -> Result<LLMResponse>;

    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<LLMResponse>;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn dimensions(&self) -> usize;

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}
```

**Factory with auto-detection:**

```rust
impl ProviderFactory {
    pub fn from_env() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        // 1. Check EDGEQUAKE_LLM_PROVIDER override
        // 2. Check OLLAMA_HOST → Ollama
        // 3. Check LMSTUDIO_HOST → LM Studio
        // 4. Check OPENAI_API_KEY → OpenAI
        // 5. Fallback → Mock
    }
}
```

**Usage:**

```rust
// Application code doesn't care which provider
let (llm, embedding) = ProviderFactory::from_env()?;

let response = llm.complete("Extract entities from this text").await?;
let vectors = embedding.embed(&["Hello", "World"]).await?;
```

**Why I like this pattern:**

1. **Compile-time safety** — Missing trait methods fail at compile time
2. **Easy testing** — Mock provider for unit tests
3. **Zero runtime cost** — Trait objects with vtable, no dynamic dispatch overhead
4. **Clean boundaries** — Each provider encapsulates its quirks

**Implemented providers:**

- OpenAI (gpt-4o, gpt-4o-mini)
- Ollama (llama3.2, qwen2.5)
- LM Studio (OpenAI-compatible)
- Azure OpenAI
- Mock (testing)

**Question for r/rust:**

We're using `async_trait` for the async trait methods. With native async traits stabilizing, should we migrate? Any gotchas?

Code: https://github.com/raphaelmansuy/edgequake

---

## r/LocalLLaMA Post

**Title:** EdgeQuake: Graph-RAG that works with Ollama, LM Studio, and cloud providers

**Body:**

Hey r/LocalLLaMA!

I wanted to share EdgeQuake, a Graph-RAG framework that's designed to work with local LLMs out of the box.

**Why this matters for local LLM users:**

Most RAG tools default to OpenAI. Running locally is often an afterthought. We flipped the priority:

```
Auto-detection order:
1. Ollama (if OLLAMA_HOST or OLLAMA_MODEL set)
2. LM Studio (if LMSTUDIO_HOST set)
3. OpenAI (if API key present)
4. Mock (fallback)
```

**Local is the default. Cloud is opt-in.**

**Tested models:**

| Model       | Ollama | Entity Extraction |
| ----------- | ------ | ----------------- |
| llama3.2:8b | ✓      | Good              |
| qwen2.5:7b  | ✓      | Good              |
| mistral:7b  | ✓      | Good              |
| gemma2:9b   | ✓      | Excellent         |
| phi3:mini   | ✓      | Fair              |

**Getting started:**

```bash
# Pull a model
ollama pull llama3.2

# Set environment
export OLLAMA_HOST="http://localhost:11434"
export OLLAMA_MODEL="llama3.2"

# Run EdgeQuake
git clone https://github.com/raphaelmansuy/edgequake
cd edgequake
make dev
```

**LM Studio support:**

```bash
export LMSTUDIO_HOST="http://localhost:1234"
export LMSTUDIO_MODEL="your-model"
```

LM Studio uses OpenAI-compatible API, so most models work.

**Cost comparison:**

| Setup              | Monthly Cost (1k docs/day) |
| ------------------ | -------------------------- |
| Ollama on M2 Mac   | $0 + electricity           |
| Ollama on RTX 4090 | $0 + electricity           |
| gpt-4o-mini        | $42                        |
| gpt-4o             | $1,200                     |

**What we're looking for:**

- Model recommendations for entity extraction
- Memory optimization tips for 8GB VRAM
- Interest in quantized model support

Code: https://github.com/raphaelmansuy/edgequake

---

## r/MachineLearning Post

**Title:** [P] LLM-agnostic RAG framework with trait-based provider abstraction

**Body:**

**TL;DR:** Built a Graph-RAG framework where switching LLM providers is an environment variable, not a code change. Supports OpenAI, Ollama, LM Studio, Azure, and Mock providers.

**Motivation:**

We kept seeing RAG applications tightly coupled to OpenAI. When requirements changed (cost optimization, local deployment, enterprise compliance), teams faced significant refactors.

**Solution:**

Rust traits abstract the provider interface:

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<LLMResponse>;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}
```

Factory auto-detects from environment:

```rust
let (llm, embedding) = ProviderFactory::from_env()?;
// Works with any provider
```

**Supported providers:**

| Provider  | Use Case             | Cost        |
| --------- | -------------------- | ----------- |
| OpenAI    | Production (cloud)   | $0.75-20/1M |
| Ollama    | Development, privacy | $0          |
| LM Studio | Desktop development  | $0          |
| Azure     | Enterprise           | ~$20/1M     |
| Mock      | Testing, CI/CD       | $0          |

**Benefits:**

1. **Cost optimization** — Dev with Ollama ($0), prod with gpt-4o-mini
2. **Testing** — Mock provider, no API costs in CI
3. **Enterprise** — Azure support for compliance
4. **Privacy** — Local models for sensitive data
5. **Future-proof** — Add new providers without code changes

**Trade-offs:**

- More initial abstraction work
- Each provider needs trait implementation
- Some provider-specific features don't map cleanly

**Code:** https://github.com/raphaelmansuy/edgequake

Paper reference: LightRAG (arXiv:2410.05779)
