# 05 — Frozen workloads (token shapes)

Parent: [README](README.md) · Protocol: [04](04-protocol.md) · Layers: [02](02-service-layers.md)

Each shape is immutable for a comparative series. Bump the `-vN` suffix when
input/output bands or lit layers change (LAW-153-5).

## Shape template

```text
  shape_id
  intent
  lit_layers
  bench_modes_allowed          # system | provider | both
  demo_default                 # yes/no (provider on demo.edgequake.com)
  input_spec                   # chars / tokens / pages
  output_band                  # expected llm_out or mock slope
  controlled_vars              # extract caps, mode, streaming
  oracle
```

Token bands below are **design targets** for generators/mocks. Provider runs record
**actual** usage; large drift from the band is a shape violation (investigate prompt
templates / caps before claiming a knee).

---

## `queue-only-v1`

| | |
|--|--|
| Intent | Isolate L1 claim/lease without LLM work |
| Lit layers | L0 (optional), L1 |
| Modes | **system** (instant mock processor) |
| Demo default | no |
| Input | N no-op tasks, payload ≤1 KiB |
| Output | 0 tokens |
| Oracle | task reaches terminal success |
| Notes | Pair with SPEC-090 claim latency experiments |

```text
  enqueue N --> workers claim --> instant complete
  measure: claim latency, pending depth, sojourn
```

---

## `ingest-short-v1`

| | |
|--|--|
| Intent | Small text ingest; extract + embed path |
| Lit layers | L1, L3, L4, L5, L6 |
| Modes | system + provider (provider **not** on shared demo by default) |
| Demo default | no |
| Input | ~2–4 KiB plain text ≈ **500–1_000** size_gate tokens · 1 doc |
| Output band | extract `llm_out` driven by SPEC-117 caps (default 40 entities / 100 records) |
| Embed | `embed_est` from unique chunk/entity/rel texts |
| Controlled | `EDGEQUAKE_MAX_EXTRACTION_*`, gleaning passes, concurrent extractions |
| Oracle | document status completed; cost_breakdown present |

---

## `ingest-dense-v1`

| | |
|--|--|
| Intent | Dense page-like text — stress extract+embed+merge |
| Lit layers | L1, L3, L4, L5, L6 |
| Modes | system + provider (dedicated tenant) |
| Demo default | no |
| Input | ~8–16 KiB ≈ **2_000–4_000** size_gate tokens · multi-chunk |
| Output | higher `llm_in` (longer prompts) · `llm_out` still capped by SPEC-117 |
| Oracle | completed; no pool acquire storm in USE |

```text
  dense text --> chunk --> extract (llm_in/out) --> embed_est --> merge/DB
```

---

## `embed-batch-v1`

| | |
|--|--|
| Intent | Embedding path without extract LLM noise |
| Lit layers | L5 (+ L6 if persist) |
| Modes | system (fake vectors) + provider embed |
| Demo default | no (unless dedicated) |
| Input | Fixed list of M strings with known char lengths |
| Output | `embed_est = Σ ceil(chars_i / 2.5)`; `embed_usage` when available |
| Controlled | `EDGEQUAKE_EMBEDDING_BATCH_SIZE` |
| Oracle | M vectors returned with expected dimension (demo: **1536**) |

---

## `query-chat-v1`  ← **demo default**

| | |
|--|--|
| Intent | Interactive RAG query on existing corpus |
| Lit layers | L0, L7, L8 (keyword LLM arm inside L7/L8 timing split) |
| Modes | **provider on demo**; system with mock retrieve+generate |
| Demo default | **yes** |
| Target | `https://demo.edgequake.com` · tenant `…0002` · workspace `…0003` |
| Input | Fixed prompt set P (versioned list in measurements); query mode **mix** |
| Context | `context_tokens` as returned (do not truncate differently across C) |
| Output band | stream answer; record `generated_tokens`, `ttft_ms` |
| Cache | cold default; warm optional second card |
| Oracle | 2xx + parseable answer/stats; SPEC-001 Acc **not** required |

```text
  POST /api/v1/query|chat  (auth + tenant + workspace)
        |
        v
  embed query --> keyword LLM? --> retrieve --> prompt --> stream answer
  L7 times: embedding_time_ms, keyword_time_ms, retrieval_time_ms
  L8 times: ttft_ms, generation_time_ms, generated_tokens
```

**Seed corpus (observed 2026-09-29 via MCP `eq_document_list` on pinned workspace):**

| Document id (prefix) | Title / file |
|----------------------|--------------|
| `01a0ea5f-…` | `agentic_2609.31562v1.pdf` |
| `01a0dd9d-…` | `If_You_Think_You_Can_Do_Real-World_Text-to-SQL_…pdf` |
| `01a0bd0b-…` | `ten_2609.18461v1.pdf` |
| `01a0b824-…` | `sol_pi_2609.20519v1.pdf` |

Prompt set must reference this corpus (or document_ids scope) so retrieval is non-empty.
Empty-hit runs are a separate shape (`query-empty-v1`) if needed later.

---

## `query-long-context-v1`

| | |
|--|--|
| Intent | Stress L8 prefill (`llm_in` / `context_tokens`) |
| Lit layers | L0, L7, L8 |
| Modes | provider + system |
| Demo default | optional (cap C tightly) |
| Input | Same as query-chat but force higher top_k / deeper fetch so context grows |
| Output | similar `llm_out` band; higher TTFT expected |
| Oracle | success; record context_tokens distribution |

---

## Extract caps as controlled variable

SPEC-117 defaults: `EDGEQUAKE_MAX_EXTRACTION_ENTITIES=40`,
`EDGEQUAKE_MAX_EXTRACTION_RECORDS=100`, selection `relation_aware` (product) vs `fifo` (Acc).

```text
  same ingest-dense-v1 text
       |
       +-- caps 40/100  --> llm_out band A
       +-- caps raised  --> llm_out band B  (NEW shape id required)
```

Never compare knees across caps without renaming the shape.

---

## Shape × demo matrix

| Shape | System local | Provider demo |
|-------|--------------|---------------|
| queue-only-v1 | yes | no |
| ingest-short-v1 | yes | dedicated tenant only |
| ingest-dense-v1 | yes | dedicated tenant only |
| embed-batch-v1 | yes | dedicated / off-window |
| **query-chat-v1** | yes | **default** |
| query-long-context-v1 | yes | optional, low C |
