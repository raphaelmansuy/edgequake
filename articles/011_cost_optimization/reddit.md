# How We Track LLM Costs in Our Production RAG System

**Subreddit**: r/MachineLearning, r/LocalLLaMA, r/rust

---

## Post Title

How we track LLM costs in our production RAG system (open source implementation)

---

## Post Body

Hey everyone,

I've been working on a Graph-RAG system and wanted to share the cost tracking approach we built. This isn't a product pitch—I'm genuinely interested in how others handle this problem.

### The Challenge

When we started deploying RAG at scale, we had no visibility into costs. We'd process a batch of documents and then wait for the API bill to see what it cost. Not great for budgeting or optimization.

### Our Approach

We built cost tracking directly into the pipeline. Here's the core structure:

```rust
pub struct ModelPricing {
    pub model: String,
    pub input_cost_per_1k: f64,  // USD
    pub output_cost_per_1k: f64, // USD
}

pub struct OperationCost {
    pub operation: String,    // "extraction", "gleaning", "embed"
    pub call_count: usize,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub total_cost_usd: f64,
}
```

Every LLM call gets recorded with its token counts, and costs are calculated using per-model pricing. We can see exactly which operations consume the most tokens.

### What We Learned

**1. Model selection is the biggest lever**

gpt-4o-mini vs gpt-4o is a 33x cost difference. For entity extraction (which is mostly pattern matching), the smaller model performs comparably. We were wasting money on gpt-4o for no quality benefit.

**2. Embedding costs add up quietly**

`text-embedding-3-small` is 6.5x cheaper than `text-embedding-3-large`. For most RAG use cases, the 1536-dimension embeddings work fine.

**3. Per-operation breakdown is essential**

Knowing that "extraction" is 60% of cost vs "embedding" at 10% tells us where to focus optimization. Generic "total cost" metrics don't help with this.

### For Local LLaMA Users (r/LocalLLaMA)

We integrated Ollama as a provider, so you can switch between OpenAI and local models at runtime. With local inference, the marginal cost per document is $0 after hardware investment.

The tradeoff is latency—local models are typically slower. But for batch processing workloads, it's worth it.

### Numbers

- gpt-4o-mini: ~$0.0014 per document
- gpt-4o: ~$0.05 per document
- Ollama: $0 per document (after hardware)

For 10,000 documents:

- gpt-4o-mini: $14
- gpt-4o: $500
- Ollama: $0

### The Code

If you want to see the implementation, it's in our open source project:

- Cost tracking: `edgequake/crates/edgequake-pipeline/src/progress.rs`
- API handlers: `edgequake/crates/edgequake-api/src/handlers/costs.rs`
- GitHub: https://github.com/raphaelmansuy/edgequake

Apache 2.0 licensed.

---

### Questions for the Community

1. How do you track LLM costs in your projects?
2. Are there other optimization strategies beyond model selection that have worked for you?
3. For those using local models—how do you factor in electricity/hardware costs for TCO calculations?

---

_Note: This project implements the LightRAG algorithm from [arXiv:2410.05779](https://arxiv.org/abs/2410.05779). Credit to the original authors._
