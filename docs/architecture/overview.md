# EdgeQuake Architecture Overview

> Understanding the system design through first principles

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              EdgeQuake System                                   │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────────┐│
│  │                            Client Layer                                     ││
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐              ││
│  │  │    WebUI        │  │   REST API      │  │   Rust SDK      │              ││
│  │  │   (Next.js)     │  │   (HTTP/JSON)   │  │   (Native)      │              ││
│  │  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘              ││
│  └───────────┼────────────────────┼────────────────────┼───────────────────────┘│
│              │                    │                    │                        │
│              └────────────────────┼────────────────────┘                        │
│                                   │                                             │
│  ┌────────────────────────────────▼────────────────────────────────────────────┐│
│  │                          API Layer (Axum)                                   ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         ││
│  │  │   Routes    │  │  Handlers   │  │  Middleware │  │   OpenAPI   │         ││
│  │  │             │  │             │  │  (Auth,Rate)│  │   (Docs)    │         ││
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘         ││
│  └────────────────────────────────┬────────────────────────────────────────────┘│
│                                   │                                             │
│  ┌────────────────────────────────▼────────────────────────────────────────────┐│
│  │                      Core Orchestration Layer                               ││
│  │                                                                             ││
│  │  ┌───────────────────────────────────────────────────────────────────────┐  ││
│  │  │                         EdgeQuake                                     │  ││
│  │  │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                │  ││
│  │  │  │  insert()   │    │   query()   │    │  delete()   │                │  ││
│  │  │  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘                │  ││
│  │  │         │                  │                  │                       │  ││
│  │  │         ▼                  ▼                  ▼                       │  ││
│  │  │  ┌──────────────────────────────────────────────────────────────────┐ │  ││
│  │  │  │              Processing Components                               │ │  ││
│  │  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐               │ │  ││
│  │  │  │  │  Pipeline   │  │ QueryEngine │  │   Tasks     │               │ │  ││
│  │  │  │  │ (ingest)    │  │ (6 modes)   │  │ (async)     │               │ │  ││
│  │  │  │  └─────────────┘  └─────────────┘  └─────────────┘               │ │  ││
│  │  │  └──────────────────────────────────────────────────────────────────┘ │  ││
│  │  └───────────────────────────────────────────────────────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────────────────────┘│
│                                   │                                             │
│         ┌─────────────────────────┼─────────────────────────┐                   │
│         │                         │                         │                   │
│         ▼                         ▼                         ▼                   │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐              │
│  │   LLM Layer     │    │  Storage Layer  │    │  PDF Processor  │              │
│  │                 │    │                 │    │                 │              │
│  │ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │              │
│  │ │  Providers  │ │    │ │   Traits    │ │    │ │  Extractor  │ │              │
│  │ │ ─────────── │ │    │ │ ─────────── │ │    │ │ ─────────── │ │              │
│  │ │ • OpenAI    │ │    │ │ • KV        │ │    │ │ • Text      │ │              │
│  │ │ • Ollama    │ │    │ │ • Vector    │ │    │ │ • Tables    │ │              │
│  │ │ • LM Studio │ │    │ │ • Graph     │ │    │ │ • Layout    │ │              │
│  │ │ • Mock      │ │    │ │             │ │    │ │             │ │              │
│  │ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │              │
│  └─────────────────┘    └────────┬────────┘    └─────────────────┘              │
│                                  │                                              │
│                    ┌─────────────┴─────────────┐                                │
│                    │                           │                                │
│                    ▼                           ▼                                │
│         ┌─────────────────────┐    ┌─────────────────────┐                      │
│         │ Memory (Dev/Test)   │    │ PostgreSQL (Prod)   │                      │
│         │                     │    │                     │                      │
│         │ • Fast, ephemeral   │    │ • pgvector (vectors)│                      │
│         │ • No persistence    │    │ • Apache AGE (graph)│                      │
│         └─────────────────────┘    └─────────────────────┘                      │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Design Principles

### Why Rust?

| Factor          | Python (LightRAG) | Rust (EdgeQuake) | Impact          |
| --------------- | ----------------- | ---------------- | --------------- |
| **Performance** | ~100 docs/min     | ~1000 docs/min   | 10x throughput  |
| **Memory**      | 2-4GB typical     | 200-400MB        | 10x efficiency  |
| **Concurrency** | GIL limited       | True async       | Better scaling  |
| **Type Safety** | Runtime errors    | Compile-time     | Fewer prod bugs |
| **Deployment**  | Python env + deps | Single binary    | Simpler ops     |

### Why 11 Crates?

**Single Responsibility Principle**:

```
┌──────────────┐  Each crate does ONE thing well
│  API        │◄─ HTTP handling
├──────────────┤
│  Core        │◄─ Orchestration
├──────────────┤
│  Pipeline    │◄─ Document processing
├──────────────┤
│  Query       │◄─ Search and retrieval
├──────────────┤
│  Storage     │◄─ Persistence abstraction
├──────────────┤
│  LLM         │◄─ AI provider abstraction
├──────────────┤
│  PDF         │◄─ Document extraction
├──────────────┤
│  Auth        │◄─ Authentication
├──────────────┤
│  Audit       │◄─ Compliance logging
├──────────────┤
│  Tasks       │◄─ Background processing
├──────────────┤
│  Rate Limiter│◄─ Throttling
└──────────────┘
```

**Benefits**:

1. **Compile-time boundary enforcement** — Can't accidentally use internal types
2. **Parallel compilation** — Each crate compiles independently
3. **Selective testing** — Run tests for one crate only
4. **Clear dependency graph** — Easy to understand data flow
5. **Swappable implementations** — Change storage without touching query

### Why Trait-Based Abstraction?

```rust
// The CORE never knows about concrete implementations
pub struct EdgeQuake {
    llm: Arc<dyn LLMProvider>,        // Could be OpenAI, Ollama, or Mock
    storage: Arc<dyn GraphStorage>,    // Could be Memory or PostgreSQL
}
```

**Advantages**:

- Production uses OpenAI, tests use Mock (zero code changes)
- Add new providers without modifying core
- Runtime provider switching (dev → prod)

---

## Crate Dependency Graph

```
                                   ┌────────────────┐
                                   │  edgequake-api │ ← HTTP Server
                                   │   (37,400 LOC) │
                                   └───────┬────────┘
                                           │
                                           ▼
                                   ┌────────────────┐
                                   │ edgequake-core │ ← Orchestration
                                   │   (15,500 LOC) │
                                   └───────┬────────┘
                                           │
                    ┌──────────────────────┼──────────────────────┐
                    │                      │                      │
                    ▼                      ▼                      ▼
          ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
          │edgequake-pipeline    │ edgequake-query │    │  edgequake-llm  │
          │   (10,500 LOC)  │    │   (11,900 LOC)  │    │   (8,500 LOC)   │
          └────────┬────────┘    └────────┬────────┘    └─────────────────┘
                   │                      │                      │
                   └──────────────────────┼──────────────────────┘
                                          │
                                          ▼
                                 ┌─────────────────┐
                                 │edgequake-storage│ ← Persistence
                                 │   (11,900 LOC)  │
                                 └─────────────────┘

  Specialized Crates (Optional):
  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
  │  edgequake-pdf  │  │  edgequake-auth │  │ edgequake-tasks │
  │   (26,000 LOC)  │  │   (2,900 LOC)   │  │   (3,400 LOC)   │
  └─────────────────┘  └─────────────────┘  └─────────────────┘
```

---

## The 11 Crates Explained

| Crate                      | Purpose                 | Key Types                                      | LOC    |
| -------------------------- | ----------------------- | ---------------------------------------------- | ------ |
| **edgequake-core**         | Central orchestration   | `EdgeQuake`, `EdgeQuakeConfig`, `InsertResult` | 15,500 |
| **edgequake-pipeline**     | Document processing     | `Pipeline`, `Chunker`, `LLMExtractor`          | 10,500 |
| **edgequake-query**        | Search and retrieval    | `QueryEngine`, `QueryMode`, `QueryContext`     | 11,900 |
| **edgequake-storage**      | Persistence abstraction | `KVStorage`, `VectorStorage`, `GraphStorage`   | 11,900 |
| **edgequake-llm**          | AI provider abstraction | `LLMProvider`, `EmbeddingProvider`             | 8,500  |
| **edgequake-api**          | HTTP REST API           | `Server`, `Router`, handlers                   | 37,400 |
| **edgequake-pdf**          | PDF extraction          | `PdfExtractor`, `TableExtractor`               | 26,000 |
| **edgequake-auth**         | Authentication          | `AuthMiddleware`, `JwtValidator`               | 2,900  |
| **edgequake-audit**        | Compliance logging      | `AuditLog`, `AuditEvent`                       | 580    |
| **edgequake-tasks**        | Background jobs         | `TaskRunner`, `Task`, `TaskStatus`             | 3,400  |
| **edgequake-rate-limiter** | Request throttling      | `RateLimiter`, `TenantQuota`                   | 1,000  |

**Total**: ~130,000 lines of Rust

---

## Key Architectural Patterns

### 1. Facade Pattern (EdgeQuake)

The `EdgeQuake` struct is a facade that coordinates all RAG operations:

```rust
// Simple interface hides complex internals
let eq = EdgeQuake::new(config)
    .with_providers(llm, embedder)
    .with_storage(kv, vector, graph)
    .initialize()
    .await?;

// User doesn't know about Pipeline, QueryEngine, etc.
let result = eq.insert("Document content").await?;
let response = eq.query("What is X?").await?;
```

### 2. Strategy Pattern (Query Modes)

Six different query strategies, selected at runtime:

```
            ┌─────────────────┐
            │   QueryEngine   │
            └────────┬────────┘
                     │
     ┌───────────────┼───────────────┐
     │               │               │
     ▼               ▼               ▼
┌─────────┐    ┌─────────┐    ┌─────────┐
│  Naive  │    │  Local  │    │ Global  │
│ (vector)│    │ (entity)│    │(commun.)│
└─────────┘    └─────────┘    └─────────┘
     │               │               │
     └───────────────┴───────────────┘
                     │
     ┌───────────────┼───────────────┐
     │               │               │
     ▼               ▼               ▼
┌─────────┐    ┌─────────┐    ┌─────────┐
│ Hybrid  │    │   Mix   │    │ Bypass  │
│(L+G)    │    │(weighted)│   │(no RAG) │
└─────────┘    └─────────┘    └─────────┘
```

### 3. Pipeline Pattern (Document Processing)

Sequential processing with configurable stages:

```
┌─────────────────────────────────────────────────────────┐
│                    Pipeline                             │
│                                                         │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌───────┐ │
│  │  Chunk   │──▶│ Extract  │──▶│  Merge   │──▶│ Store │ │
│  │          │   │ (LLM)    │   │ (dedup)  │   │       │ │
│  └──────────┘   └──────────┘   └──────────┘   └───────┘ │
│       │              │              │              │    │
│       ▼              ▼              ▼              ▼    │
│   [config]       [config]       [config]       [config] │
│   chunk_size     batch_size     threshold      backend  │
│   overlap        timeout        strategy       namespace│
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 4. Adapter Pattern (Storage)

Multiple backends behind unified traits:

```
┌─────────────────────────────────────────────────────────┐
│                   Storage Traits                         │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐     │
│  │  KVStorage   │ │VectorStorage │ │ GraphStorage │     │
│  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘     │
└─────────┼────────────────┼────────────────┼─────────────┘
          │                │                │
    ┌─────┴─────┐    ┌─────┴─────┐    ┌─────┴─────┐
    │           │    │           │    │           │
    ▼           ▼    ▼           ▼    ▼           ▼
┌───────┐  ┌───────┐ ┌───────┐  ┌───────┐ ┌───────┐  ┌───────┐
│Memory │  │Postgres││Memory │  │pgvector││Memory │  │  AGE  │
└───────┘  └───────┘ └───────┘  └───────┘ └───────┘  └───────┘
```

---

## Multi-Tenancy Architecture

EdgeQuake supports multi-tenant isolation via `tenant_id` and `workspace_id`:

```
┌─────────────────────────────────────────────────────────┐
│                    Request Flow                         │
│                                                         │
│  Request ──▶ [Middleware] ──▶ [Handler] ──▶ [Storage]   │
│               │                    │             │      │
│               ▼                    ▼             ▼      │
│         Extract tenant       Validate        Filter by  │
│         from header          permissions     namespace  │
│                                                         │
└─────────────────────────────────────────────────────────┘

Isolation enforced at storage layer:
- Tenant A cannot see Tenant B's documents
- Workspace 1 cannot see Workspace 2's entities
```

---

## Next Steps

- **[Data Flow](data-flow.md)** — Detailed ingestion and query flows
- **[Crate Details](crates/)** — Deep dive into each crate
- **[API Reference](../api-reference/rest-api.md)** — REST endpoint documentation

---

## Code References

| Component        | File                               | Lines |
| ---------------- | ---------------------------------- | ----- |
| EdgeQuake struct | edgequake-core/src/orchestrator.rs | 1-300 |
| QueryMode enum   | edgequake-core/src/types/query.rs  | -     |
| Pipeline struct  | edgequake-pipeline/src/pipeline.rs | 1-100 |
| Storage traits   | edgequake-storage/src/traits/      | -     |
| API routes       | edgequake-api/src/routes.rs        | -     |
