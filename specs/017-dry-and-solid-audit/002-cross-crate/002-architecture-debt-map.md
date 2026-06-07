# Architecture Debt Map

**Spec:** 017-dry-and-solid-audit  
**View:** Runtime call graph with duplication hotspots (code-derived).

---

## Current Production Flow (Simplified)

```text
                    ┌─────────────────────────────────────────┐
                    │           edgequake-api                 │
                    │  Handlers (32k LOC)                     │
                    │  ├─ /query, /chat  → sota_engine ★      │
                    │  ├─ /ollama/*      → query_engine (legacy) │
                    │  ├─ uploads        → create_workspace_pipeline │
                    │  └─ task processor → get_workspace_pipeline_strict │
                    └──────┬──────────────┬──────────────┬────────────┘
                           │              │              │
              ┌────────────┘              │              └──────────────┐
              ▼                           ▼                             ▼
    edgequake-query              edgequake-pipeline              edgequake-tasks
    SOTAQueryEngine (~4.3k)      Pipeline + Chunker              WorkerPool
    QueryEngine (legacy)         LLMExtractor / SOTAExtractor    DocumentTaskProcessor
    strategies/ (DEAD)           Merger (wrong normalizer!)           │
              │                           │                             │
              │                           ▼                             │
              │                  edgequake-storage ◄────────────────────┘
              │                  KV / Vector / Graph
              ▼
    edgequake-core/query/*  ◄── EdgeQuake orchestrator STILL uses this
    (~1,145 LOC, 6 modes)         (NOT edgequake-query SOTA)
```

---

## Duplication Hotspots (by LOC at risk)

| Hotspot | Estimated duplicate LOC | Crates involved |
|---------|-------------------------|-----------------|
| Query execution (3 stacks) | ~2,500+ | core, query, api |
| SOTA entry pipeline (3 files) | ~1,900 | query |
| Workspace pipeline resolution | ~300 | api |
| Memory/postgres bootstrap | ~150 | api |
| JSON extraction + prompts | ~400 | pipeline |
| MetadataFilter + tenant checks | ~200 | storage, query, core |
| Task enqueue boilerplate | ~100 | api |

**Total addressable duplication:** ~5,500+ LOC (conservative).

---

## Dependency Boundary Violations

| From | To | Issue |
|------|-----|-------|
| `edgequake-core` | (avoids) `edgequake-query` | Cycle fear → duplicated query engine in-core |
| `edgequake-core/types` | `edgequake-pdf` | `PdfParserBackend` enum pulls pdf dep into core types |
| `edgequake-api` | `edgequake-pdf2md` | Bypasses pdf facade for some paths |
| `edgequake-query/Cargo.toml` | `edgequake-core` | Declared but unused in src (dead dep) |

---

## SOLID Pressure Points

```text
┌─────────────────────────────────────────────────────────────┐
│ GOD OBJECTS (>3 unrelated concerns)                         │
├─────────────────────────────────────────────────────────────┤
│ AppState              api/state/mod.rs        25+ fields    │
│ EdgeQuake             core/orchestrator       8 subsystems  │
│ SOTAQueryEngine       query/sota_engine/      9 files       │
│ PostgresAGEGraphStorage storage/postgres/graph ~1789 LOC    │
│ PgVectorStorage       storage/postgres/vector ~1281 LOC     │
│ pipeline/helpers.rs   pipeline/               ~1003 LOC     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ FAT TRAITS (ISP)                                            │
├─────────────────────────────────────────────────────────────┤
│ GraphStorage          ~40+ methods                          │
│ ConversationService   15+ methods                           │
│ WorkspaceService      tenant + workspace + metrics + quota  │
└─────────────────────────────────────────────────────────────┘
```

---

## Target Architecture (Post-Remediation)

```text
edgequake-api (HTTP only)
  ├── AppState { storage, providers, tasks }  // decomposed
  ├── WorkspacePipelineFactory                // single module
  └── QueryHandler → QueryExecutionService    // shared query/chat

edgequake-core (types + orchestration)
  ├── Canonical QueryMode, StorageConfig names
  └── EdgeQuake → SOTAQueryEngine (via edgequake-query)

edgequake-query (single query engine)
  ├── QueryPipeline (one run() for basic/stream/workspace)
  └── strategies/ wired OR deleted

edgequake-pipeline
  ├── prompts::normalize_entity_name (ONLY)
  └── Chunker::chunk() → strategy

edgequake-storage
  ├── GraphAnalytics trait (split from GraphStorage)
  └── Memory adapters with parity overrides
```

---

## Crate Existence Verdict

| Crate | Verdict | Rationale |
|-------|---------|-----------|
| edgequake-api | **Keep** | HTTP boundary is correct |
| edgequake-core | **Keep, shrink** | Remove legacy `query/` after migration |
| edgequake-query | **Keep** | Production query path; needs internal dedup |
| edgequake-pipeline | **Keep** | Substantial domain logic |
| edgequake-storage | **Keep** | Trait abstraction justified |
| edgequake-auth | **Keep** | Security boundary (~3k LOC) |
| edgequake-tasks | **Keep** | Queue/worker domain (~6k LOC) |
| edgequake-rate-limiter | **Keep** | Testable middleware unit |
| edgequake-audit | **Merge candidate** | ~578 LOC, single consumer |
| edgequake-pdf | **Borderline** | ~306 LOC; keep if multi-backend roadmap |
