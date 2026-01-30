# LLM Agnostic Design: Write Once, Deploy Anywhere

_How EdgeQuake's provider abstraction eliminates vendor lock-in_

---

## The $10,000 OpenAI Bill

My inbox dinged at 9 AM on a Monday. The subject line: "URGENT: OpenAI billing issue."

Our startup had been processing documents through a RAG pipeline for three months. Everything worked great. Then OpenAI raised prices on the model we were using. Our monthly bill jumped from $2,400 to $10,800 overnight.

The CTO called an emergency meeting. "Can we switch to a cheaper model?"

The engineering lead sighed. "The OpenAI SDK is baked into everything. It'll take two weeks to refactor."

Two weeks. For changing a model.

That's when we learned the hard lesson about **provider lock-in**.

---

## The Provider Lock-in Problem

Here's what most AI applications look like:

```
┌─────────────────────────────────────────────────────────────────┐
│                    TIGHTLY COUPLED CODE                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│   from openai import OpenAI                                      │
│   client = OpenAI(api_key="sk-...")                              │
│                                                                   │
│   # OpenAI-specific patterns everywhere                          │
│   response = client.chat.completions.create(                     │
│       model="gpt-4o",                                            │
│       messages=[{"role": "user", "content": prompt}]             │
│   )                                                               │
│   text = response.choices[0].message.content                     │
│                                                                   │
│   # Embedded in 50+ files across the codebase                    │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

This creates several problems:

### Problem 1: Cost Optimization is a Refactor

When OpenAI releases a cheaper model (gpt-4o-mini vs gpt-4o: 26x cheaper), you should be able to switch instantly. With coupled code, it's a project.

### Problem 2: Enterprise Requirements

Your enterprise customer requires Azure OpenAI for compliance. That's a different SDK, different authentication, different response format.

### Problem 3: Privacy Concerns

A healthcare client needs on-premise deployment. No data can leave their network. OpenAI is not an option.

### Problem 4: Testing Costs Money

Every CI run makes real API calls. At $0.15 per document, 100 test runs/day = $450/month just for testing.

### Problem 5: Vendor Risk

What if OpenAI has an outage? What if they deprecate your model? What if a competitor offers 10x better pricing?

---

## The Solution: Trait-Based Abstraction

EdgeQuake solves this with Rust's trait system:

```
┌─────────────────────────────────────────────────────────────────┐
│                    EDGEQUAKE ABSTRACTION                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│   Your Application Code                                          │
│          │                                                       │
│          ▼                                                       │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │              LLMProvider Trait                           │   │
│   │   fn complete(&self, prompt: &str) -> LLMResponse       │   │
│   └─────────────────────────────────────────────────────────┘   │
│          │                                                       │
│   ┌──────┼──────┬──────────┬──────────┬──────────┐             │
│   ▼      ▼      ▼          ▼          ▼          ▼             │
│ OpenAI  Ollama  Azure    LMStudio   Gemini    Mock             │
│ (cloud) (local) (ent)    (local)    (cloud)   (test)           │
│                                                                   │
│   Switch with ONE environment variable                           │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### The LLMProvider Trait

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the provider name (e.g., "openai", "ollama")
    fn name(&self) -> &str;

    /// Get the current model (e.g., "gpt-4o-mini")
    fn model(&self) -> &str;

    /// Get maximum context length
    fn max_context_length(&self) -> usize;

    /// Generate a completion
    async fn complete(&self, prompt: &str) -> Result<LLMResponse>;

    /// Generate a completion with options
    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<LLMResponse>;
}
```

Any provider that implements this trait works with EdgeQuake. No code changes required.

### The EmbeddingProvider Trait

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Get embedding dimensions
    fn dimensions(&self) -> usize;

    /// Generate embeddings for texts
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}
```

---

## Provider Factory: Auto-Detection

EdgeQuake automatically selects the right provider based on your environment:

```rust
// This is all you need in your code
let (llm, embedding) = ProviderFactory::from_env()?;

// Use the providers - works with any implementation
let response = llm.complete("Extract entities from this text...").await?;
let vectors = embedding.embed(&["Hello world"]).await?;
```

### Auto-Detection Priority

The factory checks your environment in this order:

```
┌─────────────────────────────────────────────────────────────────┐
│                    AUTO-DETECTION FLOW                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│   1. Check EDGEQUAKE_LLM_PROVIDER (explicit override)           │
│         ↓ not set                                                │
│   2. Check OLLAMA_HOST or OLLAMA_MODEL → Use Ollama             │
│         ↓ not set                                                │
│   3. Check LMSTUDIO_HOST → Use LM Studio                        │
│         ↓ not set                                                │
│   4. Check OPENAI_API_KEY → Use OpenAI                          │
│         ↓ not set                                                │
│   5. Fallback → Use Mock provider                               │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

This means:

- **Development**: Set `OLLAMA_HOST` and use local models for free
- **Production**: Set `OPENAI_API_KEY` and use cloud models
- **Testing**: Don't set anything — Mock provider, no API calls

---

## Supported Providers

### OpenAI (Cloud)

```bash
export OPENAI_API_KEY="sk-..."
export OPENAI_MODEL="gpt-4o-mini"  # Optional, defaults to gpt-4o-mini
```

Models: gpt-4o, gpt-4o-mini, gpt-4-turbo
Best for: Production with managed infrastructure

### Ollama (Local)

```bash
export OLLAMA_HOST="http://localhost:11434"
export OLLAMA_MODEL="llama3.2"
```

Models: llama3.2, qwen2.5, mistral, gemma2
Best for: Development, privacy-sensitive applications

### LM Studio (Local)

```bash
export LMSTUDIO_HOST="http://localhost:1234"
export LMSTUDIO_MODEL="local-model"
```

Best for: Desktop development with UI model management

### Azure OpenAI (Enterprise)

```bash
export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com/"
export AZURE_OPENAI_API_KEY="..."
export AZURE_OPENAI_DEPLOYMENT="gpt-4o"
```

Best for: Enterprise compliance, regional data residency

### Mock (Testing)

```bash
export EDGEQUAKE_LLM_PROVIDER="mock"
```

Returns deterministic responses. Perfect for CI/CD.

---

## Cost Optimization Strategies

Here's the cost matrix for different scenarios:

| Scenario             | Provider | Model       | Cost/1M tokens |
| -------------------- | -------- | ----------- | -------------- |
| Development          | Ollama   | llama3.2    | **$0**         |
| CI/Testing           | Mock     | N/A         | **$0**         |
| Production (budget)  | OpenAI   | gpt-4o-mini | **$0.75**      |
| Production (quality) | OpenAI   | gpt-4o      | **$20**        |
| Enterprise           | Azure    | gpt-4o      | **~$20**       |

### Recommended Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│                    COST OPTIMIZATION FLOW                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│   LOCAL DEV          STAGING             PRODUCTION              │
│   ┌───────┐         ┌───────┐           ┌───────┐               │
│   │Ollama │   →     │ Mock  │     →     │OpenAI │               │
│   │ $0    │         │ $0    │           │$0.75/M│               │
│   └───────┘         └───────┘           └───────┘               │
│                                                                   │
│   Same code. Different environment variables.                    │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

A typical document costs:

- Ollama (local): $0.00
- gpt-4o-mini: $0.0014
- gpt-4o: $0.04

At 1,000 documents/day:

- Ollama: $0/month
- gpt-4o-mini: $42/month
- gpt-4o: $1,200/month

---

## Implementation Example

### Environment Files

```bash
# .env.development
OLLAMA_HOST=http://localhost:11434
OLLAMA_MODEL=llama3.2

# .env.staging
EDGEQUAKE_LLM_PROVIDER=mock

# .env.production
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o-mini
```

### Docker Compose

```yaml
services:
  edgequake:
    image: edgequake/edgequake
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      # Or for local: OLLAMA_HOST=http://ollama:11434

  ollama:
    image: ollama/ollama
    ports:
      - "11434:11434"
```

### Kubernetes

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: llm-provider
data:
  OPENAI_API_KEY: c2stLi4u # base64 encoded
---
apiVersion: apps/v1
kind: Deployment
spec:
  template:
    spec:
      containers:
        - name: edgequake
          envFrom:
            - secretRef:
                name: llm-provider
```

---

## Why Traits Matter

Rust's trait system provides compile-time guarantees that dynamic languages can't:

```rust
// This function works with ANY provider
async fn process_document<P: LLMProvider>(
    provider: &P,
    document: &str,
) -> Result<Vec<Entity>> {
    // Compiler guarantees provider has complete()
    let response = provider.complete(document).await?;
    parse_entities(&response.content)
}
```

If a provider doesn't implement the required methods, **it won't compile**. No runtime surprises.

---

## Getting Started

```bash
# Clone EdgeQuake
git clone https://github.com/raphaelmansuy/edgequake
cd edgequake

# Option 1: Local with Ollama (free)
ollama pull llama3.2
export OLLAMA_HOST="http://localhost:11434"
make dev

# Option 2: Cloud with OpenAI
export OPENAI_API_KEY="sk-..."
make dev
```

Same code. Same features. Your choice of provider.

---

## Key Takeaways

1. **Provider lock-in is expensive** — Changing providers should be an environment variable, not a refactor
2. **Traits provide abstraction** — `LLMProvider` and `EmbeddingProvider` work with any implementation
3. **Auto-detection simplifies config** — Set the right env vars, EdgeQuake figures out the rest
4. **Cost optimization is built-in** — Dev with Ollama ($0), test with Mock ($0), deploy with OpenAI

The LLM landscape changes monthly. Your code shouldn't have to.

---

_EdgeQuake is an open-source Graph-RAG framework implementing the LightRAG algorithm (arXiv:2410.05779) in Rust. Star us on GitHub: [raphaelmansuy/edgequake](https://github.com/raphaelmansuy/edgequake)_

_Thanks to the Ollama team for making local LLMs accessible, and to the Rust community for a trait system that makes this abstraction possible._
