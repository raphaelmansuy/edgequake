# The Hidden Economics of LLM-Powered RAG: How EdgeQuake Achieves $0.0014 Per Document

_Cost is the silent killer of production RAG systems. Here's how to build cost visibility into your knowledge graph from day one._

---

## The $10,000 Wake-Up Call

Last year, a fintech startup I consulted with deployed their first RAG system to production. They were thrilled—GPT-4 was extracting knowledge from their compliance documents beautifully. The entity relationships were accurate. The query responses were helpful.

Then the first monthly bill arrived: **$10,247**.

They had processed 50,000 documents. At roughly $0.20 per document with GPT-4, the math was brutal but predictable—if only they had tracked it beforehand.

**This is the story of nearly every production RAG deployment.**

The technology works. The costs explode. Teams scramble to optimize after the damage is done, or worse, they abandon the project entirely.

What if cost tracking was built into your RAG system from day one? What if you knew exactly what each document cost before processing it? What if you could set budget alerts and never receive a surprise bill?

This is exactly what we built into EdgeQuake.

---

## Why Most RAG Frameworks Ignore Cost

The uncomfortable truth: most RAG frameworks treat cost tracking as someone else's problem.

**LangChain**: Offers community-contributed callbacks for token counting, but no built-in cost calculation or budget management.

**LlamaIndex**: Provides token counting through `token_counter`, but leaves cost calculation to users.

**Microsoft GraphRAG**: Ships with no cost tracking whatsoever.

This isn't because cost doesn't matter—it's because these frameworks prioritize flexibility over observability. They assume you'll wrap their components in your own monitoring.

For research and prototyping, that's fine. For production, it's dangerous.

**EdgeQuake takes a different approach: cost observability is a first-class citizen.**

---

## The EdgeQuake Cost Architecture

When we designed EdgeQuake's cost tracking, we built it around three principles:

1. **Track**: Know exactly what you've spent, down to the operation level
2. **Predict**: Estimate costs before processing
3. **Control**: Set budgets and get alerts before overspending

Here's how the architecture works:

```
┌─────────────────────────────────────────────────────────┐
│                   COST TRACKING FLOW                     │
├─────────────────────────────────────────────────────────┤
│                                                          │
│   Document Upload                                        │
│        │                                                 │
│        ▼                                                 │
│   ┌─────────────┐     ┌──────────────────────────┐      │
│   │ CostTracker │────▶│ ModelPricing             │      │
│   │  (per job)  │     │  • input_cost_per_1k     │      │
│   └─────────────┘     │  • output_cost_per_1k    │      │
│        │              │  • calculate_cost()      │      │
│        │              └──────────────────────────┘      │
│        ▼                                                 │
│   ┌─────────────────────────────────────────┐           │
│   │         Pipeline Operations              │           │
│   ├─────────────────────────────────────────┤           │
│   │ Extraction │ tracker.record(...)        │──┐        │
│   │ Gleaning   │ tracker.record(...)        │  │        │
│   │ Summarize  │ tracker.record(...)        │  │        │
│   │ Embedding  │ tracker.record(...)        │  │        │
│   └─────────────────────────────────────────┘  │        │
│        │                                        │        │
│        ▼                                        ▼        │
│   ┌─────────────────────────────────────────────────┐   │
│   │              CostBreakdown (snapshot)            │   │
│   ├─────────────────────────────────────────────────┤   │
│   │ job_id: "doc-abc123"                             │   │
│   │ model: "gpt-4o-mini"                             │   │
│   │ operations:                                       │   │
│   │   extraction: {calls: 4, tokens: 8000, $0.0008} │   │
│   │   gleaning:   {calls: 2, tokens: 2000, $0.0002} │   │
│   │   embedding:  {calls: 1, tokens: 3000, $0.0001} │   │
│   │ total_cost_usd: $0.0011                          │   │
│   └─────────────────────────────────────────────────┘   │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

Every LLM call flows through the `CostTracker`, which applies the appropriate `ModelPricing` configuration and aggregates costs by operation.

---

## How Cost Calculation Works

The core of EdgeQuake's cost tracking is the `ModelPricing` struct. Here's the actual implementation from our codebase:

```rust
/// Pricing configuration for a single model.
pub struct ModelPricing {
    /// Model name.
    pub model: String,
    /// Cost per 1K input tokens (USD).
    pub input_cost_per_1k: f64,
    /// Cost per 1K output tokens (USD).
    pub output_cost_per_1k: f64,
}

impl ModelPricing {
    pub fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
        input_cost + output_cost
    }
}
```

The formula is straightforward:

```
Total Cost = (input_tokens / 1000) × input_rate + (output_tokens / 1000) × output_rate
```

For a typical document with 3000 input tokens and 1000 output tokens using gpt-4o-mini:

```
Cost = (3000 / 1000) × $0.00015 + (1000 / 1000) × $0.0006
     = 3.0 × $0.00015 + 1.0 × $0.0006
     = $0.00045 + $0.0006
     = $0.00105
```

Add multiple extraction passes (gleaning) and you get **~$0.0014 per document**.

---

## The 33x Cost Difference: Model Selection Matters

Here's the pricing table built into EdgeQuake:

| Model           | Input/1K | Output/1K | Cost per 10K Documents |
| --------------- | -------- | --------- | ---------------------- |
| **gpt-4o-mini** | $0.00015 | $0.0006   | **$14**                |
| gpt-3.5-turbo   | $0.0005  | $0.0015   | $20                    |
| claude-3-haiku  | $0.00025 | $0.00125  | $25                    |
| gpt-4o          | $0.005   | $0.015    | $467                   |
| claude-3-sonnet | $0.003   | $0.015    | $450                   |
| gpt-4-turbo     | $0.01    | $0.03     | $1,000                 |
| claude-3-opus   | $0.015   | $0.075    | $1,400                 |

**The difference between gpt-4o-mini and gpt-4o is 33x.**

But here's the key insight: for entity extraction and relationship identification, **gpt-4o-mini achieves the same 40% entity deduplication rate as larger models**.

We tested this extensively. The knowledge graphs produced by both models have comparable quality for RAG use cases. The premium models excel at nuanced reasoning and creative tasks, but entity extraction is pattern matching at scale—exactly what smaller models handle well.

**Our production recommendation: Start with gpt-4o-mini unless you have evidence that larger models improve your specific use case.**

---

## Five Optimization Strategies

### Strategy 1: Smart Model Selection (33x savings)

Switch from gpt-4o to gpt-4o-mini for document processing. Monitor entity quality metrics. Only upgrade if you see quality degradation for your specific domain.

### Strategy 2: Embedding Model Choice (6.5x savings)

| Embedding Model        | Cost/1K tokens | 10K Docs (1M tokens) |
| ---------------------- | -------------- | -------------------- |
| text-embedding-3-small | $0.00002       | $0.02                |
| text-embedding-3-large | $0.00013       | $0.13                |

For most RAG use cases, `text-embedding-3-small` with 1536 dimensions provides excellent recall. The larger model offers diminishing returns unless your domain has highly specialized vocabulary.

### Strategy 3: Optimal Chunking (balance quality vs cost)

Smaller chunks = more LLM calls = higher cost.
Larger chunks = fewer calls = lower cost, but may miss fine-grained entities.

EdgeQuake defaults to 1200 tokens with 100-token overlap—optimized through extensive testing to balance extraction quality against cost.

### Strategy 4: Caching (skip redundant work)

EdgeQuake caches:

- Extracted entities in PostgreSQL
- Embeddings with pgvector
- Document metadata for change detection

When you re-index a document that hasn't changed, the system skips LLM calls entirely. Only modified documents trigger reprocessing.

### Strategy 5: Local Models (zero marginal cost)

For organizations with strict data sovereignty requirements or high-volume processing needs, EdgeQuake integrates with Ollama for local model inference.

After the initial hardware investment, **every additional document costs $0**.

The tradeoff is latency (local models are typically slower than API calls), but for batch processing workloads, this is often acceptable.

---

## Cost Visibility in Practice

EdgeQuake exposes cost data through a comprehensive API:

```
┌─────────────────────────────────────────────────────────┐
│                  COST API ENDPOINTS                      │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  GET /api/v1/pipeline/costs/pricing                     │
│  └─▶ Returns: Available model pricing configurations    │
│                                                          │
│  POST /api/v1/pipeline/costs/estimate                   │
│  └─▶ Returns: Estimated cost before processing          │
│      Body: { model, input_tokens, output_tokens }       │
│                                                          │
│  GET /api/v1/costs/summary                              │
│  └─▶ Returns: Workspace cost totals                     │
│      • total_cost, document_count, avg_per_document     │
│      • by_operation breakdown                            │
│                                                          │
│  GET /api/v1/costs/budget                               │
│  └─▶ Returns: Budget status and alerts                  │
│      • monthly_budget_usd, spent_usd, remaining_usd     │
│      • is_over_budget, alert_threshold                  │
│                                                          │
│  GET /api/v1/costs/history                              │
│  └─▶ Returns: Cost trends over time                     │
│      Params: start_date, end_date, granularity          │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

The WebUI provides real-time dashboards built on these APIs:

- **Cost summary**: Total spend, per-document average, operation breakdown
- **Budget status**: Progress bar, alert indicators, remaining budget
- **Historical trends**: Charts showing cost over days/weeks/months
- **Per-workspace isolation**: Track costs by project or team

---

## Production Recommendation

Based on our experience deploying EdgeQuake at scale, here's the cost optimization playbook:

1. **Start with gpt-4o-mini** → ~$14 for 10,000 documents
2. **Use text-embedding-3-small** → $0.02 for 1M tokens
3. **Monitor via the cost dashboard** → Identify high-cost operations
4. **Set budget alerts at 80%** → Get warnings before overspending
5. **Evaluate Ollama for batch processing** → $0 marginal cost at scale
6. **Review monthly** → Adjust model selection based on quality metrics

---

## The Bigger Picture

Cost tracking isn't just about saving money—it's about enabling production deployment.

When you know exactly what each document costs, you can:

- **Budget accurately** for new projects
- **Price products** that depend on RAG infrastructure
- **Justify investment** to stakeholders with clear ROI
- **Scale confidently** without fear of surprise bills
- **Optimize continuously** with data-driven decisions

The alternative—deploying RAG with blind faith that costs will be "manageable"—is how projects get killed.

---

## Try It Yourself

EdgeQuake is open source under Apache 2.0. The cost tracking features described here are available today:

```bash
# Clone the repository
git clone https://github.com/raphaelmansuy/edgequake

# Start the full stack
make dev

# View cost dashboard
open http://localhost:3000/costs
```

Process a few documents, watch the costs accumulate in real-time, and experience what it feels like to have full visibility into your LLM spend.

---

## Acknowledgments

EdgeQuake implements the LightRAG algorithm as described in ["LightRAG: Simple and Fast Retrieval-Augmented Generation"](https://arxiv.org/abs/2410.05779) by Guo et al. We thank the authors for their foundational work on efficient graph-based retrieval.

---

_What cost optimization strategies have worked for your RAG deployments? I'd love to hear your experiences in the comments._

---

**Tags**: #RAG #LLM #CostOptimization #GraphRAG #EdgeQuake #Rust #MachineLearning #AI #Production
