# OODA Loop Iteration 01 - ORIENT

**Date**: 2026-01-09  
**Focus**: Prioritize documentation improvements by impact

---

## Analysis of Documentation Needs

### Priority Matrix

| Priority | Area                         | Impact | Effort | Rationale                            |
| -------- | ---------------------------- | ------ | ------ | ------------------------------------ |
| **P0**   | Create FEAT/BR/UC registries | High   | Medium | Foundation for all traceability      |
| **P0**   | Core orchestrator docs       | High   | Medium | Entry point for all operations       |
| **P1**   | Query engine SOTA docs       | High   | High   | Most complex algorithm               |
| **P1**   | Pipeline extraction docs     | High   | Medium | Critical for understanding ingestion |
| **P2**   | Storage backend docs         | Medium | Low    | Already decent, needs refs           |
| **P2**   | API handler docs             | Medium | Medium | Frontend depends on this             |
| **P3**   | PDF extraction docs          | Medium | High   | Complex but specialized              |
| **P3**   | WebUI component docs         | Low    | High   | Frontend-specific                    |

---

## Feature Categories Identified

### Core RAG Features (FEAT00XX)

1. Document ingestion
2. Text chunking with overlap
3. Entity extraction (SOTA tuple format)
4. Relationship extraction
5. Knowledge graph construction
6. Vector embedding generation
7. Multi-mode query execution
8. Streaming response generation

### Query Features (FEAT01XX)

1. Naive vector search
2. Local entity-centric search
3. Global community-based search
4. Hybrid search
5. Mix weighted search
6. Bypass (no RAG) mode
7. Keyword extraction (LLM-based)
8. Context truncation

### Storage Features (FEAT02XX)

1. In-memory storage (testing)
2. PostgreSQL KV storage
3. pgvector integration
4. Apache AGE graph storage
5. Community detection
6. Multi-tenant isolation

### Pipeline Features (FEAT03XX)

1. Character-based chunking
2. Token-based chunking
3. LLM-based extraction
4. Entity normalization
5. Description summarization
6. Lineage tracking
7. Progress reporting
8. Cost tracking

### API Features (FEAT04XX)

1. Document upload (file)
2. Document upload (text)
3. Query execution
4. Query streaming (SSE)
5. Graph exploration
6. Task tracking
7. Conversation management
8. Workspace management

---

## Business Rules Identified

### Data Integrity (BR00XX)

1. Entity names must be normalized (UPPERCASE_UNDERSCORED)
2. Chunk overlap must be < chunk size
3. Document IDs must be unique per tenant
4. Embeddings must match configured dimensions

### Query Processing (BR01XX)

1. Token budget must not exceed LLM context limit
2. Graph context takes priority over naive context
3. Streaming must use SSE format
4. Query mode must be valid enum value

### Multi-Tenancy (BR02XX)

1. Tenant isolation is mandatory
2. API key maps to single tenant
3. Cross-tenant queries forbidden
4. Rate limits are per-tenant

### Cost Management (BR03XX)

1. LLM calls must be tracked
2. Caching reduces redundant calls
3. Batch processing for efficiency
4. Progress must report cost estimates

---

## Use Cases Identified

### Document Management (UC00XX)

1. Upload single document (text)
2. Upload document file (PDF, TXT, MD)
3. List documents in workspace
4. Delete document
5. Re-process failed document

### Knowledge Graph (UC01XX)

1. View graph visualization
2. Explore entity relationships
3. Search entities by name
4. Create manual entity
5. Create manual relationship

### Query Execution (UC02XX)

1. Execute simple query
2. Execute query with mode selection
3. Stream query response
4. Query with conversation context
5. Query specific workspace

### Workspace Management (UC03XX)

1. Create workspace
2. List workspaces
3. Delete workspace
4. Get workspace statistics

### Conversation Management (UC04XX)

1. Create conversation
2. List conversations
3. Add message to conversation
4. Get conversation history
5. Delete conversation

---

## Documentation Strategy

### Phase 1: Foundation (Iterations 1-10)

- Create central registry files (features.md, business_rules.md, use_cases.md)
- Document core types and orchestrator
- Add FEAT/BR/UC references to lib.rs files

### Phase 2: Core Algorithms (Iterations 11-25)

- Document query engine SOTA algorithm
- Document pipeline extraction process
- Document storage layer patterns

### Phase 3: Integration Points (Iterations 26-40)

- Document API contracts
- Document WebUI integration
- Add cross-references

### Phase 4: Polish (Iterations 41-50)

- Review all docstrings
- Ensure non-regression
- Final validation

---

## Key Insights

1. **Foundation first** - Registry files enable systematic traceability
2. **High-impact areas** - Core and query crates affect all users
3. **Algorithm docs critical** - SOTA query engine is the differentiator
4. **Incremental approach** - Small commits with feature refs

---

## Next Steps

→ Decide: Define specific actions for iteration 01
