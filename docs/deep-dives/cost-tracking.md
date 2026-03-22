---
title: 'Deep Dive: Cost Tracking'
---

# Deep Dive: Cost Tracking

> **How EdgeQuake Tracks and Reports LLM Costs**

LLM operations have real monetary costs. EdgeQuake provides comprehensive cost tracking to help you monitor, budget, and optimize your knowledge graph operations.

---

## Overview

Cost tracking captures token usage across all LLM operations:

```
┌─────────────────────────────────────────────────────────────────┐
│                    COST TRACKING PIPELINE                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Document Ingestion:                                            │
│                                                                 │
│    ┌────────────────────────────────────────────────────────┐   │
│    │ Extract Entities                                       │   │
│    │ ─────────────────                                      │   │
│    │ Input: 2,500 tokens  × $0.00015/1K = $0.000375         │   │
│    │ Output: 800 tokens   × $0.0006/1K  = $0.000480         │   │
│    │                                     ──────────         │   │
│    │                            Subtotal: $0.000855         │   │
│    └────────────────────────────────────────────────────────┘   │
│                                                                 │
│    ┌────────────────────────────────────────────────────────┐   │
│    │ Gleaning (pass 2)                                      │   │
│    │ ─────────────────                                      │   │
│    │ Input: 3,200 tokens  × $0.00015/1K = $0.000480         │   │
│    │ Output: 600 tokens   × $0.0006/1K  = $0.000360         │   │
│    │                                     ──────────         │   │
│    │                            Subtotal: $0.000840         │   │
│    └────────────────────────────────────────────────────────┘   │
│                                                                 │
│    ┌────────────────────────────────────────────────────────┐   │
│    │ Embeddings (5 chunks)                                  │   │
│    │ ──────────────────────                                 │   │
│    │ Input: 6,000 tokens  × $0.00002/1K = $0.000120         │   │
│    │ Output: 0 tokens     × $0.0/1K     = $0.000000         │   │
│    │                                     ──────────         │   │
│    │                            Subtotal: $0.000120         │   │
│    └────────────────────────────────────────────────────────┘   │
│                                                                 │
│    ════════════════════════════════════════════════════════════ │
│    TOTAL: $0.001815 (~$0.0018 per document)                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why Track Costs?

| Purpose               | Benefit                       |
| --------------------- | ----------------------------- |
| **Budget Management** | Prevent unexpected bills      |
| **Optimization**      | Identify expensive operations |
| **Comparison**        | Evaluate different models     |
| **Chargeback**        | Bill per-workspace usage      |

---

## Core Data Structures

### ModelPricing

Pricing configuration for a model:

```rust
/// Model pricing information (per 1K tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Model name
    pub model: String,

    /// Cost per 1K input tokens (USD)
    pub input_cost_per_1k: f64,

    /// Cost per 1K output tokens (USD)
    pub output_cost_per_1k: f64,
}

impl ModelPricing {
    /// Calculate cost for token usage
    pub fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
        input_cost + output_cost
    }
}
```

### OperationCost

Cost breakdown by operation type:

```rust
/// Cost for a single operation type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationCost {
    /// Operation type (extract, glean, summarize, embed)
    pub operation: String,

    /// Number of LLM calls
    pub call_count: usize,

    /// Total input tokens consumed
    pub input_tokens: usize,

    /// Total output tokens generated
    pub output_tokens: usize,

    /// Total cost (USD)
    pub total_cost_usd: f64,
}
```

### CostBreakdown

Complete cost summary for a job:

```rust
/// Complete cost breakdown for an ingestion job.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// Job ID
    pub job_id: String,

    /// Model used
    pub model: String,

    /// Per-operation costs
    pub operations: HashMap<String, OperationCost>,

    /// Total input tokens
    pub total_input_tokens: usize,

    /// Total output tokens
    pub total_output_tokens: usize,

    /// Total cost (USD)
    pub total_cost_usd: f64,
}
```

### CostTracker

Thread-safe tracker for concurrent operations:

```rust
/// Thread-safe cost tracker.
pub struct CostTracker {
    inner: Arc<RwLock<CostBreakdown>>,
    pricing: ModelPricing,
}

impl CostTracker {
    /// Create with gpt-5-nano pricing
    pub fn new_gpt4o_mini(job_id: impl Into<String>) -> Self;

    /// Create with gpt-4o pricing
    pub fn new_gpt4o(job_id: impl Into<String>) -> Self;

    /// Record token usage for an operation
    pub async fn record(&self, operation: &str, input_tokens: usize, output_tokens: usize);

    /// Get current cost breakdown
    pub async fn snapshot(&self) -> CostBreakdown;

    /// Get total cost so far
    pub async fn total_cost(&self) -> f64;
}
```

---

## Default Model Pricing

EdgeQuake includes built-in pricing for common models:

### OpenAI Models

| Model           | Input (per 1K) | Output (per 1K) | Use Case                        |
| --------------- | -------------- | --------------- | ------------------------------- |
| `gpt-5-nano`   | $0.00015       | $0.0006         | Entity extraction (recommended) |
| `gpt-4o`        | $0.005         | $0.015          | Complex reasoning               |
| `gpt-4-turbo`   | $0.01          | $0.03           | Legacy applications             |
| `gpt-3.5-turbo` | $0.0005        | $0.0015         | Budget option                   |

### Anthropic Models

| Model             | Input (per 1K) | Output (per 1K) | Use Case        |
| ----------------- | -------------- | --------------- | --------------- |
| `claude-3-haiku`  | $0.00025       | $0.00125        | Fast, cheap     |
| `claude-3-sonnet` | $0.003         | $0.015          | Balanced        |
| `claude-3-opus`   | $0.015         | $0.075          | Highest quality |

### Embedding Models

| Model                    | Input (per 1K) | Output | Use Case           |
| ------------------------ | -------------- | ------ | ------------------ |
| `text-embedding-3-small` | $0.00002       | N/A    | Default embeddings |
| `text-embedding-3-large` | $0.00013       | N/A    | Higher quality     |

---

## Usage

### Basic Cost Tracking

```rust
use edgequake_pipeline::progress::{CostTracker, ModelPricing};

// Create tracker with gpt-5-nano pricing
let tracker = CostTracker::new_gpt4o_mini("job-123");

// Record entity extraction
tracker.record("extract", 2500, 800).await;

// Record gleaning pass
tracker.record("glean", 3200, 600).await;

// Get total cost
let total = tracker.total_cost().await;
println!("Total cost: ${:.4}", total);
```

### Custom Model Pricing

```rust
// Define custom pricing (e.g., for Ollama, free)
let pricing = ModelPricing::new("llama3", 0.0, 0.0);
let tracker = CostTracker::new("job-123", "llama3", pricing);

// All operations are free!
tracker.record("extract", 10000, 5000).await;
assert_eq!(tracker.total_cost().await, 0.0);
```

### Get Cost Breakdown

```rust
let breakdown = tracker.snapshot().await;

println!("Job: {}", breakdown.job_id);
println!("Model: {}", breakdown.model);
println!("Total: ${:.4}", breakdown.total_cost_usd);
println!();

for (op, cost) in &breakdown.operations {
    println!("{}: {} calls, {} in / {} out, ${:.4}",
             op, cost.call_count,
             cost.input_tokens, cost.output_tokens,
             cost.total_cost_usd);
}
```

---

## Operation Types

Costs are tracked by operation type:

| Operation   | Description                    | Typical Ratio  |
| ----------- | ------------------------------ | -------------- |
| `extract`   | Entity/relationship extraction | 60-70% of cost |
| `glean`     | Multi-pass refinement          | 15-25% of cost |
| `summarize` | Community summaries            | 5-10% of cost  |
| `embed`     | Embedding generation           | 5-10% of cost  |
| `query`     | Query processing               | Per-query cost |

---

## Cost Optimization

### Model Selection

```
┌─────────────────────────────────────────────────────────────────┐
│                    COST VS QUALITY TRADEOFFS                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Cost                                                           │
│   ▲                                                             │
│   │                                                             │
│   │                              ● gpt-4-turbo                  │
│   │                                                             │
│   │                    ● gpt-4o                                 │
│   │                                                             │
│   │          ● claude-3-sonnet                                  │
│   │                                                             │
│   │  ● gpt-5-nano (recommended)                                │
│   │  ● claude-3-haiku                                           │
│   │                                                             │
│   └──────────────────────────────────────────────────────▶      │
│                                                    Quality      │
│                                                                 │
│  Recommendation: gpt-5-nano offers best cost/quality ratio     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Reduce Token Usage

1. **Shorter Chunks** - Reduce chunk size (tradeoff: less context)
2. **Fewer Gleaning Passes** - Use `max_gleaning_iterations: 1`
3. **Skip Summarization** - Disable if not using Global queries
4. **Batch Processing** - Combine small documents

### Cost Per Document

Typical costs with `gpt-5-nano`:

| Document Size | Chunks  | Entities | Cost    |
| ------------- | ------- | -------- | ------- |
| 1 KB          | 1       | 5-10     | ~$0.001 |
| 10 KB         | 3-5     | 20-40    | ~$0.003 |
| 100 KB        | 20-30   | 80-150   | ~$0.015 |
| 1 MB          | 100-150 | 400-600  | ~$0.10  |

---

## API Integration

### Get Cost Breakdown via API

```bash
# Get current pricing configuration
curl http://localhost:8080/api/v1/pipeline/costs/pricing

# Response
{
  "models": {
    "gpt-5-nano": {
      "input_cost_per_1k": 0.00015,
      "output_cost_per_1k": 0.0006
    },
    "gpt-4o": {
      "input_cost_per_1k": 0.005,
      "output_cost_per_1k": 0.015
    }
  }
}
```

### Cost in Ingestion Response

```bash
# Upload document
curl -X POST "http://localhost:8080/api/v1/rag/upload" \
  -F "files=@document.pdf"

# Response includes cost
{
  "job_id": "job-abc123",
  "status": "completed",
  "documents_processed": 1,
  "cost": {
    "total_cost_usd": 0.0014,
    "operations": {
      "extract": { "calls": 3, "cost_usd": 0.0010 },
      "embed": { "calls": 5, "cost_usd": 0.0004 }
    }
  }
}
```

---

## Real-World Cost Examples

### Small Knowledge Base (100 documents)

```
Documents: 100 (avg 5KB each)
Model: gpt-5-nano

Extraction:   ~$0.10
Gleaning:     ~$0.05
Embeddings:   ~$0.02
─────────────────────
Total:        ~$0.17 (~$0.0017/doc)
```

### Medium Knowledge Base (1,000 documents)

```
Documents: 1,000 (avg 20KB each)
Model: gpt-5-nano

Extraction:   ~$2.50
Gleaning:     ~$1.50
Embeddings:   ~$0.30
─────────────────────
Total:        ~$4.30 (~$0.0043/doc)
```

### Large Knowledge Base (10,000 documents)

```
Documents: 10,000 (avg 50KB each)
Model: gpt-5-nano

Extraction:   ~$75.00
Gleaning:     ~$45.00
Embeddings:   ~$8.00
─────────────────────
Total:        ~$128 (~$0.013/doc)
```

---

## Query Costs

Each query also incurs LLM costs:

| Query Mode | Typical Cost |
| ---------- | ------------ |
| Naive      | ~$0.0005     |
| Local      | ~$0.0008     |
| Global     | ~$0.0015     |
| Hybrid     | ~$0.0012     |

**Example:** 1,000 queries/day with Hybrid mode:

- Daily: 1,000 × $0.0012 = $1.20
- Monthly: ~$36

---

## Best Practices

1. **Start with gpt-5-nano** - Best cost/quality ratio
2. **Monitor Per-Workspace** - Track costs by tenant
3. **Set Budget Alerts** - Implement spending limits
4. **Review Cost Breakdown** - Identify optimization opportunities
5. **Consider Local Models** - Ollama is free (but slower)

---

## Troubleshooting

### Unexpectedly High Costs

**Check:**

1. Number of gleaning passes (default: 2)
2. Chunk size (larger = more tokens per call)
3. Document complexity (dense text = more entities)

**Solutions:**

- Reduce `max_gleaning_iterations`
- Use smaller model for initial extraction
- Implement document filtering

### Missing Cost Data

**Cause:** Operations not using CostTracker

**Solution:** Ensure all LLM calls go through tracked pipeline

---

## See Also

- [Pipeline Progress](/docs/deep-dives/pipeline-progress/) - Real-time progress tracking
- [Operations Guide](/docs/operations/monitoring/) - Production monitoring
- [Configuration](/docs/operations/configuration/) - Model settings
