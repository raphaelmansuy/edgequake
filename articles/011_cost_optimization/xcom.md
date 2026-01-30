# 🔢 $0.0014 Per Document: The Economics of Production RAG

## Thread for X.com (Twitter)

---

### Tweet 1 (Hook)

🔢 $0.0014 per document.

Not a typo.

That's what it costs to process a document through a production Graph-RAG system.

Here's the math behind LLM cost optimization 🧵

---

### Tweet 2 (The Problem)

First, the problem.

A fintech startup deployed GPT-4 RAG.

First month's bill: $10,247

50,000 documents × $0.20 each = project nearly killed.

This is how most production RAG deployments die.

---

### Tweet 3 (The Root Cause)

The root cause?

No cost visibility.

Most RAG frameworks treat cost tracking as "someone else's problem."

• LangChain: Community callbacks only
• LlamaIndex: Token counting only  
• GraphRAG: Nothing

EdgeQuake: Built-in cost observability.

---

### Tweet 4 (The Formula)

The cost formula is simple:

```
Cost = (input_tokens / 1000) × input_rate
     + (output_tokens / 1000) × output_rate
```

For gpt-4o-mini:
• Input: $0.00015 / 1K tokens
• Output: $0.0006 / 1K tokens

Typical doc: ~$0.0014

---

### Tweet 5 (The Rust Implementation)

Here's the actual Rust implementation:

```rust
pub fn calculate_cost(
    &self,
    input_tokens: usize,
    output_tokens: usize
) -> f64 {
    (input_tokens as f64 / 1000.0) * self.input_cost_per_1k
    + (output_tokens as f64 / 1000.0) * self.output_cost_per_1k
}
```

Every token tracked. Every call logged.

---

### Tweet 6 (The 33x Difference)

The model selection decision changes everything:

| Model       | 10K Docs |
| ----------- | -------- |
| gpt-4o-mini | $14      |
| gpt-4o      | $467     |

That's a **33x** cost difference.

Same extraction quality. Same 40% entity deduplication.

---

### Tweet 7 (Embedding Costs)

Embeddings matter too:

• text-embedding-3-small: $0.00002/1K
• text-embedding-3-large: $0.00013/1K

6.5x difference.

For most RAG use cases, the small model works fine.

---

### Tweet 8 (Five Strategies)

Five cost optimization strategies:

1. Model selection → 33x savings
2. Embedding choice → 6.5x savings
3. Smart chunking → Fewer LLM calls
4. Caching → Skip unchanged docs
5. Local models → $0 marginal cost

Stack them all.

---

### Tweet 9 (Ollama Option)

The nuclear option: Ollama.

Run models locally.

After hardware investment: **$0 per document**.

Tradeoff: Slower inference.

For batch processing? Worth it.

---

### Tweet 10 (Budget Alerts)

EdgeQuake includes budget alerts:

```
GET /api/v1/costs/budget

Response:
{
  "monthly_budget_usd": 100.0,
  "spent_usd": 42.0,
  "remaining_usd": 58.0,
  "alert_threshold": 80.0
}
```

Get warned before you overspend.

---

### Tweet 11 (Cost Dashboard)

The WebUI shows:

• Real-time cost per document
• Per-operation breakdown (extraction vs embedding)
• Historical cost trends
• Workspace isolation

No more blind deployments.

---

### Tweet 12 (The Insight)

The insight:

Cost tracking isn't about saving money.

It's about **enabling production deployment**.

When costs are predictable:
→ Projects get approved
→ Products get priced correctly
→ Teams scale confidently

---

### Tweet 13 (LightRAG Credit)

EdgeQuake implements the LightRAG algorithm from:

"LightRAG: Simple and Fast Retrieval-Augmented Generation"
arxiv.org/abs/2410.05779

Thanks to Guo et al. for the foundational research.

---

### Tweet 14 (CTA)

Try it yourself:

```bash
git clone github.com/raphaelmansuy/edgequake
make dev
```

Open http://localhost:3000/costs

Watch the cost dashboard in real-time.

Apache 2.0 licensed. Free forever.

---

### Tweet 15 (Engagement)

What cost optimization strategies have worked for your LLM deployments?

Reply with your learnings 👇

---

**End of Thread**
