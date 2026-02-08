# Iteration 01 - Observe

**Date**: 2026-02-08  
**Mission**: Reliable Document Ingestion Pipeline

## Observations

### 1. Document Upload Flow Testing

Successfully tested the document upload flow using MCP Playwright:

```
UPLOAD TEST RESULT:
- File: "Projet Loi de Finances 2026.pdf" (460KB)
- PDF Conversion: ✅ 10/10 pages extracted
- Entity Extraction: ✅ 5 entities extracted
- Cost: $0.00054
- Status: Completed
```

**Knowledge Graph Verification:**

- Total entities: 200 (from 2 documents)
- Entity types: LOCATION (197), PRODUCT (2), ORGANIZATION (1)
- Connections: 9

### 2. Storage Mode Selection Logic

From `edgequake/src/main.rs` (lines 249-262):

```rust
// Create application state - use PostgreSQL if DATABASE_URL is set
let state = if let Ok(database_url) = std::env::var("DATABASE_URL") {
    info!("🐘 DATABASE_URL detected - using PostgreSQL storage");
    AppState::new_postgres(&database_url, &api_key)
        .await
        .expect("Failed to initialize PostgreSQL storage")
} else {
    info!("💾 No DATABASE_URL set - using in-memory storage (data will not persist)");
    AppState::new_memory(...)
};
```

**Storage mode is correctly determined by `DATABASE_URL` presence.**

### 3. In-Memory Provider Inventory

**Production Code (legitimate):**
| File | Component | Purpose |
|------|-----------|---------|
| `edgequake-api/src/state.rs` | `InMemoryWorkspaceService` | Dev/test mode |
| `edgequake-api/src/state.rs` | `InMemoryConversationService` | Dev/test mode |
| `edgequake-query/src/keywords/cache.rs` | `InMemoryKeywordCache` | Performance cache (valid) |

**Assessment:**

- `InMemoryKeywordCache` is a **valid performance cache**, not to be removed
- `InMemoryWorkspaceService` and `InMemoryConversationService` are used when `DATABASE_URL` is not set
- The concern is ensuring production **always** uses PostgreSQL

### 4. gpt-4o-mini References Found

**CRITICAL: gpt-4o-mini is deprecated/quota exceeded. Must replace with gpt-5-nano.**

| File                                                 | Line                    | Context       |
| ---------------------------------------------------- | ----------------------- | ------------- |
| `edgequake/models.toml`                              | 24                      | Comment       |
| `edgequake/docs/configuration.md`                    | 91, 112                 | Documentation |
| `edgequake-pipeline/src/lineage.rs`                  | 362, 413, 633, 638, 694 | Code + tests  |
| `edgequake-pipeline/tests/cost_integration_tests.rs` | Multiple                | Test files    |

### 5. Current Service Health

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "storage_mode": "postgresql",
  "llm_provider_name": "ollama",
  "components": {
    "kv_storage": true,
    "vector_storage": true,
    "graph_storage": true,
    "llm_provider": true
  }
}
```

**Currently using Ollama provider (local), not OpenAI.**

### 6. File Size Limit

Observed upload size limit of 10MB:

```
File "Note de fiscalité automobile - MAJ 16 avril 2025.pdf" is too large (11.35MB). Maximum size is 10MB.
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        EdgeQuake Server                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌─────────────┐    ┌─────────────────┐    │
│  │   main.rs    │───►│   AppState  │───►│  StorageMode    │    │
│  │              │    │             │    │  ┌────────────┐ │    │
│  │ Check ENV:   │    │ - postgres  │    │  │ Memory     │ │    │
│  │ DATABASE_URL │    │ - memory    │    │  │ PostgreSQL │ │    │
│  └──────────────┘    └─────────────┘    │  └────────────┘ │    │
│                                          └─────────────────┘    │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Document Pipeline                      │   │
│  │  ┌──────────┐ ┌───────────┐ ┌────────────┐ ┌─────────┐  │   │
│  │  │PDF       │►│Text       │►│Entity      │►│Graph    │  │   │
│  │  │Extraction│ │Chunking   │ │Extraction  │ │Storage  │  │   │
│  │  └──────────┘ └───────────┘ └────────────┘ └─────────┘  │   │
│  │                    │                                       │   │
│  │                    ▼                                       │   │
│  │  ┌─────────────────────────────────────────────────────┐  │   │
│  │  │  LLM Provider (Ollama/OpenAI)                       │  │   │
│  │  │  - Entity extraction                                 │  │   │
│  │  │  - Embedding generation                              │  │   │
│  │  │  MODEL: gpt-4o-mini ← DEPRECATED (replace: gpt-5-nano)  │   │
│  │  └─────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Key Finding

**The ingestion pipeline IS WORKING** when:

1. `DATABASE_URL` is set (PostgreSQL mode)
2. Ollama or OpenAI provider is available
3. PDF files are under 10MB limit

**Issues to Fix:**

1. Replace `gpt-4o-mini` with `gpt-5-nano` in all documentation and defaults
2. Ensure no accidental use of in-memory storage in production
3. Clean up dead code (if any)
