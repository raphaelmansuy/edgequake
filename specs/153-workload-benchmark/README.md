# SPEC-153 — Workload & Token Capacity Benchmark

> **Status:** Protocol defined (documents only). No runner yet.  
> **Product pin:** EdgeQuake v0.28.3+ (live demo observed 2026-09-29)  
> **Execution target:** [https://demo.edgequake.com/](https://demo.edgequake.com/)  
> **Tenant pin:** `00000000-0000-0000-0000-000000000002`  
> **Workspace pin:** `00000000-0000-0000-0000-000000000003`  
> **Default Provider shape:** `query-chat-v1` (L0 + L7 + L8)  
> **Scope:** Measure **performance and workload capacity** of each EdgeQuake
> service layer, normalized on **typed tokens** (not a single undifferentiated
> “tokens/sec”).  
> **Related:** [SPEC-001 Acc](../001-benchmark/) · [SPEC-063 capacity](../063-architecture-capacity-assessment/) ·
> [SPEC-090 DB perf](../090-performance/) · [SPEC-016 capacity](../016-datalayer-audit/006-capacity/) ·
> [SPEC-103 LLM cache](../103-llm-cache/) · [SPEC-112 pools](../112-connection-pool/) ·
> [SPEC-117 extract caps](../117-extraction-budget/) · [SPEC-126 prompt cache](../126-provider-kv-cache/) ·
> [SPEC-152 MCP](../152-new-mcp-contract/)

## Job to be done

An operator or release engineer must be able to answer, with evidence, **for a
named demo tenant**:

1. **How much typed token work** can each service layer sustain **inside its SLOs**?
2. Which resource **bent the knee** (queue, provider semaphore, pool, CPU)?
3. How much of wall time is **EdgeQuake system** vs **external LLM/embed provider**?
4. Which claims are **proven floors**, which are **hypotheses**, which are **forbidden** until instrumentation exists?

SPEC-001 answers *quality*. SPEC-090 / SPEC-063 answer *database physics*.
SPEC-153 answers *token-normalized capacity per layer*.

## Demo pin (Provider execution)

| Field | Value |
|-------|-------|
| Base URL | `https://demo.edgequake.com` |
| Tenant ID | `00000000-0000-0000-0000-000000000002` |
| Workspace ID | `00000000-0000-0000-0000-000000000003` |
| Auth | Required (JWT / API key); MCP OAuth scopes `edgequake:read`, `edgequake:query` |
| Observed stack | v0.28.3 · PG18 · openai `gpt-5.4-mini` · `text-embedding-3-small` @ 1536 |
| Default layers | L0 admission · L7 retrieve · L8 generate |
| Shared-tenant safety | No Provider ingest storms; L1–L6 Provider = dedicated tenant / System bench |

Full facts + 5-WHY: [00-why](00-why.md). Protocol steps: [04-protocol](04-protocol.md).

## One-screen architecture

```text
  Frozen token shape (05-workloads)
           |
           v
  Offered load sweep (concurrency or arrival rate)
           |
     +-----+-----+
     |           |
     v           v
  System bench   Provider bench
  (clocked mock) (live usage)
     |           |
     +-----+-----+
           |
           v
  Typed token ledger per layer (03)
           |
           v
  Unit is GOOD iff EVERY layer SLO holds
           |
           v
  Knee = max load with attainment >= 90%
           |
           v
  USE signal that saturated (01 LAW-153-2/8)
```

## Capacity definition (headline)

**Goodput at the knee** — the highest offered load at which ≥90% of completed
units meet **all** SLOs for the measured layer(s), divided by the steady-state
window.

- DistServe (OSDI 2024): goodput = max request rate adhering to SLO attainment.
- NVIDIA AIPerf `--goodput`: multi-metric SLO conjunction on the same request.
- Little’s law: past the knee, raising concurrency **lowers** goodput.
- USE method: name the resource that saturated (utilization / saturation / errors).

## Service layers (summary)

| Layer | Bound class | Token claim allowed? |
|-------|-------------|----------------------|
| L0 HTTP admission | Rate limit / CPU | No — requests only |
| L1 Task queue | Queue / DB / provider semaphore | No — tasks + wait |
| L2 PDF convert | CPU (EdgeParse) or LLM (Vision) | Vision **forbidden** until tokens counted |
| L3 Chunk | CPU | Size-gate tokens only (not billed) |
| L4 Entity extract | LLM | `llm_in` + `llm_out` (provider) |
| L5 Embeddings | Embed provider | `embed_est` only today; never sum with L4 |
| L6 Merge + persist | DB | Vectors / nodes / edges; pool USE |
| L7 Query retrieve | DB + query embed | Time arms; keyword LLM separate |
| L8 Query generate | LLM | `context_tokens`, `generated_tokens`, TTFT |

Full map: [02-service-layers](02-service-layers.md).

## Reading order

1. **Why:** [00-why](00-why.md) — three 5-WHY chains  
2. **Laws:** [01-first-principles](01-first-principles.md) — LAW-153-1…8  
3. **Layers:** [02-service-layers](02-service-layers.md) — code anchors + knobs  
4. **Ledger:** [03-token-ledger](03-token-ledger.md) — species, forbidden sums  
5. **Protocol:** [04-protocol](04-protocol.md) — executable test protocol  
6. **Shapes:** [05-workloads](05-workloads.md) — frozen token shapes  
7. **Xref:** [06-cross-ref](06-cross-ref.md) — law ↔ layer ↔ code ↔ sibling specs  
8. **High-load users:** [07-highload-protocol](07-highload-protocol.md) — concurrent Q&A, machines inventory, charts  

Artifacts (when runner exists): [`measurements/`](measurements/).

## Sibling map — who measures what

| Spec | Measures | Does **not** measure |
|------|----------|----------------------|
| SPEC-001 | GraphRAG Acc / fair latency ratio | Token goodput, queue saturation |
| SPEC-047 | Multimodal Acc | Capacity |
| SPEC-063 | ANN / vector proven floors | LLM-token-normalized workload |
| SPEC-090 | Counter, pool, claim_next, ANN order, PDF list | End-to-end RAG tokens |
| SPEC-016 §006 | HNSW RAM + write RT model | Live token knee |
| **SPEC-153** | **Typed-token goodput per L0–L8** | Answer quality (Acc) |

## Dual bench (mandatory pair)

| Mode | Provider | Purpose |
|------|----------|---------|
| **System** | Deterministic token-delay mock (`service = overhead + slope × tokens`) | Isolate queue/chunk/merge/retrieve without vendor noise |
| **Provider** | Live LLM + embed; use provider `prompt_tokens` / `completion_tokens` | Real token accounting; pair with System to attribute wall time |

Cache **off** is the cold capacity number. Cache **on** is a separate mode
(`answer_cache_hit`, `keyword_cache_hit`, `cache_hit_tokens` — SPEC-103 / 126).

## Success criteria (this pack)

| ID | Criterion |
|----|-----------|
| S1 | 5-WHY root causes documented and cross-linked to laws |
| S2 | Every layer has a code anchor and an allowed claim class |
| S3 | Token species are typed; mixed sums are forbidden |
| S4 | Protocol is executable without inventing “proven” tokens/sec |
| S5 | Workloads freeze input/output token shapes and which layers they light |
| S6 | Cross-ref matrix ties laws ↔ layers ↔ symbols ↔ siblings |

## Non-goals (this pass)

- No harness, Makefile target, or Criterion/k6 suite yet.
- No new production metrics code (vision usage, provider embed `usage`).
- No invented proven throughput numbers in FAQ/docs.
- No replacement of SPEC-001 Acc or SPEC-090 DB physics.
- Vision-token capacity and provider embedding `usage` are **explicit gaps**,
  not silent zeros.

## Commands (future runner — named only)

```text
# Placeholder names for a later WP; not implemented in this pack:
make bench153-doctor          # env card + functional oracle
make bench153-system          # clocked mock knee sweep
make bench153-provider        # live usage knee sweep
make bench153-report          # merge JSON → measurements/
```

Pass/fail for a run is: **report card complete and internally consistent**
(LAW-153-8), not “hit an arbitrary tokens/sec target.”
