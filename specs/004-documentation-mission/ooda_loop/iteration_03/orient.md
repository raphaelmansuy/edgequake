# OODA Iteration 03 - Orient

**Date**: 2025-01-XX
**Focus**: Understanding LightRAG Algorithm for Deep-Dive Article

## 🧭 Orientation Analysis

### 1. LightRAG Core Innovation

The LightRAG paper identifies three key limitations in traditional RAG systems:

```
┌─────────────────────────────────────────────────────────────────┐
│                    TRADITIONAL RAG PROBLEMS                       │
├─────────────────────────────────────────────────────────────────┤
│ 1. FLAT DATA REPRESENTATIONS                                     │
│    └─> Cannot understand entity relationships                    │
│                                                                   │
│ 2. INADEQUATE CONTEXTUAL AWARENESS                              │
│    └─> Fragmented answers missing inter-dependencies            │
│                                                                   │
│ 3. EXPENSIVE COMMUNITY TRAVERSAL (GraphRAG)                     │
│    └─> 610,000+ tokens per query for community reports          │
└─────────────────────────────────────────────────────────────────┘
```

### 2. EdgeQuake's Solution Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    EDGEQUAKE GRAPH-RAG PIPELINE                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌────────┐  │
│  │ Document │ ──> │ Chunking │ ──> │ Extract  │ ──> │ Build  │  │
│  │ Ingest   │     │ Adaptive │     │ Entities │     │ Graph  │  │
│  └──────────┘     └──────────┘     └──────────┘     └────────┘  │
│       │                │                │                │       │
│       │                │                │                │       │
│       v                v                v                v       │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    KNOWLEDGE GRAPH                           ││
│  │  ┌─────┐        ┌─────┐        ┌─────┐                      ││
│  │  │NODE │──edge──│NODE │──edge──│NODE │                      ││
│  │  └─────┘        └─────┘        └─────┘                      ││
│  │     │              │              │                          ││
│  │     └──────────────┴──────────────┘                          ││
│  │            + Vector Embeddings                               ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### 3. Dual-Level Retrieval Strategy

EdgeQuake implements LightRAG's dual-level approach with extensions:

```
                    USER QUERY
                        │
                        v
            ┌───────────────────────┐
            │  Keyword Extraction   │
            │  (Low + High Level)   │
            └───────────────────────┘
                        │
           ┌────────────┴────────────┐
           │                         │
           v                         v
    ┌─────────────┐          ┌─────────────┐
    │  LOW-LEVEL  │          │ HIGH-LEVEL  │
    │  RETRIEVAL  │          │  RETRIEVAL  │
    ├─────────────┤          ├─────────────┤
    │ • Entities  │          │ • Topics    │
    │ • Direct    │          │ • Themes    │
    │   relations │          │ • Summaries │
    │ • Neighbors │          │ • Communities│
    └─────────────┘          └─────────────┘
           │                         │
           └────────────┬────────────┘
                        │
                        v
            ┌───────────────────────┐
            │    CONTEXT FUSION     │
            │  (Balanced Truncation)│
            └───────────────────────┘
                        │
                        v
            ┌───────────────────────┐
            │   LLM ANSWER GEN      │
            └───────────────────────┘
```

### 4. Entity Extraction State Machine

```
                    ┌──────────────┐
                    │    START     │
                    └──────┬───────┘
                           │
                           v
              ┌────────────────────────┐
              │   BUILD SYSTEM PROMPT  │
              │   (Entity Types List)  │
              └────────────┬───────────┘
                           │
                           v
              ┌────────────────────────┐
              │   BUILD USER PROMPT    │
              │   (Chunk Content)      │
              └────────────┬───────────┘
                           │
                           v
              ┌────────────────────────┐
              │     LLM EXTRACTION     │
              │   (Tuple/JSON Format)  │
              └────────────┬───────────┘
                           │
               ┌───────────┴───────────┐
               │                       │
               v                       v
        ┌──────────────┐       ┌──────────────┐
        │ finish_reason│       │ finish_reason│
        │   = "stop"   │       │  = "length"  │
        │   (Success)  │       │ (Truncated)  │
        └──────┬───────┘       └──────┬───────┘
               │                       │
               v                       v
        ┌──────────────┐       ┌──────────────┐
        │ PARSE TUPLES │       │ RETRY WITH   │
        │ Line by Line │       │ 2x max_tokens│
        └──────┬───────┘       └──────┬───────┘
               │                       │
               v                       │
        ┌──────────────┐               │
        │  NORMALIZE   │ <─────────────┘
        │ Entity Names │
        └──────┬───────┘
               │
               v
        ┌──────────────┐
        │   GLEANING   │─────┐
        │  (Optional)  │     │ Iteration loop
        └──────┬───────┘ <───┘
               │
               v
        ┌──────────────┐
        │    RESULT    │
        └──────────────┘
```

### 5. Key Technical Decisions

| Decision               | Rationale                                         |
| ---------------------- | ------------------------------------------------- |
| Tuple format over JSON | Streaming-friendly, partial recovery, no escaping |
| Hybrid parser          | Backward compatibility + migration path           |
| Adaptive max_tokens    | Handle varying entity density                     |
| Entity normalization   | Enable proper node merging                        |
| 6 query modes          | Cover all use cases (LightRAG has 3)              |

### 6. What Makes EdgeQuake Different from LightRAG Python

```
┌────────────────────┬────────────────────┬────────────────────┐
│      Feature       │     LightRAG       │     EdgeQuake      │
├────────────────────┼────────────────────┼────────────────────┤
│ Language           │ Python             │ Rust (async)       │
│ Query Modes        │ 3 (local/global/   │ 6 (+naive, mix,    │
│                    │   hybrid)          │   bypass)          │
│ Error Recovery     │ Basic              │ Adaptive retry     │
│ Token Management   │ Fixed limits       │ Progressive scaling│
│ Multi-tenant       │ No                 │ Yes (workspace_id) │
│ Streaming          │ Limited            │ Full SSE support   │
│ Storage            │ Neo4j              │ PG/AGE + pgvector  │
└────────────────────┴────────────────────┴────────────────────┘
```

### 7. Article Structure Planning

Based on observations, the deep-dive article should cover:

1. **First Principles**: Why graphs improve RAG
2. **Algorithm Walkthrough**: Step-by-step extraction process
3. **Tuple Format**: Why it's more robust than JSON
4. **Normalization**: Preventing graph fragmentation
5. **Dual-Level Retrieval**: Low vs High level explained
6. **Gleaning**: Multi-pass extraction strategy
7. **Query Modes**: When to use which mode
8. **Comparisons**: vs GraphRAG, NaiveRAG

## 🎯 Documentation Priority

**HIGH PRIORITY**:

1. LightRAG algorithm deep-dive article
2. Query mode selection guide

**MEDIUM PRIORITY**:

1. Entity normalization technical note
2. Tuple vs JSON format comparison

**LOW PRIORITY** (later iterations):

1. GraphRAG comparison detailed analysis
2. Performance benchmarks reproduction
