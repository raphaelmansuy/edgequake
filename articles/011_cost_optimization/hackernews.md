# Show HN: Built-in LLM Cost Tracking for Graph-RAG (EdgeQuake)

**Title**: Show HN: EdgeQuake – Graph-RAG with built-in LLM cost tracking

---

## Post Body

Hi HN,

I've been building EdgeQuake, a Rust-based Graph-RAG framework that implements the LightRAG algorithm [1]. One feature I wanted to share is the built-in cost tracking system.

**The Problem**

Most RAG frameworks treat cost tracking as an afterthought. You process documents, get a surprise bill, then scramble to optimize. I've seen teams abandon projects after realizing their GPT-4 RAG was costing $0.20+ per document at scale.

**The Approach**

EdgeQuake tracks costs at the operation level (extraction, gleaning, embedding) with a simple formula:

```rust
pub fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
    let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
    let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
    input_cost + output_cost
}
```

Every LLM call flows through a `CostTracker` that applies per-model pricing and aggregates costs by operation. The result is a `CostBreakdown` struct that shows exactly where your tokens went.

**The Numbers**

With gpt-4o-mini, we're seeing ~$0.0014 per document for typical knowledge extraction. Compare that to gpt-4o at ~$0.05 per document—a 33x difference with comparable extraction quality for entity/relationship identification.

**Built-in Pricing**

The system ships with pricing for OpenAI, Anthropic, and embedding models:

```rust
pricing.insert("gpt-4o-mini", ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006));
pricing.insert("gpt-4o", ModelPricing::new("gpt-4o", 0.005, 0.015));
pricing.insert("claude-3-haiku", ModelPricing::new("claude-3-haiku", 0.00025, 0.00125));
```

**API Endpoints**

The cost data is exposed via REST:

- `GET /api/v1/pipeline/costs/pricing` – Available model pricing
- `POST /api/v1/pipeline/costs/estimate` – Pre-calculate cost before processing
- `GET /api/v1/costs/summary` – Workspace totals and per-operation breakdown
- `GET /api/v1/costs/budget` – Budget status and alerts
- `GET /api/v1/costs/history` – Cost trends over time

**Local Models**

For Ollama users, the marginal cost is $0. EdgeQuake's provider abstraction lets you switch between OpenAI and local models at runtime.

**Source**

- GitHub: https://github.com/raphaelmansuy/edgequake
- Apache 2.0 license
- Cost tracking: `edgequake/crates/edgequake-pipeline/src/progress.rs`

---

Interested in feedback on:

1. What pricing models are you using that aren't included?
2. How do you handle cost tracking in your LLM applications?
3. Are there cost metrics you'd want that we haven't thought of?

---

[1] LightRAG: Simple and Fast Retrieval-Augmented Generation - https://arxiv.org/abs/2410.05779
