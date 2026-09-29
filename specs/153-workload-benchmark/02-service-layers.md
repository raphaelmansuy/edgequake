# 02 — Service layers (L0–L8)

Parent: [README](README.md) · Laws: [01](01-first-principles.md) · Ledger: [03](03-token-ledger.md)

Each layer may publish only the claim class in the rightmost column. Times are always
`service_ms` and `sojourn_ms` (LAW-153-3).

## Layer map

```text
  Client / MCP / REST
        |
        v
  L0  HTTP admission (+ rate limiter)
        |
        +-- sync query/chat ---------------------> L7 retrieve -> L8 generate
        |
        v
  L1  Task queue / WorkerPool / provider semaphore
        |
        v
  L2  PDF convert (EdgeParse CPU | Vision LLM)
        |
        v
  L3  Chunk (CPU size-gate)
        |
        +-----> L4 Extract (LLM) ----+
        |                            |
        +-----> L5 Embed ------------+--> L6 Merge + Postgres persist
```

## L0 — HTTP admission

| | |
|--|--|
| **Bound** | CPU + token-bucket rate limit |
| **Entrypoints** | Axum routes in `edgequake-api` (`handlers/`, `routes.rs`) |
| **Knobs** | `edgequake-rate-limiter` — default **100**/window, burst **20** |
| **Allowed claim** | Admitted vs rejected requests/sec; 429 rate |
| **Token claim** | **Forbidden** — requests are the unit |
| **Demo note** | Hits `https://demo.edgequake.com` with auth; measure 401/429 separately from capacity errors |

## L1 — Task queue & workers

| | |
|--|--|
| **Bound** | Queue DB + worker pool + provider in-flight |
| **Code** | [`edgequake-tasks`](../../edgequake/crates/edgequake-tasks/) — `WorkerPool`, `claim_next`, [`provider_capacity.rs`](../../edgequake/crates/edgequake-tasks/src/provider_capacity.rs), `admission.rs` |
| **Knobs** | `EDGEQUAKE_TASK_MAX_WORKERS` (metrics default **4**); workers default `num_cpus*4` (min 4); `EDGEQUAKE_PROVIDER_MAX_INFLIGHT` (local default **1**, cloud **0**=unlimited); admission **512 MiB** |
| **Allowed claim** | Tasks claimed/sec; claim latency vs pending N; wait in queue |
| **Token claim** | **Forbidden** at L1 — tokens accrue in L4/L5/L8 |
| **Sibling** | SPEC-090 M-5.1 claim cost |

```text
  pending tasks --SKIP LOCKED--> worker
        |                          |
        |                   acquire provider semaphore
        |                          |
        v                          v
  queue wait (sojourn - service)   process document
```

## L2 — PDF convert

| | |
|--|--|
| **Bound** | **CPU** (EdgeParse) or **LLM** (Vision VLM) |
| **Code** | `edgequake-pdf` · `PdfConverter::convert`; API [`parse/service.rs`](../../edgequake/crates/edgequake-api/src/handlers/parse/service.rs) |
| **Allowed claim (EdgeParse)** | Pages/sec, service_ms/page |
| **Allowed claim (Vision)** | Pages/sec **only** until tokens instrumented |
| **Token claim** | **Forbidden** today — `prompt_tokens` / `completion_tokens` are `None` |
| **Demo note** | Default SPEC-153 demo runs **do not** stress Vision ingest on the shared tenant |

## L3 — Chunk

| | |
|--|--|
| **Bound** | CPU |
| **Code** | `edgequake-pipeline` chunker; `token_estimator` / tiktoken `cl100k_base` (else chars/4) for **sizing** |
| **Allowed claim** | Chunks/sec; chars→chunks ratio |
| **Token claim** | Size-gate token counts only — **not** billed usage |

## L4 — Entity extraction (LLM)

| | |
|--|--|
| **Bound** | LLM chat (provider) |
| **Code** | `extractor/sota.rs`, `gleaning.rs`, `assign_token_usage`; stats in `CostBreakdownStats.extraction_*` |
| **Knobs** | `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` (cloud **16**, local **1**, cap **32**); SPEC-117 caps entities **40** / records **100** |
| **Allowed claim** | Goodput of `llm_in + llm_out` (species reported separately) under extract SLOs |
| **Token claim** | Provider `prompt_tokens` / `completion_tokens` only |

## L5 — Embeddings

| | |
|--|--|
| **Bound** | Embedding provider (often parallel to LLM) |
| **Code** | [`embeddings.rs`](../../edgequake/crates/edgequake-pipeline/src/pipeline/helpers/embeddings.rs) — `estimate_embed_tokens`, `EMBED_CHARS_PER_TOKEN=2.5` |
| **Knobs** | `EDGEQUAKE_EMBEDDING_BATCH_SIZE` (else `provider.max_batch_size()`) |
| **Allowed claim** | `embed_est` tokens/sec labeled **estimate**; batch size vs latency |
| **Token claim** | `embed_usage` **forbidden until** provider usage is plumbed |
| **Rule** | Never sum L5 into L4 goodput (LAW-153-1) |

## L6 — Merge + persist

| | |
|--|--|
| **Bound** | Postgres (ingest pool) + optional merge LLM |
| **Code** | pipeline merger; `PgPoolBundle` ingest role |
| **Knobs** | `EDGEQUAKE_DB_POOL_SIZE_INGEST` default **12**; merge async default **8** |
| **Allowed claim** | Vectors/nodes/edges upserted/sec; pool util / wait (USE) |
| **Token claim** | Only if merge summarization LLM is on — then typed as L4-class species on that path |
| **Sibling** | SPEC-090 / SPEC-016 write RT model |

## L7 — Query retrieve

| | |
|--|--|
| **Bound** | Query embed + ANN/graph (+ keyword LLM separate) |
| **Code** | [`QueryEngine`](../../edgequake/crates/edgequake-query/) · [`QueryStats`](../../edgequake/crates/edgequake-query/src/types.rs) |
| **Fields** | `embedding_time_ms`, `keyword_time_ms`, `retrieval_time_ms`, arm timings |
| **Knobs** | `EDGEQUAKE_DB_POOL_SIZE_QUERY` default **16**; read-path semaphore |
| **Allowed claim** | Retrieve sojourn under SLO; keyword path as **LLM** sub-arm (not embed) |
| **Demo pin** | Default Provider bench target on demo tenant `…0002` / workspace `…0003` |

## L8 — Query generate

| | |
|--|--|
| **Bound** | Answer LLM (streaming preferred for TTFT) |
| **Code** | `engine_impl/prompt.rs`; streaming accumulator; `QueryStats.{ttft_ms,context_tokens,generated_tokens,generation_time_ms}` |
| **Allowed claim** | Goodput of `generated_tokens` under TTFT + e2e latency SLOs; `context_tokens` reported |
| **Token claim** | Provider usage; reasoning tokens separate (AIPerf TTFT vs TTFO) |
| **Cache** | Record `answer_cache_hit` / `keyword_cache_hit` (LAW-153-6) |

## Pool defaults (L1/L6/L7)

From [`pool_bundle.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/pool_bundle.rs):

| Role | Env | Default |
|------|-----|---------|
| Query | `EDGEQUAKE_DB_POOL_SIZE_QUERY` | 16 |
| Ingest | `EDGEQUAKE_DB_POOL_SIZE_INGEST` | 12 |
| Queue | `EDGEQUAKE_DB_POOL_SIZE_QUEUE` | 4 |
| Admin | `EDGEQUAKE_DB_POOL_SIZE_ADMIN` | 2 |

## Demo layer default matrix

| Layer | System bench (local) | Provider on demo |
|-------|----------------------|------------------|
| L0 | Optional | **Yes** (auth + rate limit) |
| L1–L6 | **Yes** (mock) | Opt-in dedicated tenant only |
| L7–L8 | Yes (mock retrieve+generate) | **Yes** (default demo execution) |

Health `components.llm_provider` is always `true` — do not gate layer readiness on it.
