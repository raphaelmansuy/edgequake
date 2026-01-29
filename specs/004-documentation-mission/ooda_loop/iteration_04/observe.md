# OODA Iteration 04 - Observe

**Date**: 2025-01-XX
**Focus**: Core Concepts Documentation

## 📊 Observations

### 1. Missing Concepts Documentation

From `docs/README.md`, the following concept files are referenced but don't exist:

- `docs/concepts/graph-rag.md`
- `docs/concepts/entity-extraction.md`
- `docs/concepts/knowledge-graph.md`
- `docs/concepts/hybrid-retrieval.md`

### 2. Concept Dependencies

The concepts build on each other:

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONCEPT DEPENDENCY GRAPH                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│                    ┌──────────────┐                              │
│                    │  GRAPH-RAG   │ ← Foundation concept         │
│                    │  (Why?)      │                              │
│                    └──────┬───────┘                              │
│                           │                                       │
│           ┌───────────────┼───────────────┐                      │
│           │               │               │                       │
│           v               v               v                       │
│    ┌──────────────┐ ┌──────────────┐ ┌──────────────┐            │
│    │   ENTITY     │ │  KNOWLEDGE   │ │   HYBRID     │            │
│    │  EXTRACTION  │ │    GRAPH     │ │  RETRIEVAL   │            │
│    │   (How?)     │ │  (Where?)    │ │   (Query?)   │            │
│    └──────────────┘ └──────────────┘ └──────────────┘            │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### 3. Key Code Locations for Each Concept

| Concept           | Primary Code Location                   |
| ----------------- | --------------------------------------- |
| Graph-RAG         | `edgequake-core/src/orchestrator.rs`    |
| Entity Extraction | `edgequake-pipeline/src/extractor.rs`   |
| Knowledge Graph   | `edgequake-storage/src/traits/graph.rs` |
| Hybrid Retrieval  | `edgequake-query/src/engine.rs`         |

### 4. Existing Deep-Dive Coverage

The LightRAG Algorithm deep-dive already covers:

- Entity extraction (briefly)
- Dual-level retrieval
- Query modes

The concepts docs should be **shorter, simpler** - linking to deep-dives for details.

### 5. Target Audience Analysis

| Audience              | Concept Level           | Time   |
| --------------------- | ----------------------- | ------ |
| New developers        | Graph-RAG intro         | 5 min  |
| Curious engineers     | Entity extraction       | 10 min |
| System architects     | Knowledge graph storage | 15 min |
| Integration engineers | Hybrid retrieval        | 10 min |

## 📁 Files to Create

1. `docs/concepts/graph-rag.md` - Foundation concept (~150 lines)
2. `docs/concepts/entity-extraction.md` - Pipeline concept (~200 lines)
3. `docs/concepts/knowledge-graph.md` - Storage concept (~180 lines)
4. `docs/concepts/hybrid-retrieval.md` - Query concept (~160 lines)

## 🔍 Research Needed

- None - all information available in codebase and LightRAG paper already fetched
