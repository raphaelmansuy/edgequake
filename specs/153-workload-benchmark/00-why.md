# 00 — WHY (5-WHY)

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

## The job to be done

An operator measuring EdgeQuake on **https://demo.edgequake.com/** must answer:

1. How much **typed** token work each service layer sustains inside SLOs.
2. Whether the knee is EdgeQuake (queue/pool/CPU) or the external LLM/embed provider.
3. What is true for a **named tenant + workspace**, not a mixed multi-tenant soup.

Everything below is why a naive “docs/sec” or single “tokens/sec” number fails that job.

---

## 5-WHY chain A — Mixed token species

| # | Question | Answer |
|---|----------|--------|
| 1 | Why do capacity claims disagree between teams? | One report sums “all tokens”; another counts only completion tokens. |
| 2 | Why does summing feel natural? | Providers bill and report `total_tokens`, and dashboards copy that field. |
| 3 | Why is that wrong for EdgeQuake capacity? | Prefill (`llm_in`), decode (`llm_out`), reasoning, and embeddings have different latency models and different code paths. |
| 4 | Why does the code already split them? | Extraction records provider `prompt_tokens`/`completion_tokens`; embeddings use `estimate_embed_tokens` (`ceil(chars/2.5)`); vision parse still stores `None`. |
| 5 | **Root cause** | **Token species are not fungible.** Capacity must be typed (`llm_in`, `llm_out`, `llm_reasoning`, `embed_est` / `embed_usage`) and never silently mixed. |

```text
  WRONG                              RIGHT
  -----                              -----
  "12k tokens/s"                     L4 llm_in+llm_out goodput @ knee
         |                           L5 embed_est throughput (labeled estimate)
         v                           L8 generated_tokens + TTFT SLO
  hides which layer saturated
```

Cross-ref: [LAW-153-1](01-first-principles.md) · [03-token-ledger](03-token-ledger.md).

---

## 5-WHY chain B — Latency mistaken for capacity

| # | Question | Answer |
|---|----------|--------|
| 1 | Why do we ship “p50 query = 1.2s” as a capacity story? | Latency is easy to measure once. |
| 2 | Why is that not capacity? | Capacity is the **maximum load** that still meets latency SLOs. |
| 3 | Why does load change the answer? | Queues form; Little’s law: concurrency ≈ arrival_rate × sojourn. Past the knee, sojourn explodes and goodput falls. |
| 4 | Why do LLM systems need goodput specifically? | DistServe / AIPerf define goodput as rate under **joint** TTFT/TPOT (or multi-metric) SLO attainment, not average latency alone. |
| 5 | **Root cause** | **Capacity = goodput at the knee**, not a cold single-shot latency. Measurement must sweep offered load until attainment drops below 90%. |

```text
  Load -->
  Goodput
    ^
    |     ****
    |    *    *
    |   *      *----  knee (attainment < 90%)
    |  *
    +------------------> concurrency / arrival rate
```

Cross-ref: [LAW-153-2](01-first-principles.md) · [04-protocol](04-protocol.md).

---

## 5-WHY chain C — Existing benches answer a different question

| # | Question | Answer |
|---|----------|--------|
| 1 | Why do we still lack a token capacity number for demo? | SPEC-001 publishes Acc; SPEC-090 publishes DB physics; neither is token-goodput. |
| 2 | Why can’t Acc double as capacity? | Acc freezes quality under a dual-SUT protocol; it is not a load sweep. |
| 3 | Why can’t SPEC-090 / 063 double as capacity? | They prove claim/pool/ANN floors without LLM token ledgers. |
| 4 | Why is demo special? | Auth is on; MCP scopes are `edgequake:read` / `edgequake:query` (query-only). Ingest load against the shared demo tenant is a different risk class than query L7/L8. |
| 5 | **Root cause** | **No first-class layer×token protocol pinned to a demo tenant.** SPEC-153 closes that gap. |

```text
  SPEC-001  -----> Acc (quality)
  SPEC-090  -----> DB claim / pool / ANN
  SPEC-063  -----> vector proven floors
  SPEC-153  -----> typed-token goodput @ knee
                   pinned to demo tenant+workspace
```

Cross-ref: [README sibling map](README.md) · [02-service-layers](02-service-layers.md) · demo pin below.

---

## Demo execution target (pinned)

Observed **2026-09-29** against live demo:

| Field | Value |
|-------|-------|
| Base URL | `https://demo.edgequake.com` |
| `/live` | `OK` |
| `/ready` | `{"ready":true}` |
| Product version | `0.28.3` (build `20260929.071631`) |
| Storage | PostgreSQL · PG18 · pgvector 0.8.5 · AGE 1.8.0 |
| Schema | migrations_applied 159 · latest_version 161 · pending 0 |
| LLM | `openai` / `gpt-5.4-mini` |
| Embedding | `openai` / `text-embedding-3-small` · dim **1536** |
| Health `workspace_id` string | `"default"` (server default label; not the UUID pin) |
| **Bench tenant_id** | `00000000-0000-0000-0000-000000000002` |
| **Bench workspace_id** | `00000000-0000-0000-0000-000000000003` |
| Auth | Required (`EDGEQUAKE_AUTH_ENABLED=true` on GCP demo) |
| MCP resource | `https://demo.edgequake.com/mcp` |
| MCP scopes | `edgequake:read`, `edgequake:query` (**query-only**; no ingest/delete) |

```text
  https://demo.edgequake.com
            |
            +-- /live /ready /health   (fleet liveness; llm_provider always true)
            |
            +-- REST /api/v1/...       (JWT / API key + X-Tenant-ID + Workspace)
            |
            +-- MCP /mcp               (OAuth; query tools only)
                        |
                        v
              tenant  …0002
              workspace …0003
              documents (sample observed via MCP):
                agentic_2609.31562v1.pdf
                If_You_Think_You_Can_Do_Real-World_Text-to-SQL_...pdf
                ten_2609.18461v1.pdf
                sol_pi_2609.20519v1.pdf
```

**Override:** set `EDGEQUAKE_BENCH_TENANT_ID` / `EDGEQUAKE_BENCH_WORKSPACE_ID` only when deliberately measuring another tenant. The report card must echo the UUIDs used.

**Safety:** Provider-mode **ingest** load sweeps (L1–L6) against this shared demo tenant are **opt-in** and rate-capped. Default demo execution for SPEC-153 is **L7/L8 query + L0 admission**, which matches MCP capabilities and protects shared corpus integrity. Full ingest capacity uses **System bench** (clocked mock) locally or a dedicated bench tenant.

---

## Cost of the status quo

- Operators compare OpenAI “tokens/s” to EdgeQuake wall time and blame the wrong layer.
- Demo Acc screenshots get treated as capacity proof.
- Vision and embed estimates get published as if they were provider usage.
- Multi-tenant queue fairness (`max_tasks_per_tenant`) is invisible without a named tenant pin.

## Why this spec now

We already have: `CostBreakdownStats`, `QueryStats` (including `ttft_ms`), provider capacity semaphores, role-split pools, and a live demo with auth + MCP. SPEC-153 defines the **protocol and ledger** so a later runner can produce comparable report cards without reinventing measurement each release.

Cross-refs: laws in [01](01-first-principles.md) · layers in [02](02-service-layers.md) · protocol in [04](04-protocol.md).
