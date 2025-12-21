# LightRAG Technical Debt & Known Issues

## Overview

This document catalogs technical debt, deprecated features, known limitations, and improvement opportunities in the LightRAG codebase. Use this as a reference for maintenance, refactoring, and migration planning.

---

## Deprecated Features

### API Deprecations

| Feature | Location | Status | Replacement |
|---------|----------|--------|-------------|
| `/documents/status` endpoint | `document_routes.py:2736` | Deprecated | Use `/documents/paginated` |
| `max_tokens` parameter | `binding_options.py:535` | Deprecated | Use `max_completion_tokens` |
| `history_turns` parameter | `base.py:144` | Deprecated | Conversation history sent directly to LLM |
| `auto_manage_storages_states` | `base.py:600` | Deprecated | Use explicit `initialize_storages()` |
| `merge_strategy` parameter | `utils_graph.py:1185,1553` | Deprecated | Each field uses built-in strategy |

### Schema Changes

| Change | Affected Storage | Migration Required |
|--------|------------------|-------------------|
| `content` field removal | DocStatus storage | Auto-migrated on load |
| `mode` field removal | LLM cache (PostgreSQL) | Auto-migrated via `migrate_llm_cache_schema()` |

### Example: Deprecated Field Handling

```python
# Pattern from json_doc_status_impl.py:114
# Remove deprecated content field if it exists
if "content" in doc_status:
    del doc_status["content"]
```

---

## Known Limitations

### Chunking

```yaml
chunking_limitations:
  - description: Token-based chunking may split mid-sentence
    impact: Reduced extraction quality
    workaround: Use split_by_character options
    
  - description: Overlap is token-based, not semantic
    impact: Related content may still be split
    workaround: Increase overlap size
    
  - description: No support for structured document formats
    impact: Tables, code blocks may be split incorrectly
    workaround: Pre-process documents before insertion
```

### Entity Extraction

```yaml
extraction_limitations:
  - description: Relies on LLM output format compliance
    impact: Malformed output loses entities
    mitigation: Retry logic with format hints
    
  - description: Entity type normalization is basic
    impact: "person" vs "people" treated differently
    workaround: Post-processing normalization
    
  - description: Cross-chunk entity resolution not guaranteed
    impact: Same entity in different chunks may not merge
    workaround: Increase chunk overlap
```

### Query Processing

```yaml
query_limitations:
  - description: Community detection not implemented
    impact: Global mode falls back to high-degree nodes
    related: Leiden algorithm planned but not complete
    
  - description: No query caching by default
    impact: Repeated queries hit LLM each time
    workaround: Enable LLM cache layer
    
  - description: Streaming mode has limited error recovery
    impact: Mid-stream failures require full retry
    mitigation: Use non-streaming for critical queries
```

### Storage

```yaml
storage_limitations:
  - description: NanoVectorDB is memory-bound
    impact: Large datasets may cause OOM
    workaround: Use Milvus/Qdrant for production
    
  - description: NetworkX graph not persistent by default
    impact: Graph rebuilt on restart
    workaround: Use Neo4j for persistence
    
  - description: JSON storage has no atomic transactions
    impact: Concurrent writes may corrupt data
    workaround: Use database backends for production
```

---

## Technical Debt Items

### Code Quality

```yaml
code_debt:
  - location: lightrag/operate.py
    issue: File is 5000+ lines
    recommendation: Split into logical modules (chunking, extraction, merging, query)
    priority: Medium
    
  - location: lightrag/lightrag.py
    issue: File is 4000+ lines with many methods
    recommendation: Extract API layer, extract storage management
    priority: Medium
    
  - location: lightrag/utils.py
    issue: Utility file over 2800 lines
    recommendation: Split by domain (text, async, config)
    priority: Low
```

### Error Handling

```yaml
error_debt:
  - location: lightrag/operate.py
    issue: Silent exception catching in extraction parsing
    recommendation: Log warnings for malformed output
    priority: High
    
  - location: lightrag/llm/*.py
    issue: Inconsistent retry logic across providers
    recommendation: Centralize retry decorator with configurable policy
    priority: Medium
```

### Testing

```yaml
testing_debt:
  - issue: Low unit test coverage for operate.py algorithms
    recommendation: Add tests for chunking, extraction parsing, merging
    priority: High
    
  - issue: Mock-heavy tests don't verify integration
    recommendation: Add more integration tests with real LLM
    priority: Medium
    
  - issue: No performance regression tests
    recommendation: Add benchmark suite with baselines
    priority: Low
```

### Documentation

```yaml
documentation_debt:
  - issue: API docstrings inconsistent in format
    recommendation: Standardize on Google-style docstrings
    priority: Low
    
  - issue: Some algorithms lack inline comments
    recommendation: Add step-by-step comments for complex logic
    priority: Medium
```

---

## Security Considerations

### Current Gaps

```yaml
security_gaps:
  - issue: No input sanitization for entity names
    risk: Injection in graph queries
    recommendation: Sanitize before storage operations
    priority: High
    
  - issue: LLM prompts may leak sensitive document content
    risk: Data exposure via prompt injection
    recommendation: Add content filtering option
    priority: Medium
    
  - issue: No rate limiting on API endpoints
    risk: DoS via excessive queries
    recommendation: Add configurable rate limits
    priority: Medium
```

### Implemented Mitigations

```yaml
security_implemented:
  - Multi-tenancy with namespace isolation
  - Document ID hashing prevents enumeration
  - Async processing prevents blocking attacks
  - Storage callbacks for audit logging
```

---

## Performance Optimization Opportunities

### Identified Optimizations

```yaml
optimizations:
  - area: Document Insertion
    current: Sequential chunk processing
    improvement: Parallel chunk extraction with batching
    estimated_gain: 2-3x throughput
    
  - area: Vector Search
    current: Search all vectors then filter
    improvement: Pre-filter by source ID
    estimated_gain: 10-50% latency reduction
    
  - area: Entity Merging
    current: In-memory aggregation
    improvement: Streaming merge with bloom filter
    estimated_gain: 50% memory reduction
    
  - area: Description Summarization
    current: Map-reduce on every update
    improvement: Incremental summarization with caching
    estimated_gain: 80% fewer LLM calls
```

### Memory Optimization

```yaml
memory_optimizations:
  - issue: Full document kept in memory during processing
    recommendation: Stream chunks to storage immediately
    
  - issue: Entity/relationship lists grow unbounded
    recommendation: Implement pagination for large extractions
    
  - issue: Graph traversal loads full subgraph
    recommendation: Implement lazy loading with iterators
```

---

## Refactoring Roadmap

### Short-Term (1-2 months)

```yaml
short_term:
  - Remove deprecated parameters with migration guide
  - Add deprecation warnings for planned removals
  - Split operate.py into logical modules
  - Add unit tests for core algorithms
  - Standardize error handling patterns
```

### Medium-Term (3-6 months)

```yaml
medium_term:
  - Implement community detection (Leiden algorithm)
  - Add query result caching layer
  - Implement streaming document insertion
  - Add performance benchmarks
  - Refactor LLM integration with unified retry policy
```

### Long-Term (6-12 months)

```yaml
long_term:
  - Implement multi-model embedding support
  - Add real-time graph update streaming
  - Implement distributed processing mode
  - Add advanced security features (content filtering)
  - Create migration tools for storage backend changes
```

---

## Migration Notes

### Breaking Changes Log

```yaml
breaking_changes:
  v2.0:
    - auto_manage_storages_states removed
    - history_turns parameter removed
    - JSON storage schema updated
    
  v1.5:
    - LLM cache mode field removed
    - DocStatus content field removed
```

### Upgrade Checklist

```yaml
upgrade_checklist:
  - [ ] Review deprecated feature usage
  - [ ] Update to new API parameter names
  - [ ] Run storage migrations if needed
  - [ ] Test with new version in staging
  - [ ] Update client integrations
```

---

## Issue Tracking

### Known Issues

| ID | Description | Severity | Status |
|----|-------------|----------|--------|
| #001 | Community detection not implemented | Low | Planned |
| #002 | Entity type normalization inconsistent | Medium | Open |
| #003 | Streaming errors not recoverable | Medium | Open |
| #004 | Large document insertion memory spike | Medium | Investigating |

### Workarounds

```yaml
workarounds:
  - issue: Memory spike on large documents
    workaround: Pre-chunk documents before insertion
    
  - issue: Entity extraction format errors
    workaround: Use models with better instruction following
    
  - issue: Graph query timeout
    workaround: Reduce max_depth and max_nodes parameters
```

---

## Cross-References

- [Configuration](08-configuration.md) - Parameter defaults and deprecations
- [Security & Errors](09-security-errors.md) - Error handling patterns
- [Testing](10-testing-quality.md) - Test coverage requirements
- [Rebuild Checklist](11-rebuild-checklist.md) - Implementation priorities
