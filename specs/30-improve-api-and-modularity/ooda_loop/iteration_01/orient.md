# OODA Iteration 01 - Orient

**Date**: 2026-01-07
**Focus**: Root cause analysis and refactoring opportunities

## Analysis of Large Files

### 1. `sota_engine.rs` (2,004 lines) - HIGHEST PRIORITY

**Current responsibilities (violates SRP):**

- Query orchestration (main entry points)
- Keyword extraction coordination
- Mode selection logic
- 5 different mode implementations (Local, Global, Hybrid, Mix, Naive)
- Reranking logic
- Token budgeting/truncation
- LLM prompt generation
- Context building
- Source tracking extraction

**Natural extraction boundaries:**

```
┌─────────────────────────────────────────────────┐
│              SOTAQueryEngine                    │
│  ┌─────────────┐  ┌─────────────────────────┐   │
│  │ query()     │  │ query_stream()          │   │
│  │ get_context │  │ query_stream_with_ctx   │   │
│  └──────┬──────┘  └────────────┬────────────┘   │
│         │                      │                │
│  ┌──────▼──────────────────────▼──────────────┐ │
│  │          ModeRouter (NEW)                  │ │
│  │  - query_local()                           │ │
│  │  - query_global()                          │ │
│  │  - query_hybrid()                          │ │
│  │  - query_mix()                             │ │
│  │  - query_naive()                           │ │
│  └────────────────────────────────────────────┘ │
│                                                 │
│  ┌─────────────────┐  ┌─────────────────────┐   │
│  │ ContextBuilder  │  │ PromptBuilder       │   │
│  │ (NEW)           │  │ (NEW)               │   │
│  └─────────────────┘  └─────────────────────┘   │
│                                                 │
│  ┌─────────────────┐  ┌─────────────────────┐   │
│  │ ChunkReranker   │  │ KeywordValidator    │   │
│  │ (NEW)           │  │ (NEW)               │   │
│  └─────────────────┘  └─────────────────────┘   │
└─────────────────────────────────────────────────┘
```

**Recommended extractions:**

1. **`mode_router.rs`** (~400 lines) - Move query_local, query_global, query_hybrid, query_mix, query_naive
2. **`context_builder.rs`** (~200 lines) - Entity/relationship/chunk assembly logic
3. **`prompt_builder.rs`** (~150 lines) - LLM prompt construction
4. **`reranker.rs`** (~100 lines) - Chunk reranking logic

### 2. `postgres/graph.rs` (1,784 lines) - HIGH PRIORITY

**Current responsibilities:**

- Entity CRUD (upsert, get, delete)
- Relationship CRUD
- Batch operations
- Vector metadata management
- RLS (Row Level Security) filtering
- Source tracking
- Community storage

**Natural extraction boundaries:**

```
┌─────────────────────────────────────────────────┐
│         PostgresAGEGraphStorage                 │
│                                                 │
│  ┌─────────────────┐  ┌─────────────────────┐   │
│  │ EntityOps       │  │ RelationshipOps     │   │
│  │ (trait impl)    │  │ (trait impl)        │   │
│  └─────────────────┘  └─────────────────────┘   │
│                                                 │
│  ┌─────────────────┐  ┌─────────────────────┐   │
│  │ BatchOps        │  │ SourceTracking      │   │
│  │ (trait impl)    │  │ (trait impl)        │   │
│  └─────────────────┘  └─────────────────────┘   │
│                                                 │
│  ┌─────────────────────────────────────────┐    │
│  │ sql_builders.rs (NEW) - SQL generation  │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

**Recommended extractions:**

1. **`sql_builders.rs`** (~300 lines) - SQL query construction functions
2. **`batch_ops.rs`** (~200 lines) - Batch operation implementations

### 3. `strategies.rs` (820 lines) - MEDIUM PRIORITY

**Current state:** 5 strategies in one file

- `LocalStrategy`
- `GlobalStrategy`
- `HybridStrategy`
- `MixStrategy`
- `NaiveStrategy`

**Recommendation:** Split into individual strategy files:

```
strategies/
├── mod.rs          # Re-exports + create_strategy()
├── local.rs        # LocalStrategy
├── global.rs       # GlobalStrategy
├── hybrid.rs       # HybridStrategy
├── mix.rs          # MixStrategy
└── naive.rs        # NaiveStrategy
```

### 4. `orchestrator.rs` (1,137 lines) - LOWER PRIORITY

**Analysis:** Actually well-structured! The "WHY" comments explain each section clearly.
The length is justified by the many configuration options and pipeline setup.

**No immediate refactoring needed**, but could extract:

- Builder patterns for config
- Pipeline setup logic

## Dependency Analysis

```
edgequake-api
    └─── edgequake-core
              └─── edgequake-query      ← REFACTOR TARGET
              └─── edgequake-storage    ← REFACTOR TARGET
              └─── edgequake-pipeline
              └─── edgequake-llm
```

**Safe refactoring path:**

1. Start with `sota_engine.rs` (leaf crate, well-tested)
2. Then `postgres/graph.rs` (storage layer)
3. Finally `strategies.rs` (already modular pattern)

## Risk Assessment

| File              | Risk   | Test Coverage | Impact            |
| ----------------- | ------ | ------------- | ----------------- |
| sota_engine.rs    | Medium | High          | Core query path   |
| postgres/graph.rs | Medium | Medium        | Postgres only     |
| strategies.rs     | Low    | High          | Isolated patterns |

## Key Insight

The codebase follows good practices:

- Excellent "WHY" documentation in `orchestrator.rs`
- Good trait-based abstraction in storage layer
- Clear module boundaries between crates

The main issue is **file size**, not **architectural problems**.
Refactoring should focus on **extracting without changing behavior**.

## Next: Decide

→ Prioritize which extractions to do first
→ Define the exact module boundaries
→ Plan test strategy to ensure non-regression
