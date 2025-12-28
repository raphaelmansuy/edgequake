# Ingestion Pipeline Scratchpad

> Working notes for SOTA GenAI-powered ingestion pipeline design.
> Last updated: 2024-12-28
> Status: ✅ COMPLETE - All deliverables created

## Deliverables Completed

| Document | Status | Description |
|----------|--------|-------------|
| [01-architecture.md](01-architecture.md) | ✅ | System architecture with ASCII diagrams |
| [02-comparison.md](02-comparison.md) | ✅ | Rust vs Python feature comparison |
| [03-data-models.md](03-data-models.md) | ✅ | Complete data model specifications |
| [04-api-contracts.md](04-api-contracts.md) | ✅ | API endpoint definitions |
| [05-implementation-plan.md](05-implementation-plan.md) | ✅ | Phased implementation roadmap |
| [06-testing-strategy.md](06-testing-strategy.md) | ✅ | Test plans and strategies |
| [plan.md](plan.md) | ✅ | Master plan consolidating all deliverables |

---

## Session 1: Codebase Analysis

### Current Rust Implementation (edgequake/)

**Crate Structure:**
```
edgequake/crates/
├── edgequake-api/       # REST API with Axum
├── edgequake-auth/      # Authentication
├── edgequake-core/      # Orchestrator, tenant manager
├── edgequake-llm/       # LLM providers (OpenAI, Mock)
├── edgequake-pipeline/  # Document processing pipeline ← KEY
├── edgequake-query/     # Query engine
├── edgequake-storage/   # Storage adapters
└── edgequake-tasks/     # Background tasks
```

**Pipeline Architecture (Current):**
```
Document → Chunker → Extractor → Merger → Embeddings → Storage
             │           │           │
             ↓           ↓           ↓
         TextChunk   Entities    GraphNode
                   Relationships GraphEdge
```

**Key Components Identified:**

1. **Pipeline (pipeline.rs)**
   - PipelineConfig: chunk sizes, batch sizes, feature flags
   - ProcessingResult: document_id, chunks, extractions, stats
   - ProcessingStats: chunk_count, entity_count, llm_calls, total_tokens

2. **Chunker (chunker.rs)**
   - ChunkerConfig: chunk_size, overlap, min_size, separators
   - TextChunk: id, content, index, start_offset, end_offset, token_count, embedding
   - ChunkingStrategy trait: allows custom chunkers
   - TokenBasedChunking: default implementation
   - CharacterBasedChunking: for pre-split content (GAP-017)

3. **Extractor (extractor.rs)**
   - ExtractionResult: entities, relationships, source_chunk_id, metadata
   - ExtractedEntity: name, entity_type, description, importance, source_spans, embedding
   - ExtractedRelationship: source, target, relation_type, description, weight, keywords, embedding
   - EntityExtractor trait
   - SimpleExtractor: regex-based (testing)
   - LLMExtractor: real LLM extraction
   - GleaningExtractor: re-extraction for missed entities (GAP-018)

4. **Merger (merger.rs)**
   - MergerConfig: max_description_length, description_decay, min_importance
   - KnowledgeGraphMerger: merges into graph storage
   - Description merging (keeps longer)
   - Keyword merging

5. **Orchestrator (orchestrator.rs)**
   - EdgeQuakeConfig: namespace, tenant_id, workspace_id, LLM/embedding models
   - EdgeQuake: main coordinator

### Current Python Implementation (lightrag/)

**Key Files:**
```
lightrag/
├── operate.py           # Main operations (5000 lines!)
├── prompt.py            # LLM prompts
├── lightrag.py          # Main class
├── base.py              # Storage interfaces
├── kg/                  # Knowledge graph operations
└── tenant_rag_manager.py
```

**Key Patterns Observed:**

1. **Map-Reduce for Description Summarization**
   - `_handle_entity_relation_summary()`: Uses map-reduce when descriptions exceed token limits
   - Chunks descriptions, summarizes each chunk, then recursively summarizes summaries
   - `force_llm_summary_on_merge` config option

2. **LLM Caching**
   - Extensive caching of extraction results
   - `llm_cache_list` per chunk for rebuilding
   - Can rebuild KG from cached extractions

3. **Tuple-Based Extraction Format**
   - Uses `<|#|>` as tuple delimiter
   - Format: `entity<|#|>name<|#|>type<|#|>description`
   - Format: `relation<|#|>source<|#|>target<|#|>keywords<|#|>description`

4. **Pipeline Status Tracking**
   - Detailed progress tracking
   - `latest_message` and `history_messages`
   - Error counts, success counts

5. **Entity Name Normalization**
   - `sanitize_and_normalize_extracted_text()`
   - Consistent uppercase normalization
   - Truncation for long identifiers

### Gaps Identified (vs Spec Requirements)

| Requirement | Current Status | Gap |
|------------|----------------|-----|
| Line number tracking (start/end) | Char offsets only | **MISSING** |
| Full lineage (doc→chunk→entity) | Partial | **NEEDS ENHANCEMENT** |
| Cost tracking (tokens, $) | Basic (llm_calls, tokens) | **NEEDS DETAIL** |
| MapReduce for large docs | Not implemented | **MISSING** |
| Progress API | Basic stats | **NEEDS ENHANCEMENT** |
| Document suppression | Not implemented | **MISSING** |
| Entity CRUD with cascade | Partial | **NEEDS ENHANCEMENT** |
| Citation retrieval | Not implemented | **MISSING** |
| RAGAS/MLflow integration | Not implemented | **MISSING** |
| Ontology schema | Not implemented | **FUTURE** |
| Multi-namespace queries | Not implemented | **FUTURE** |

### Architecture Decisions Needed

1. **Lineage Model**: How to track doc→chunk→entity relationships?
   - Option A: Embedded in each entity/relationship
   - Option B: Separate lineage table/storage
   - **Recommendation**: Separate lineage storage for flexibility

2. **Cost Model**: How to track and attribute costs?
   - Per-document, per-chunk, per-entity
   - LLM provider costs (input/output tokens, model)
   - **Recommendation**: IngestionCost struct at each level

3. **Progress Reporting**: Granularity?
   - Document level: started, chunks_created, entities_extracted, completed
   - Pipeline level: total_docs, completed_docs, failed_docs
   - **Recommendation**: Both levels with event streaming

4. **Document Suppression**: What happens to graph?
   - Option A: Mark as deleted (soft delete)
   - Option B: Remove entities/relationships with only this source
   - Option C: Decrement weights, remove if zero
   - **Recommendation**: Option B with tombstones for audit

5. **Multi-tenant Isolation**: Storage level or API level?
   - Currently: namespace-based isolation
   - **Recommendation**: Both - storage-level partitioning + API-level access control

---

## Session Notes

### TODO for Next Phase

- [ ] Create comprehensive data model diagram
- [ ] Define all new struct/enum definitions
- [ ] Design API endpoints for pipeline monitoring
- [ ] Create comparison matrix (Rust vs Python features)
- [ ] Define MapReduce implementation strategy
- [ ] Design evaluation suite integration

### Questions to Resolve

1. Should lineage track all intermediate transformations?
2. How to handle entity merges in citation tracking?
3. What level of cost granularity is needed for billing?
4. How to support predefined ontology schemas?

### Key Insights

1. LightRAG's tuple-based extraction is more robust for parsing
2. Map-reduce is essential for large documents (>100 chunks)
3. Caching enables efficient rebuilding without re-extraction
4. Progress tracking should be event-driven for real-time updates

---
