# LLM Agnostic Design: Write Once, Deploy Anywhere

**X.com Thread** (15 tweets)

---

**1/15**
Your RAG system is probably locked to one LLM provider.

When OpenAI raised prices, one startup needed 2 weeks to switch models.

Here's how EdgeQuake makes provider switching an environment variable:

🧵

---

**2/15**
The lock-in problem:

```
OpenAI SDK in 50+ files
├── openai.chat.completions.create()
├── response.choices[0].message
└── Hardcoded everywhere
```

Cost spike? Refactor.
Enterprise needs Azure? Rewrite.
Privacy requires local? Impossible.

---

**3/15**
The solution: Trait-based abstraction

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    async fn complete(&self, prompt: &str)
        -> Result<LLMResponse>;
}
```

Any provider implementing this works.

---

**4/15**
The architecture:

```
   Your Code
       │
       ▼
 LLMProvider Trait
       │
   ┌───┴───┬───────┬───────┐
   ▼       ▼       ▼       ▼
OpenAI  Ollama  Azure   Mock
```

Switch with one environment variable.
Zero code changes.

---

**5/15**
EdgeQuake's provider factory:

```rust
// This is ALL your code needs
let (llm, embedding) =
    ProviderFactory::from_env()?;

// Works with ANY provider
let response =
    llm.complete("Extract entities...").await?;
```

---

**6/15**
Auto-detection priority:

1. Check EDGEQUAKE_LLM_PROVIDER (override)
2. Check OLLAMA_HOST → Use Ollama
3. Check LMSTUDIO_HOST → Use LM Studio
4. Check OPENAI_API_KEY → Use OpenAI
5. Fallback → Mock provider

Set the right env vars. We figure out the rest.

---

**7/15**
Supported providers:

• OpenAI (gpt-4o, gpt-4o-mini)
• Ollama (llama3.2, qwen2.5)
• LM Studio (any local model)
• Azure OpenAI (enterprise)
• Gemini (experimental)
• Mock (CI/testing)

---

**8/15**
Development (free):

```bash
ollama pull llama3.2
export OLLAMA_HOST="http://localhost:11434"
make dev
```

No API costs.
No rate limits.
Runs on your laptop.

---

**9/15**
Testing (free):

```bash
export EDGEQUAKE_LLM_PROVIDER="mock"
cargo test
```

Deterministic responses.
No API calls.
CI costs: $0.

---

**10/15**
Production (optimized):

```bash
export OPENAI_API_KEY="sk-..."
export OPENAI_MODEL="gpt-4o-mini"
```

gpt-4o-mini: $0.75/1M tokens
gpt-4o: $20/1M tokens

26x cost difference. Same code.

---

**11/15**
Cost per 1,000 documents/day:

| Provider    | Monthly Cost |
| ----------- | ------------ |
| Ollama      | $0           |
| Mock        | $0           |
| gpt-4o-mini | $42          |
| gpt-4o      | $1,200       |

Pick based on your needs.

---

**12/15**
Enterprise deployment:

```bash
export AZURE_OPENAI_ENDPOINT="..."
export AZURE_OPENAI_API_KEY="..."
export AZURE_OPENAI_DEPLOYMENT="gpt-4o"
```

Same code.
Compliance-ready.
Regional data residency.

---

**13/15**
Why Rust traits matter:

```rust
async fn process<P: LLMProvider>(
    provider: &P,
    doc: &str,
) -> Result<Vec<Entity>> {
    // Compiler guarantees this works
    let response = provider.complete(doc).await?;
    parse_entities(&response.content)
}
```

Compile-time safety. No runtime surprises.

---

**14/15**
The recommended flow:

```
LOCAL DEV     STAGING      PRODUCTION
 Ollama   →    Mock    →    OpenAI
   $0           $0         $0.75/M

Same code throughout.
```

---

**15/15**
TL;DR:

EdgeQuake's provider abstraction:
• 6+ supported providers
• Auto-detection from environment
• Zero code changes to switch
• $0 for dev and testing
• Compile-time type safety

The LLM landscape changes monthly.
Your code shouldn't.

github.com/raphaelmansuy/edgequake

/thread
