# 001 — Improvement Plan (EdgeQuake vs LightRAG)

**Cross-ref:** [README](../README.md) · [SPEC-025 Plan](../../025-egdequake-audit/012-improvement-plan.md)

**Date:** 2026-06-27  
**Principle:** Close parity gaps where LightRAG is ahead; preserve extensions where EdgeQuake is ahead; measure everything.

---

## Strategic Position

```text
  DO NOT                          DO
  ──────                          ──

  Rewrite as LightRAG clone       Keep Postgres-native extensions
  Add 13 storage backends         Add parser registry (1 at a time)
  Claim GraphRAG                  Document category honestly
  Remove Mix/RRF default          Add LightRAG-compat mode flag
  Copy operate.py monolith        Port algorithms as traits/modules
```

---

## Priority Matrix

| ID | Item | vs LightRAG Gap | Sev | Effort | Phase |
|----|------|-----------------|:---:|:------:|:-----:|
| P-01 | Head-to-head eval harness | Neither has it | P1 | 2w | 1 |
| P-02 | Semantic/recursive chunking | LR 4 strategies | P1 | 3w | 2 |
| P-03 | Markdown IR ingestion | LR native parser | P1 | 2w | 2 |
| P-04 | Section heading context | LR breadcrumb | P2 | 1w | 2 |
| P-05 | LightRAG-compat Mix mode | RR vs RRF | P2 | 3d | 1 |
| P-06 | DOCX parser | LR native | P2 | 3w | 3 |
| P-07 | Multimodal VLM ingest | LR analyze stage | P2 | 4w | 4 |
| P-08 | LLM role separation | LR llm_roles | P2 | 1w | 2 |
| P-09 | Source ID limit policies | LR FIFO/keep | P3 | 1w | 3 |
| P-10 | RAGAS CI gate | EQ skeleton only | P1 | 1w | 1 |
| P-11 | Admission saga (KV after worker) | Both weak | P1 | 2w | 2 |
| P-12 | External queue (SQS/NATS) | LR in-proc | P2 | 3w | 4 |
| P-13 | GraphRAG track (community reports) | Neither | P3 | 8w | 5 |
| P-14 | Agentic retrieval (CRAG) | Neither | P3 | 4w | 5 |

---

## Phase 1 — Measure & Align (2 weeks)

**Goal:** Prove parity claims with data, not assertions.

### P-01 Head-to-Head Eval

```text
  fixtures/
  ├── shared_corpus/          (10 docs: 3 PDF, 3 MD, 4 text)
  ├── shared_queries.json     (50 from spec025 + 20 broad)
  └── expected_graph_stats.json

  scripts/compare_lightrag_edgequake.py
       │
       ├── ingest both (same LLM + embed config)
       ├── run 70 queries per mode
       └── output: recall, latency, cost, graph diff
```

**Gate:** EdgeQuake context_entity_recall ≥ LightRAG − 5% on shared corpus.

### P-05 LightRAG-Compat Mix Flag

```text
  EDGEQUAKE_MIX_FUSION=rrf|lightrag_round_robin

  When lightrag_round_robin:
    Mix uses same slot order as operate.py L4456-4510
    Enables byte-level comparison in P-01
```

### P-10 RAGAS CI Gate

Extend `eval/metrics.rs`:
- Wire keyword_recall ≥ 0.7 on golden set (smoke)
- Fail CI if regression > 5% vs baseline artifact

---

## Phase 2 — Ingestion Parity (4 weeks)

**Detailed plan:** [phase2/001-phase2-ingestion-parity-plan.md](./phase2/001-phase2-ingestion-parity-plan.md) · [E2E matrix](./phase2/002-e2e-test-matrix.md)

### P-02 Chunking Strategy Registry

Port LightRAG pattern:

```text
  edgequake-pipeline/src/chunker/
  ├── fixed.rs          (existing adaptive)
  ├── recursive.rs      (NEW — port chunker/recursive_character.py)
  └── registry.rs       (select by workspace metadata or Content-Type)

  API: POST /documents { "chunk_strategy": "recursive" | "fixed" }
```

**Do not port semantic_vector first** — requires embed-per-sentence, high cost.

### P-03 Markdown IR

Evaluate porting `lightrag/parser/markdown/` IR builder vs wrapping existing MD path.

Minimum: heading-aware splits + breadcrumb metadata on chunks.

### P-04 Section Context

Inject `---Section Context---` into extraction prompts when chunk metadata has heading path.

**Source reference:** `operate.py::_truncate_section_context`.

### P-08 LLM Role Separation

```text
  WorkspaceConfig:
    llm_extract_model
    llm_query_model
    llm_summary_model

  Mirror lightrag/llm_roles.py priority semantics.
```

### P-11 Admission Saga Fix

```text
  Current:  HTTP → KV write → enqueue → worker persist
  Target:   HTTP → staging KV → worker → promote on success
                         └── delete staging on failure
```

Eliminates N-12 (Failed docs with KV orphans).

---

## Phase 3 — Format Breadth (4 weeks)

### P-06 DOCX Native Parser

Options:
1. Port LightRAG `parser/docx/` to Rust (high fidelity, high effort)
2. Integrate `docx-rs` + custom IR mapping (medium effort)
3. Sidecar to LightRAG parser CLI (fast, ops burden)

**Recommendation:** Option 2 for v1; golden tests against LR fixtures.

### P-09 Source ID Limits

Port `apply_source_ids_limit` FIFO/keep from LightRAG for entities with 100+ source chunks.

---

## Phase 4 — Scale & Multimodal (6 weeks)

### P-07 Multimodal Ingest

```text
  Phase 4a: Image upload → VLM describe → text inject (simple)
  Phase 4b: PDF inline images (requires parser IR)

  Reference: lightrag/pipeline.py analyze stage
```

### P-12 External Task Queue

Move WorkerPool to optional NATS/SQS for horizontal worker scale.

Keep Postgres task table as SSOT; external queue as delivery mechanism.

---

## Phase 5 — Category Extensions (optional tracks)

### P-13 GraphRAG Track

Only if product requires broad thematic Q&A beyond LightRAG:

1. Community report generation (LLM summary per Louvain cluster)
2. Report vector index
3. Global mode: retrieve reports + map-reduce

**8+ weeks. Do not start until Phase 1 eval passes.**

### P-14 Agentic Retrieval

Minimal CRAG:

```text
  retrieve → confidence score → if low: rewrite query → re-retrieve once
```

No full agent loop. Cap at 2 retrieval passes.

---

## Preserve EdgeQuake Advantages (do NOT regress)

| Extension | Why keep |
|-----------|----------|
| BM25/FTS all arms | LightRAG lacks; measurable recall win |
| RRF Mix default | Better ranking; offer compat flag only |
| Cross-encoder rerank | Production quality |
| Intent routing | Cost control |
| Conversation history | Multi-turn UX |
| Saga compensation | Data integrity |
| Workspace tenancy | Enterprise requirement |
| `/health` components | Ops requirement |

---

## Success Criteria (SPEC-026 exit)

| Criterion | Target |
|-----------|--------|
| Algorithm parity tests | 100% contract pass |
| Head-to-head recall | ≥ LightRAG − 5% |
| Chunking strategies | ≥ 2 (fixed + recursive) |
| Format support | text + PDF + MD + DOCX |
| RAGAS smoke CI | green on 50 cases |
| Documented category | "LightRAG-class graph RAG" not GraphRAG |
| LightRAG-compat mode | documented API flag |

---

## Timeline Summary

```text
  Week  1-2   Phase 1  Measure + compat flag + RAGAS gate
  Week  3-6   Phase 2  Chunking + MD IR + LLM roles + saga
  Week  7-10  Phase 3  DOCX + source limits
  Week 11-16  Phase 4  Multimodal + external queue
  Week 17+    Phase 5  GraphRAG / agentic (product decision)
```

**Minimum viable parity:** Phase 1 + Phase 2 = **6 weeks**.

---

## What LightRAG Should Learn from EdgeQuake

Honest reverse recommendations for LightRAG upstream:

1. Add BM25/FTS fusion to query path
2. Make Postgres the documented golden production path
3. Add RAGAS eval skeleton
4. Split `operate.py` into extract/query/merge modules
5. Document Mix vs Hybrid cost profile

These are **contributions EdgeQuake can upstream** if maintaining LR fork alignment.
