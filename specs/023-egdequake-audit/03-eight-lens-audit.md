# 03 — Eight-Lens Audit Matrix

> **Spec**: 023-egdequake-audit  
> **Method**: Each lens evaluates ingestion + query independently, then cross-references shared findings.

---

## Cross-reference matrix

| Finding | AI Eng | LightRAG | GraphRAG | SOTA 2026 | Sys Eng | O(n) | Rust/DRY | PG/AGE |
|---------|--------|----------|----------|-----------|---------|------|----------|--------|
| RC-023-1 injection bypass | ● | ● | | ● | ●● | ●● | ●● | ● |
| RC-023-2 global mislabel | | ●● | ●● | ● | | | | |
| RC-023-3 no eval harness | ●● | ● | ● | ●● | ● | | ● | |
| RC-023-4 BM25 not cross-enc | ●● | ● | | ●● | ● | ● | | |
| RC-023-5 mix ≠ RRF | ● | ● | | ●● | | | ● | |
| RC-023-6 communities orphaned | | ●● | ●● | ●● | ● | ●● | ● | ● |
| RC-023-7 AGE batch inline | | | | | ● | | | ●● |
| RC-023-8 vector metadata bloat | | | | | ●● | ● | | ●● |

Legend: ●● = primary owner lens, ● = secondary impact

---

## Lens 1: AI Engineering

**Question**: Is the ML pipeline (prompt → extract → retrieve → generate) engineered for measurable quality?

### Ingestion

| Strength | Weakness |
|----------|----------|
| Dual prompt formats (tuple + JSON) with recovery | No A/B prompt versioning in production |
| Gleaning pass for recall | Gleaning doubles LLM cost per chunk |
| Configurable entity schema (strict/permissive) | No confidence scores on extractions |
| Resilient partial-failure processing | Failed chunks silently reduce graph coverage |

### Query

| Strength | Weakness |
|----------|----------|
| Dual-level embeddings (low/high keywords) | Keyword LLM adds latency + failure mode |
| BM25 rerank with min-score fallback | No cross-encoder (RC-023-4) |
| Six explicit modes for experimentation | No automatic mode routing by query intent |
| `context_only` for eval/debug | No built-in RAGAS loop (RC-023-3) |

**Grade: B**

**First principle**: You cannot improve what you do not measure. Contract tests prove **correctness**; they do not prove **recall@k**.

---

## Lens 2: LightRAG Expert

**Question**: Does EdgeQuake faithfully implement LightRAG's dual-level retrieval + incremental merge?

### Fidelity checklist

| LightRAG concept | EdgeQuake | Verdict |
|------------------|-----------|---------|
| Entity/rel extraction per chunk | ✅ LLMExtractor + prompts | Match |
| Entity normalization | ✅ EntityId | Match |
| Graph merge on re-ingest | ✅ KnowledgeGraphMerger | Match |
| Local retrieval (entity → chunks) | ✅ query_local | Match |
| Global retrieval (high-level) | ⚠️ rel-vector search, not community | Partial |
| Naive chunk search | ✅ query_naive | Match |
| Hybrid combination | ✅ round-robin 3-way | Match + extension |
| Incremental update | ✅ merge, no full rebuild | Match |
| Mix weighted mode | ✅ EdgeQuake extension (FEAT0105) | Superset |

**Grade: A−**

**Brutal note**: EdgeQuake is a **credible LightRAG fork**, not a pixel-perfect port. Global mode behavior differs when relationship vectors are sparse — degree fallback is an EdgeQuake invention, not LightRAG canon.

---

## Lens 3: GraphRAG Expert

**Question**: Does EdgeQuake deliver GraphRAG's core value — hierarchical community summaries for global reasoning?

### Reality check

```
Microsoft GraphRAG index-time:
  entities + relations → community detection → LLM community reports

EdgeQuake index-time:
  entities + relations → flat AGE graph (+ optional admin community API)

EdgeQuake query-time global:
  relationship vector ANN → entity batch fetch → chunks
```

| GraphRAG feature | EdgeQuake |
|------------------|-----------|
| Leiden community detection | ✅ code in `community.rs` |
| Community summary reports | ❌ not generated |
| Global search over summaries | ❌ |
| Local search over entities | ✅ |
| Map-reduce over communities | ❌ |
| Incremental graph update | ✅ (LightRAG-style merge) |

**Grade: C+**

**Honest positioning**: EdgeQuake is **LightRAG-class**, not **GraphRAG-class**. Calling global mode "community-based" in `modes.rs` is **technically false** (RC-023-2).

---

## Lens 4: SOTA RAG Expert (June 2026)

**Reference baseline** (industry consensus, Jun 2026):

1. Hybrid retrieval: dense + sparse (BM25) + structured/graph layer
2. Fusion: RRF or learned reranker
3. Cross-encoder rerank on top-50
4. Eval-gated deployment: RAGAS / DeepEval thresholds
5. GraphRAG for cross-doc synthesis only when benchmarks justify cost
6. LightRAG-style graphs for incremental entity-rich corpora

### EdgeQuake vs SOTA

| SOTA layer | EdgeQuake | Gap |
|------------|-----------|-----|
| Dense retrieval | ✅ pgvector HNSW | — |
| Sparse retrieval | ⚠️ BM25 rerank only, not retrieval arm | No BM25 index at retrieve stage |
| Graph retrieval | ✅ local/global arms | Flat, no communities |
| Fusion | ⚠️ round-robin / weighted scores | Not RRF |
| Neural rerank | ❌ | RC-023-4 |
| Eval CI | ❌ | RC-023-3 |
| Agentic multi-step | ❌ | out of scope |
| Metadata filtering | ✅ SQL pushdown | Strong |
| Tenancy | ✅ tenant + workspace | Strong |

**Grade: B−**

**What EdgeQuake does better than average**: Workspace isolation, saga ingestion (canonical path), contract-test culture, mix mode tunability.

**What average 2026 production beats you on**: Cross-encoder rerank + eval gates + sparse retrieval index.

---

## Lens 5: System Engineer

**Question**: Will this survive partial failures, restarts, and multi-tenant load?

### Ingestion reliability

```
Canonical path saga:
  ┌─────────────┐     success      ┌─────────────┐
  │ chunk vecs  │ ───────────────► │ graph merge │
  └─────────────┘                  └─────────────┘
        │                                 │
        │ merge fail                      │
        ▼                                 ▼
  compensate: delete chunk vecs    quarantine log
```

| Concern | Status |
|---------|--------|
| Cross-store consistency | ✅ saga on canonical path |
| Injection path | ❌ RC-023-1 |
| Task queue retry | ✅ edgequake_tasks |
| Idempotent re-ingest | ✅ merger |
| Cache coherency | ✅ except injection |
| Observability | ⚠️ tracing spans exist; no unified retrieval trace export |

**Grade: B+**

---

## Lens 6: O(n) Expert

**Question**: Where are the hidden linear loops and N+1 round-trips?

### Ingestion hotspots

| Hotspot | Complexity | Fix priority |
|---------|------------|--------------|
| LLM extraction | O(C) parallel | scale via concurrency limits |
| LLM summarization on merge | O(E) sequential | disable or batch |
| Merger batch upsert | O(1) RTT | ✅ done |
| Injection per-chunk upsert | O(C) RTT | **I1** |
| Chunk content in vector JSON | O(storage) bloat | **I8** |

### Query hotspots

| Hotspot | Complexity | Status |
|---------|------------|--------|
| Global degree batch | O(1) | ✅ P-G3 |
| Local chunk ID collect | O(entities × chunks) | acceptable |
| BM25 rerank | O(k × \|q\|) | fine for k≤20 |
| Community detection (admin) | O(V + E) full graph load | guarded by ResourceGuard |

**Grade: B+**

---

## Lens 7: Full-Stack Rust / SOLID / DRY / First Principles

### SOLID scorecard

| Principle | Evidence | Grade |
|-----------|----------|-------|
| **S** Single responsibility | `IngestionPersister` trait isolates persistence | A |
| **O** Open/closed | Storage via traits (`GraphStorage`, `VectorStorage`) | A |
| **L** Liskov | Memory + Postgres adapters pass contract tests | A− |
| **I** Interface segregation | Separate read/write graph traits | A |
| **D** Dependency inversion | API depends on `IngestionPersister`, not AGE | A |

### DRY violations (remaining)

| Duplication | Location | Fix |
|-------------|----------|-----|
| Injection persist logic | `injection.rs:949-1028` | Route through `persist_ingestion_result` |
| Merger config construction | injection inline `MergerConfig::default()` | Use `IngestionPersistSettings` SSOT |

### First-principles Rust quality

| Signal | Assessment |
|--------|------------|
| Error types (`Result`, no unwrap in hot paths) | Strong |
| Async batching (`tokio::join!`) | Strong |
| Contract tests as law | Strong |
| Module size | Mostly good post-SPEC-017 |

**Grade: A−**

One file (`injection.rs` ~1000 LOC) breaks the otherwise clean architecture.

---

## Lens 8: Postgres / AGE / pgvector Expert

### pgvector

| Feature | Status |
|---------|--------|
| Version | 0.8.0 in Docker (P-H3) |
| Index | HNSW cosine, tunable m/ef |
| Filtered ANN | iterative_scan when ≥0.8 |
| Batch upsert | UNNEST single txn |
| Workspace tables | registry pattern |

### AGE

| Feature | Status |
|---------|--------|
| Version | PG16 / v1.6.0-rc0 |
| Hot read/delete Cypher | parameterized `$1::agtype` (P-H7) |
| Batch node upsert | inline escaped literals (AGE MERGE limit) |
| Batch read bypass | SQL UNNEST for bulk fetch |
| Graph per workspace | ✅ |

### Storage anti-patterns

| Issue | Impact |
|-------|--------|
| Full chunk text in vector metadata JSON | Duplicates KV store; index bloat (RC-023-8) |
| Embedding stored as text `[f32,...]` | Parse overhead; consider pgvector native type consistently |

**Grade: A−**

Postgres layer is among the strongest parts of the codebase.

---

## Composite lens summary

```
                    INGESTION          QUERY
AI Engineering         B                B
LightRAG              A−               A−
GraphRAG              C+               C+
SOTA 2026             B−               B−
System Engineer       B+               A−
O(n)                  B+               B+
Rust/SOLID/DRY        A−               A
Postgres/AGE          A−               A−
```

**Weakest link**: GraphRAG positioning + injection path + eval gap.  
**Strongest link**: Canonical persister + query batching + Postgres adapters.
