# OODA Iteration 02 - Orient

**Date**: 2026-01-29
**Focus**: Strategic analysis for architecture documentation

---

## 1. First Principles: Why This Architecture?

### The RAG Problem

Traditional RAG treats documents as bags of chunks:

```
Document → Chunks → Vectors → Similarity Search → Context → LLM
```

**Limitation**: Loses semantic relationships between concepts.

### The LightRAG Solution

Add a knowledge graph layer:

```
Document → Chunks + Entities + Relationships
              │         │            │
              ▼         ▼            ▼
         Vectors   Graph Nodes   Graph Edges
              │         │            │
              └─────────┴────────────┘
                        │
                        ▼
              Hybrid Search (vector + graph)
```

### Why EdgeQuake's 11-Crate Design?

**Single Responsibility Principle**:

- Each crate does ONE thing well
- Easy to test in isolation
- Clear dependency boundaries
- Compile-time enforcement of layering

**Dependency Inversion**:

- Core depends on traits, not implementations
- Storage/LLM swappable at runtime
- Testing uses mocks without code changes

---

## 2. Architectural Patterns Identified

### Pattern 1: Orchestrator (edgequake-core)

```
                EdgeQuake (Facade)
                       │
       ┌───────────────┼───────────────┐
       │               │               │
    Pipeline      QueryEngine      Storage
       │               │               │
       └───────────────┴───────────────┘
                       │
                  LLM Providers
```

**Why**: Single entry point simplifies API, coordinates complex operations.

### Pattern 2: Trait-based Abstraction (edgequake-llm, edgequake-storage)

```
    ┌──────────────────────────────────────┐
    │           Trait Interface            │
    │   (LLMProvider, EmbeddingProvider)   │
    └──────────────────────────────────────┘
                       │
       ┌───────────────┼───────────────┐
       │               │               │
   OpenAI          Ollama           Mock
```

**Why**: Runtime provider switching, testability, extensibility.

### Pattern 3: Pipeline Pattern (edgequake-pipeline)

```
Chunk → Extract → Merge → Store
  │        │        │        │
  └──▶ Stage can be disabled/configured
```

**Why**: Flexible processing, stage-level caching, easy debugging.

---

## 3. Documentation Gap Analysis

### Current State (archive/docs/0002-architecture-overview.md)

- ✅ Good ASCII diagrams
- ✅ Crate descriptions
- ❌ Not in docs/ (in archive/)
- ❌ Missing WHY explanations
- ❌ Missing data flow details

### Needed Documentation

| Document                  | Priority | Status          |
| ------------------------- | -------- | --------------- |
| architecture/overview.md  | HIGH     | Create new      |
| architecture/data-flow.md | HIGH     | Create new      |
| architecture/crates/\*.md | MEDIUM   | Create skeleton |

---

## 4. High-Signal Content Strategy

### For Overview.md

1. Start with system diagram (ASCII)
2. Explain WHY the design (First Principles)
3. Show crate dependency graph
4. Link to detailed crate docs

### For Data-Flow.md

1. Ingestion sequence diagram
2. Query sequence diagram
3. State machine for document processing
4. Error handling flow

### For Crate Docs

1. Purpose (single sentence)
2. Key types and traits
3. Usage example
4. Integration points

---

## 5. Verification Against Code

Will verify:

- [ ] All crates mentioned exist in Cargo.toml
- [ ] Trait definitions match actual code
- [ ] API endpoints match routes.rs
- [ ] Feature IDs cross-reference correctly

---

## 6. Risk Assessment

| Risk                                      | Likelihood | Mitigation                  |
| ----------------------------------------- | ---------- | --------------------------- |
| Architecture changes before docs complete | LOW        | Reference git hashes        |
| Diagrams become stale                     | MEDIUM     | Use code comments as source |
| Over-documentation                        | MEDIUM     | Focus on WHY over WHAT      |
