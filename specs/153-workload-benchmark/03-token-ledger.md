# 03 — Token ledger

Parent: [README](README.md) · Laws: [01](01-first-principles.md) · Layers: [02](02-service-layers.md)

## Species catalog

| Species | Definition | Source of truth in EdgeQuake | Trust |
|---------|------------|------------------------------|-------|
| `llm_in` | Prompt / prefill tokens | Provider `prompt_tokens` → extract / keyword / answer records | **Counted** when provider returns usage |
| `llm_out` | Completion / decode tokens | Provider `completion_tokens` | **Counted** when provider returns usage |
| `llm_reasoning` | Reasoning / thinking tokens | Provider reasoning fields when present | **Counted** only if plumbed; else **missing** |
| `embed_est` | Estimated embedding tokens | `estimate_embed_tokens` = `ceil(chars / 2.5)` | **Estimated** |
| `embed_usage` | Provider embedding usage | Not wired on product ingest path | **Missing** |
| `vision_*` | Vision OCR tokens | `parse/service.rs` sets `None` | **Missing** — claim forbidden |
| `size_gate` | Chunk sizing tokens | tiktoken / chars÷4 estimators | **Gate only** — not capacity currency |

```text
  COUNTED          ESTIMATED         MISSING (do not invent)
  -------          ---------         -----------------------
  llm_in           embed_est         embed_usage
  llm_out                            vision prompt/completion
  (llm_reasoning*)                   Ollama-compat len/4 as "truth"
```

\* Reasoning: follow AIPerf — TTFT includes first token of any type; TTFO is first
non-reasoning output token. Never fold reasoning into `generated_tokens` without a label.

## Aggregation rules

```text
  ALLOWED
  -------
  L4_goodput = (Σ llm_in + Σ llm_out) / steady_s     # still report species split
  L8_out_goodput = Σ llm_out / steady_s
  L5_est_rate = Σ embed_est / steady_s               # label: estimate

  FORBIDDEN
  ---------
  Σ(llm_* + embed_est)           # mixed physics
  Σ(llm_* + vision_None→0)       # silent zero
  using size_gate as billed work
  using Ollama shim len/4 as capacity truth
```

## Where the code writes the ledger

| Path | Struct / symbol | Species |
|------|-----------------|---------|
| Extract | `ExtractionResult` via `assign_token_usage`; `CostBreakdownStats.extraction_{input,output}_tokens` | `llm_in`, `llm_out` |
| Embed ingest | `CostBreakdownStats.embedding_tokens` ← `estimate_embed_tokens` | `embed_est` |
| Cost tracker | `CostTracker::record(operation, input, output)` | priced `llm_*` |
| Query | `QueryStats.context_tokens`, `generated_tokens` | context / `llm_out` |
| Query timing | `embedding_time_ms`, `keyword_time_ms`, `generation_time_ms`, `ttft_ms` | time arms (not tokens) |
| Streaming | `TokenUsage { prompt_tokens, completion_tokens }` | `llm_in`, `llm_out` |
| Vision parse | metrics `prompt_tokens: None`, `completion_tokens: None` | **gap** |
| Health | `llm_provider: true` hardcoded | **not a ledger** |

## Report-card fields (per completed unit)

```text
  unit_id
  layer_set                 # e.g. [L7, L8]
  shape_id                  # from 05-workloads
  tenant_id                 # required on demo
  workspace_id
  cache_mode                # cold | warm
  bench_mode                # system | provider

  llm_in llm_out llm_reasoning
  embed_est embed_usage     # usage null if missing
  context_tokens generated_tokens

  service_ms sojourn_ms
  ttft_ms itl_ms            # streaming L8
  embedding_time_ms keyword_time_ms retrieval_time_ms generation_time_ms

  http_status oracle_ok
  error_class               # none | auth | rate_limit | 5xx | timeout | parse
```

## Demo tenant ledger pin

Every unit recorded against live demo **must** include:

```text
  base_url      = https://demo.edgequake.com
  tenant_id     = 00000000-0000-0000-0000-000000000002   # unless override
  workspace_id  = 00000000-0000-0000-0000-000000000003
  providers     = openai/gpt-5.4-mini + text-embedding-3-small@1536
```

Cross-tenant rolls without per-tenant breakdown violate LAW-153-7’s pin rule.

## Gaps register (explicit)

| Gap ID | Description | Until fixed |
|--------|-------------|-------------|
| G-153-VISION | Vision tokens `None` | No Vision token capacity claim |
| G-153-EMBED-USAGE | Embeddings estimated only | Publish `embed_est` labeled; no `embed_usage` |
| G-153-HEALTH-LLM | Health LLM always true | Probe elsewhere for USE |
| G-153-OLLAMA-SHIM | Compat API `len/4` | Not capacity truth |

Closing a gap requires code + a new measurement artifact — not a doc-only edit of this table’s trust column.
