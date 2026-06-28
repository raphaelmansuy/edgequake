# 001 — Algorithm Comparison (Code Law)

**Cross-ref:** [001 First Principles](../001-first-principles/001-first-principles.md) · [004 Query](../004-query/001-query-comparison.md) · [008 LightRAG Expert](../008-expert-lenses/002-lightrag-expert.md)

**Findings:** C-01, C-02, C-06, C-10, C-11

---

## 1. Entity Extraction

### LightRAG

```text
  chunk ──> extract_entities() ──> gleaning loop (max_gleaning)
                │
                └── merge_nodes_and_edges()
                        ├── entity merge + LLM summary
                        ├── relationship merge
                        └── upsert graph + 3 VDBs
```

**Source:** `operate.py::extract_entities` (L3320), `merge_nodes_and_edges` (L2914), `constants.py::DEFAULT_MAX_GLEANING`.

Features:
- Entity name truncation (char + byte limits for Milvus)
- Section context breadcrumbs in prompts
- Source ID limits (FIFO/keep)
- Keyed graph locks per entity

### EdgeQuake

```text
  chunk ──> LLMExtractor ──> optional GleaningExtractor
                │
                └── KnowledgeGraphMerger::merge()
                        ├── entity dedup + summary
                        ├── relationship merge
                        └── AGE upsert + entity/rel vectors
```

**Source:** `edgequake-pipeline/src/extractor/`, `edgequake-pipeline/src/merger/`, `build_ingestion_pipeline()` in `ingestion_pipeline.rs`.

| Algorithm step | LightRAG | EdgeQuake | Match |
|----------------|----------|-----------|:-----:|
| Per-chunk LLM extract | ✓ | ✓ | ✓ |
| Gleaning multi-pass | ✓ configurable | ✓ metadata-driven | ✓ |
| Entity normalization | UPPERCASE | UPPERCASE_UNDERSCORE | ✓ |
| Merge with LLM summary | ✓ | ✓ | ✓ |
| Source provenance tracking | ✓ source_ids | ✓ source_chunk_ids | ✓ |
| Section heading context | ✓ breadcrumb | △ limited | △ |

**Verdict:** Core extraction parity **A**. EdgeQuake lacks LightRAG's heading breadcrumb injection and multimodal markup stripping.

---

## 2. Chunking

### LightRAG — Four Strategies

| Code | Strategy | File |
|------|----------|------|
| F | Fixed token size | `chunker/token_size.py` |
| R | Recursive character | `chunker/recursive_character.py` |
| V | Semantic vector | `chunker/semantic_vector.py` |
| P | Paragraph semantic | `chunker/paragraph_semantic.py` |

Routing via `process_options.chunking` selector (`pipeline.py::_CHUNKING_METHOD_LABELS`).

### EdgeQuake — Adaptive Fixed

```rust
// adaptive_chunking.rs
// <50KB → 1200 tokens, 50-150KB → 800, >150KB → 600
calculate_adaptive_chunk_size(document_size_bytes)
```

**Gap (C-03):** EdgeQuake has **one strategy** (size-adaptive fixed). LightRAG supports semantic and paragraph-aware chunking that can materially change graph quality on structured documents.

---

## 3. Query Modes — Side by Side

### Mode Enum Parity

| Mode | LightRAG `QueryParam.mode` | EdgeQuake `QueryMode` | Notes |
|------|---------------------------|----------------------|-------|
| local | ✓ default in local path | ✓ | Entity ANN |
| global | ✓ | ✓ | Relationship ANN |
| hybrid | ✓ | ✓ | Round-robin interleave |
| mix | ✓ **default** | ✓ **default** | + naive vector arm |
| naive | ✓ | ✓ | Chunk ANN only |
| bypass | ✓ | ✓ | Direct LLM |

**Source:** `lightrag/base.py::QueryParam` L86 (`mode: ... = "mix"`), `edgequake-query/src/modes.rs`.

### Keyword Extraction

Both:
1. LLM extracts high-level + low-level keywords from query
2. Validate keywords against graph vocabulary
3. Fallback: short query → use raw query as low-level keyword

**Source:** LightRAG `operate.py::get_keywords_from_query`; EdgeQuake `keywords/` module + `query_pipeline.rs::pipeline_prepare`.

### Context Construction

```text
  LightRAG _build_query_context          EdgeQuake mode dispatch
  ─────────────────────────────          ────────────────────────

  LOCAL:  _get_node_data                  modes/local.rs
          └─ entity VDB search            └─ entity ANN + graph BFS
          └─ 1-hop edges                  └─ graph_depth (default 2)
          └─ provenance chunks            └─ chunk_retrieval (ID allowlist)

  GLOBAL: _get_edge_data                  modes/global.rs
          └─ rel VDB search               └─ rel ANN
          └─ entity co-occurrence         └─ community_id expand (EQ only)
          └─ provenance chunks            └─ chunk_retrieval

  MIX:    hybrid + _get_vector_context    modes/mix.rs
          └─ round-robin entities/rels    └─ 3 arms parallel
          └─ + chunk VDB direct           └─ RRF fusion (EQ default)
```

**C-10:** Both default to Mix. LightRAG Mix uses **round-robin** entity/rel merge + vector chunks. EdgeQuake Mix uses **RRF score fusion** across three arms — better ranking, different ordering on ties.

**C-11:** EdgeQuake adds `community_id` co-membership expansion in Global mode via Louvain index at ingest. LightRAG has **no equivalent** community index in query path.

---

## 4. Hybrid Merge Algorithm

### LightRAG (operate.py L4456-4510)

```text
  Round-robin merge entities:
    slot i: local_entities[i] → global_entities[i]
    dedupe by entity_name

  Round-robin merge relations:
    slot i: local_relations[i] → global_relations[i]
    dedupe by (src, tgt) pair

  Then: process_chunks_unified() for provenance + vector chunks
```

### EdgeQuake

```text
  hybrid_merge.rs:
    slot i: local[i] → global[i] → naive[i]
    dedupe chunk IDs, cap at max_chunks

  mix.rs:
    tokio::join!(local, global, naive)
    RRF fusion (default) or weighted min-max blend
```

| Merge style | LightRAG Hybrid | EdgeQuake Hybrid | EdgeQuake Mix |
|-------------|:---------------:|:----------------:|:-------------:|
| Round-robin | ✓ | ✓ | — |
| RRF | — | optional env | ✓ default |
| Naive arm | Mix only | Hybrid includes | ✓ |
| BM25 fusion | ✗ | ✓ all arms | ✓ |

**C-02:** EdgeQuake exceeds LightRAG on retrieval fusion. Not a parity violation — an extension.

---

## 5. What Neither Implements (C-06, C-07)

```text
  GraphRAG (Microsoft)              SOTA Agentic RAG (Jun 2026)
  ────────────────────              ────────────────────────────

  Community detection @ index       Query decomposition
  Community REPORT generation       CRAG confidence gates
  Map-reduce over summaries         Iterative retrieval loops
  Dynamic community selection       Self-RAG reflection
                                    Faithfulness verification

  LightRAG:  ✗ none                 LightRAG:  ✗ none
  EdgeQuake: △ Louvain ID only      EdgeQuake: △ intent routing only
```

---

## 6. Algorithm Parity Scorecard

| Component | Parity | EdgeQuake Delta |
|-----------|:------:|-----------------|
| Entity extraction | **A** | — |
| Gleaning | **A** | — |
| Graph merge | **A-** | Less source_id limit sophistication |
| Chunking | **C+** | 1 vs 4 strategies |
| Keyword query | **A** | + conversation context |
| Local retrieval | **A+** | + BM25, multi-hop |
| Global retrieval | **A** | + community_id (deviation) |
| Hybrid merge | **A** | round-robin match |
| Mix merge | **B+** | RRF ≠ round-robin |
| Naive retrieval | **A+** | + FTS |
| Reranking | **B+** | cross-encoder vs external service |

**Overall algorithm grade: A- (LightRAG-class with extensions and chunking gap)**
