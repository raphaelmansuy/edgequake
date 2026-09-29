# 01 — First principles (LAW-153)

Parent: [README](README.md) · Why: [00](00-why.md) · Protocol: [04](04-protocol.md)

These laws are derived from queueing theory, DistServe/AIPerf goodput practice, the USE method, and EdgeQuake code — not taste. A report that violates a law is invalid evidence.

## LAW-153-1 — Token species are not fungible

Prefill tokens, completion tokens, reasoning tokens, and embedding tokens are different physical work.

```text
  llm_in        = provider prompt_tokens          (prefill-bound)
  llm_out       = provider completion_tokens      (decode-bound)
  llm_reasoning = reasoning / thinking tokens     (if provider returns them)
  embed_est     = ceil(chars / 2.5)               (EdgeQuake estimate today)
  embed_usage   = provider embedding usage        (MISSING in product path)
```

**Forbidden:** publishing a single undifferentiated “tokens/sec” that sums species.  
**Required:** report each species separately; never add `embed_est` into `llm_*`.

Code: `LLMResponse.{prompt,completion}_tokens` (edgequake-llm); `estimate_embed_tokens` in
[`embeddings.rs`](../../edgequake/crates/edgequake-pipeline/src/pipeline/helpers/embeddings.rs)
(`EMBED_CHARS_PER_TOKEN = 2.5`); vision `prompt_tokens: None` in
[`parse/service.rs`](../../edgequake/crates/edgequake-api/src/handlers/parse/service.rs).

External: NVIDIA AIPerf separates TTFT / TTFO / output-token metrics for reasoning models
([AIPerf metrics reference](https://docs.nvidia.com/aiperf/dev/reference/ai-perf-metrics-reference)).

## LAW-153-2 — Capacity is goodput at the knee

**Goodput** = maximum offered load at which ≥ **90%** of completed units meet **every**
SLO for the measured layer(s), divided by the steady-state window.

```text
  for load in sweep:
      run steady window
      attainment = good_units / completed_units
      if attainment < 0.90: stop; knee = previous load
  capacity = knee_good_units / steady_seconds
```

DistServe (OSDI 2024): *“per-GPU goodput — the maximum request rate that can be served
adhering to the SLO attainment goal”* ([arXiv:2401.09670](https://arxiv.org/abs/2401.09670)).  
AIPerf: `--goodput "metric:threshold …"` — a request counts only if **all** constraints hold.

**Implication:** a cold p50 latency is a **latency sample**, not a capacity claim.

## LAW-153-3 — Service time ≠ sojourn time

```text
  sojourn = queue_wait + service
  service = time inside the layer under test
```

Little’s law: `L ≈ λ × W` (concurrency ≈ arrival_rate × sojourn). Raising concurrency past
the knee inflates `W` and can **reduce** goodput.

**Implication:** every layer report records both `service_ms` and `sojourn_ms` percentiles
(p50/p90/p99). L1 queue wait is first-class, not noise.

## LAW-153-4 — Dual bench: System and Provider

```text
  System bench     Provider bench
  --------------   ----------------
  clocked mock     live LLM + embed
  known token shape  provider usage fields
  isolates EQ layers attributes wall to vendor
         \            /
          v          v
       same typed ledger + same SLOs
```

**System:** `service_ms = overhead_ms + slope_ms_per_token × tokens` (deterministic).  
**Provider:** use provider-reported usage; pair with System to attribute time.

Publishing Provider numbers without a matching System card (same shape, same load grid)
is incomplete attribution.

## LAW-153-5 — Freeze the token shape

Capacity comparisons require a **frozen workload shape** ([05](05-workloads.md)):

- input token band (prompt / chunk / query)
- expected output band (or mock slope)
- which layers light (L0–L8)
- extract caps (SPEC-117) as a controlled variable — they change `llm_out`

Changing the shape mid-sweep invalidates the knee.

## LAW-153-6 — Cache is a mode, not a free lunch

Cold capacity = product LLM caches **off**
(`EDGEQUAKE_LLM_CACHE=0` or keyword/answer overrides per SPEC-103; provider KV mode recorded).

Warm / cache-on is a **separate** report card. Flags to record:
`answer_cache_hit`, `keyword_cache_hit`, `cache_hit_tokens` (SPEC-103 / SPEC-126).

Mixing cold and warm samples in one attainment calculation is forbidden.

## LAW-153-7 — One independent variable per sweep

Pin everything else on the env card ([04](04-protocol.md)):

| Always pin | Examples |
|------------|----------|
| Target | `https://demo.edgequake.com` |
| Tenant / workspace | `…0002` / `…0003` (or override) |
| Provider + models | openai `gpt-5.4-mini` + `text-embedding-3-small` |
| Worker / pool / inflight knobs | see [02](02-service-layers.md) |
| Workload shape id | e.g. `query-chat-v1` |
| Cache mode | cold \| warm |

Sweep **only** offered load (concurrency closed-loop **or** arrival rate open-loop).

## LAW-153-8 — Steady state, percentiles, and proof ladder

1. **Warmup** discarded (not in attainment).  
2. **Steady window** fixed duration or fixed completed-N (protocol chooses; report both).  
3. Report **p50 / p90 / p99** — never mean alone for SLO decisions.  
4. **USE** on the bent knee: utilization, saturation (queue depth, pool wait, provider semaphore), errors.  
5. Follow SPEC-063 proof ladder: unmeasured SLO thresholds are **hypotheses**; only measured knees may be called **proven floors**.

`/health` sets `llm_provider: true` without a live probe
([`health.rs`](../../edgequake/crates/edgequake-api/src/handlers/health.rs)). Health JSON is
**not** a capacity signal (USE may still read `operational.task_queue` pressure).

## Demo-scope corollary

On `https://demo.edgequake.com/`, default SPEC-153 execution is **tenant-scoped** to
`tenant_id=…0002`, `workspace_id=…0003`. Cross-tenant aggregation without stating the pin
is forbidden. MCP query-only scopes imply Provider **ingest** sweeps need a different
admission path (REST + elevated credentials) or stay on System bench.

## Law index

| Law | One-liner |
|-----|-----------|
| LAW-153-1 | Typed tokens; no mixed sums |
| LAW-153-2 | Goodput @ knee (≥90% attainment) |
| LAW-153-3 | Service vs sojourn; Little’s law |
| LAW-153-4 | System + Provider dual bench |
| LAW-153-5 | Frozen shapes |
| LAW-153-6 | Cache is a mode |
| LAW-153-7 | One variable per sweep |
| LAW-153-8 | Steady state, percentiles, USE, proof ladder |
