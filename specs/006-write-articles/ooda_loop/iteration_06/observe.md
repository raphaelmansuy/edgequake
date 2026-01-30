# OODA Iteration 06 - Observe

## Mission Re-Read ✅

**Mission**: Write 20+ promotional articles for EdgeQuake (Medium, LinkedIn, X, HN, Reddit, Substack)
**Spec File**: `./specs/006-write-articles.md`
**Current Article**: 006_llm_provider_abstraction

---

## 🔭 Territory Mapping

### LLM Provider Architecture (from codebase)

**Source Files Analyzed**:

- `edgequake-llm/src/factory.rs` - Provider factory with auto-detection
- `edgequake-llm/src/traits.rs` - LLMProvider and EmbeddingProvider traits
- `edgequake-llm/src/providers/` - Provider implementations

---

### Supported Providers

From `src/providers/`:

| Provider     | File              | Type      | API Key Required       |
| ------------ | ----------------- | --------- | ---------------------- |
| OpenAI       | `openai.rs`       | Cloud     | Yes (`OPENAI_API_KEY`) |
| Ollama       | `ollama.rs`       | Local     | No                     |
| LM Studio    | `lmstudio.rs`     | Local     | No                     |
| Azure OpenAI | `azure_openai.rs` | Cloud     | Yes                    |
| Gemini       | `gemini.rs`       | Cloud     | Yes                    |
| Jina         | `jina.rs`         | Embedding | Yes                    |
| Mock         | `mock.rs`         | Testing   | No                     |

---

### Provider Factory Pattern

From `factory.rs`:

```rust
/// Auto-detect and create providers from environment.
///
/// # Priority
///
/// 1. `EDGEQUAKE_LLM_PROVIDER` environment variable (explicit selection)
/// 2. Auto-detect: OLLAMA_HOST → LMSTUDIO_HOST → OPENAI_API_KEY → Mock
pub fn from_env() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)>
```

**Auto-Detection Priority**:

1. Check `EDGEQUAKE_LLM_PROVIDER` override
2. Check for `OLLAMA_HOST` or `OLLAMA_MODEL` → Use Ollama
3. Check for `LMSTUDIO_HOST` → Use LM Studio
4. Check for `OPENAI_API_KEY` → Use OpenAI
5. Fallback → Use Mock provider

---

### Provider Traits

From `traits.rs`:

```rust
/// WHY: Trait-Based Provider Abstraction
///
/// Using traits instead of concrete types enables:
/// - **Testing**: MockProvider for unit tests (no API calls)
/// - **Flexibility**: Swap providers without code changes
/// - **Cost control**: Route to different providers based on request type
/// - **Resilience**: Fallback providers when primary is unavailable

#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn max_context_length(&self) -> usize;
    async fn complete(&self, prompt: &str) -> Result<LLMResponse>;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn dimensions(&self) -> usize;
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}
```

---

### Environment Configuration

| Variable                 | Provider  | Purpose                 |
| ------------------------ | --------- | ----------------------- |
| `EDGEQUAKE_LLM_PROVIDER` | All       | Override auto-detection |
| `OPENAI_API_KEY`         | OpenAI    | API authentication      |
| `OPENAI_BASE_URL`        | OpenAI    | Custom endpoint         |
| `OLLAMA_HOST`            | Ollama    | Server URL              |
| `OLLAMA_MODEL`           | Ollama    | Model name              |
| `LMSTUDIO_HOST`          | LM Studio | Server URL              |
| `AZURE_OPENAI_*`         | Azure     | Azure configuration     |

---

### Key Benefits Identified

1. **Zero Code Changes** - Switch providers via environment variables
2. **Cost Control** - Local (Ollama) for dev, cloud (OpenAI) for prod
3. **Testing** - Mock provider eliminates API costs in CI
4. **Resilience** - Fallback chain when providers fail
5. **Vendor Flexibility** - No lock-in to single provider

---

### Usage Pattern

```rust
// Auto-detect from environment
let (llm, embedding) = ProviderFactory::from_env()?;

// Use traits - code works with any provider
let response = llm.complete("Extract entities from...").await?;
let embeddings = embedding.embed(&texts).await?;
```

---

### Cost Comparison

| Provider | Model       | Input (1M tokens) | Output (1M tokens) |
| -------- | ----------- | ----------------- | ------------------ |
| OpenAI   | gpt-4o-mini | $0.15             | $0.60              |
| OpenAI   | gpt-4o      | $5.00             | $15.00             |
| Ollama   | llama3.2    | $0.00 (local)     | $0.00              |
| Ollama   | qwen2.5     | $0.00 (local)     | $0.00              |
| Azure    | gpt-4o      | ~$5.00            | ~$15.00            |
