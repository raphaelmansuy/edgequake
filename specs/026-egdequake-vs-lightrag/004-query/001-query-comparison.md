# 001 — Query Pipeline Comparison

**Cross-ref:** [002 Algorithms](../002-algorithms/001-algorithm-comparison.md) · [007 Evaluations](../007-evaluations/001-evaluation-comparison.md)

**Findings:** C-02, C-07, C-10

---

## 1. Pipeline SSOT

### LightRAG Entry Points

```text
  LightRAG.aquery(query, param)
       │
       ├── mode == "naive"  ──> naive_query()
       ├── mode == "bypass" ──> direct LLM
       └── else             ──> kg_query()
                                    │
                                    └── _build_query_context()
```

**Source:** `lightrag/lightrag.py`, `operate.py::kg_query` (L3786), `naive_query` (L5740).

### EdgeQuake Entry Points

```text
  POST /query  →  QueryEngine::run_query_pipeline()
       │
       ├── pipeline_prepare   (keywords ∥ embed ∥ intent)
       ├── pipeline_retrieve  (mode dispatch)
       └── pipeline_finalize  (rerank → truncate → LLM)
```

**Source:** `query_pipeline.rs`, `engine_impl/modes/*`, `engine_impl/prompt.rs`.

---

## 2. Prepare Phase

| Step | LightRAG | EdgeQuake |
|------|:--------:|:---------:|
| LLM keyword extract | ✓ | ✓ |
| High/low level split | ✓ | ✓ `QueryEmbeddings` |
| Query embedding | ✓ batch optional | ✓ parallel |
| Conversation history | ✗ | ✓ `conversation_context.rs` |
| Intent-based mode routing | ✗ | ✓ `intent.rs` (adaptive) |
| Keyword graph validation | ✓ | ✓ |

**EdgeQuake advantage:** Multi-turn context wired into keyword extraction and prompts (SPEC-025 5.1). LightRAG treats each query independently.

---

## 3. Retrieve Phase — Mode Matrix

```text
                    NAIVE     LOCAL      GLOBAL     HYBRID      MIX
                    ─────     ─────      ──────     ──────      ───
  LightRAG          chunk     entity     rel        RR local    RR + chunk
                    VDB       VDB+graph  VDB+graph  + global    VDB

  EdgeQuake         chunk     entity     rel        3-arm RR    3-arm RRF
                    ANN+FTS   ANN+FTS    ANN+FTS    parallel    parallel
                              +BFS       +community
                              depth=2    expand
```

| Extension | LightRAG | EdgeQuake |
|-----------|:--------:|:---------:|
| BM25 / FTS | ✗ | ✓ all arms |
| Graph multi-hop | 1-hop default | configurable `graph_depth` |
| Cross-encoder rerank | external service | `EDGEQUAKE_RERANKER` |
| RRF fusion | ✗ | ✓ Mix default |
| Community co-expand | ✗ | ✓ Louvain IDs |
| Document filter | △ | ✓ `document_ids` |
| Workspace scoping | ✓ namespace | ✓ tenant+workspace |

---

## 4. Finalize Phase

### LightRAG

```text
  truncate_list_by_token_size(entities, relations, chunks)
       │
       └── LLM generate (stream or batch)
       └── optional rerank via rerank.py (external HTTP)
```

Token limits: `DEFAULT_MAX_ENTITY_TOKENS`, `DEFAULT_MAX_RELATION_TOKENS`, `DEFAULT_MAX_TOTAL_TOKENS` in `constants.py`.

### EdgeQuake

```text
  filter_context_by_document_ids
       │
       ├── cross_encoder rerank (optional)
       ├── balance_context (10K / 10K / remainder)
       └── LLM generate (stream supported)
```

**Source:** `engine_impl/prompt.rs`, `bootstrap.rs::create_production_reranker`.

---

## 5. Cost Profile (C-07 adjacent)

```text
  Per query (default Mix, both systems)
  ───────────────────────────────────

  LightRAG Mix:
    1× keyword LLM
    1× entity VDB + graph + chunk hydrate
    1× rel VDB + graph + chunk hydrate
    1× chunk VDB
    1× answer LLM
    ≈ 2 LLM calls + 3 VDB searches

  EdgeQuake Mix (no adaptive routing):
    1× keyword LLM
    3× parallel arms (each: VDB + optional BM25 + graph)
    1× rerank (optional)
    1× answer LLM
    ≈ 2 LLM calls + 3+ VDB + 3 BM25 pools

  EdgeQuake with intent routing (SPEC-025 6.4):
    Exploratory → Naive only (1 arm)
    Comparative → Local only
    Procedural → full Mix
```

EdgeQuake added **intent routing** to mitigate cost. LightRAG has **no equivalent** — always runs full mode path.

---

## 6. Streaming

| Feature | LightRAG | EdgeQuake |
|---------|:--------:|:---------:|
| Stream LLM response | ✓ | ✓ |
| Stream retrieval progress | △ | △ |
| `context_only` | ✓ | ✓ |
| `prompt_only` | ✓ | ✓ |
| Query result cache | ✓ hashing_kv | ✓ workspace-scoped |

---

## 7. Query Pipeline Grades

| Dimension | LightRAG | EdgeQuake |
|-----------|:--------:|:---------:|
| Mode fidelity | **A** (reference) | **A** |
| Retrieval richness | **B+** | **A** |
| Multi-turn | **D** | **B+** |
| Cost awareness | **C** | **B** (intent routing) |
| Agentic / iterative | **F** | **F** |
| Code clarity | **C** (monolith) | **A-** (modular) |

**Winner on query stack: EdgeQuake** — extensions justify deviation from pure round-robin Mix.

**Winner on simplicity: LightRAG** — fewer moving parts, predictable cost per mode.
