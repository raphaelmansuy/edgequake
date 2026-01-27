# Appendix B: Architecture Decision Log

## Overview

This document records significant architecture decisions made in LightRAG, including the context, options considered, and rationale for each decision. Use this as a reference for understanding "why" choices were made.

---

## ADR-001: Graph-Based Knowledge Representation

**Status:** Accepted  
**Date:** Foundation  

### Context
Traditional RAG systems use only vector similarity for retrieval. This misses structural relationships between concepts and entities.

### Decision
Build knowledge graphs from extracted entities and relationships, enabling graph-based retrieval alongside vector search.

### Options Considered
1. **Pure vector search** - Simple but loses structural information
2. **Keyword extraction** - Lightweight but no relationship capture
3. **Knowledge graph construction** - Complex but rich representation
4. **Hybrid approach** - Graph + vector (chosen)

### Rationale
- Graph structure captures relationships that vectors cannot
- Enables multi-hop reasoning across connected entities
- Supports both local (entity-focused) and global (structure-focused) queries
- Vector embeddings still capture semantic similarity

### Consequences
- Increased complexity in extraction pipeline
- LLM calls required for entity/relationship extraction
- Need for graph storage infrastructure
- Richer query capabilities

---

## ADR-002: Async-First Architecture

**Status:** Accepted  
**Date:** Foundation  

### Context
RAG pipelines involve many I/O-bound operations: LLM calls, embedding generation, storage operations.

### Decision
Use Python's async/await throughout the codebase for non-blocking operations.

### Options Considered
1. **Synchronous with threading** - Simpler code but GIL limitations
2. **Multiprocessing** - True parallelism but IPC overhead
3. **Async/await** - Event-driven, efficient I/O (chosen)
4. **Mixed approach** - Complexity burden

### Rationale
- LLM and embedding calls are I/O-bound (network)
- Storage operations are I/O-bound (disk/network)
- Async allows high concurrency without thread overhead
- Python ecosystem has mature async support

### Consequences
- Async virus: callers must be async too
- Sync wrappers needed for legacy integration
- Debugging async code requires tooling knowledge
- Connection pool management required

---

## ADR-003: Pluggable Storage Backends

**Status:** Accepted  
**Date:** Foundation  

### Context
Different deployment scenarios require different storage solutions. Development needs simplicity; production needs scale.

### Decision
Define abstract storage interfaces with multiple implementations.

### Options Considered
1. **Single storage backend** - Simple but inflexible
2. **Configuration-based selection** - Flexible but limited (chosen partial)
3. **Plugin architecture** - Maximum flexibility (chosen)
4. **External storage service** - Decoupled but dependency

### Rationale
- Development: File-based storage for zero dependencies
- Testing: In-memory storage for speed
- Production: Database backends for scale and persistence
- Users can implement custom backends

### Consequences
- Interface contracts must be stable
- Testing required across all implementations
- Documentation for each backend
- Potential inconsistencies between implementations

---

## ADR-004: LLM Agnostic Design

**Status:** Accepted  
**Date:** Foundation  

### Context
The LLM landscape is rapidly evolving. Users need flexibility in model selection.

### Decision
Accept LLM and embedding functions as callables rather than binding to specific providers.

### Options Considered
1. **OpenAI-only** - Simple but vendor lock-in
2. **LangChain integration** - Ecosystem but dependency
3. **Function-based abstraction** - Flexible (chosen)
4. **Provider plugins** - Complex registration system

### Rationale
- Users can use any LLM (cloud, local, custom)
- No forced dependencies on LLM libraries
- Easy to add new providers without core changes
- Testing with mock functions is straightforward

### Consequences
- Users must provide their own LLM wrappers
- Helper implementations provided but optional
- Token counting may vary by model
- Provider-specific features require custom code

---

## ADR-005: Token-Based Chunking

**Status:** Accepted  
**Date:** Foundation  

### Context
Documents must be split into chunks for processing. Various strategies exist.

### Decision
Use token-based chunking with configurable overlap.

### Options Considered
1. **Fixed character length** - Simple but variable token count
2. **Sentence-based** - Natural boundaries but variable size
3. **Token-based** - Consistent LLM context usage (chosen)
4. **Semantic chunking** - Optimal but expensive

### Rationale
- LLM context windows are token-limited
- Consistent chunk sizes for predictable costs
- Overlap preserves context across boundaries
- Configurable for different use cases

### Consequences
- May split mid-sentence
- Token counting overhead
- Tokenizer dependency (tiktoken)
- Split_by_character option for natural breaks

---

## ADR-006: MD5 for Content Hashing

**Status:** Accepted  
**Date:** Foundation  

### Context
Documents and chunks need unique, deterministic identifiers for deduplication.

### Decision
Use MD5 hash of content as identifier.

### Options Considered
1. **UUID** - Unique but not deterministic
2. **Incremental ID** - Simple but no dedup
3. **MD5 hash** - Deterministic, fast (chosen)
4. **SHA-256** - More secure but unnecessary

### Rationale
- Same content always produces same ID
- Enables duplicate detection across sessions
- Fast to compute even for large content
- 32-character hex string is manageable

### Consequences
- Hash collisions theoretically possible (negligible)
- Content changes create new IDs
- No cryptographic security (not needed)
- Works well for content-addressable storage

---

## ADR-007: Uppercase Entity Normalization

**Status:** Accepted  
**Date:** Implementation  

### Context
Entity names may appear in different cases across documents. Need consistent deduplication.

### Decision
Normalize all entity names to uppercase.

### Options Considered
1. **Case-sensitive** - Preserves original but fragments entities
2. **Lowercase** - Common but reads oddly
3. **Uppercase** - Stands out as entity (chosen)
4. **Title case** - Pretty but complex rules

### Rationale
- "John Doe", "JOHN DOE", "john doe" become one entity
- Uppercase visually distinguishes entities in output
- Simple transformation, no complex rules
- Consistent with knowledge graph conventions

### Consequences
- Original casing not preserved
- Proper nouns like "iOS" become "IOS"
- Potential false merges for case-significant terms
- Clear visual distinction in graph visualization

---

## ADR-008: Map-Reduce for Large Summaries

**Status:** Accepted  
**Date:** Implementation  

### Context
Entity descriptions can grow very large when merged across many chunks. LLM context limits apply.

### Decision
Use map-reduce pattern for summarizing large descriptions.

### Options Considered
1. **Truncation** - Fast but loses information
2. **Random sampling** - Quick but inconsistent
3. **Map-reduce** - Complete but slow (chosen)
4. **Hierarchical merging** - Complex but optimal

### Rationale
- All description content is considered
- Scales to arbitrary description length
- LLM produces coherent summaries
- Pattern is well-understood

### Consequences
- Multiple LLM calls for large descriptions
- Latency increases with description size
- Potential information loss in summarization
- Configurable summary token limit

---

## ADR-009: Multi-Tenancy via Namespace Isolation

**Status:** Accepted  
**Date:** v2.0  

### Context
Multiple organizations need to use LightRAG without data leakage.

### Decision
Implement tenant isolation through storage namespace prefixes.

### Options Considered
1. **Separate instances** - Complete isolation but resource heavy
2. **Database-level isolation** - Secure but complex
3. **Namespace prefixing** - Efficient (chosen)
4. **Row-level security** - Backend-specific

### Rationale
- Single deployment serves multiple tenants
- Namespace prefixes isolate data in shared storage
- No cross-tenant query possible by design
- Efficient resource utilization

### Consequences
- Storage implementations must respect namespaces
- Tenant ID required for all operations
- Admin operations span all tenants
- Performance scales with total data, not per-tenant

---

## ADR-010: Query Modes as Enum

**Status:** Accepted  
**Date:** Implementation  

### Context
Different retrieval strategies suit different query types.

### Decision
Define discrete query modes with specific retrieval behavior.

### Options Considered
1. **Single strategy** - Simple but limiting
2. **Continuous parameters** - Flexible but complex
3. **Discrete modes** - Clear behavior (chosen)
4. **Custom pipelines** - Maximum flexibility but complex

### Rationale
- Clear expectations for each mode
- Easy to document and explain
- Extensible by adding new modes
- Bypass mode enables custom processing

### Consequences
- Users must choose mode (or use default)
- Mode behavior is fixed
- New modes require code changes
- Hybrid is default for most use cases

---

## ADR-011: Sync Wrappers for Async Methods

**Status:** Accepted  
**Date:** Implementation  

### Context
Not all Python code is async. Users may need sync interfaces.

### Decision
Provide sync wrapper methods that run async methods in an event loop.

### Options Considered
1. **Async only** - Modern but breaking for sync code
2. **Sync with threading** - Traditional but inconsistent
3. **Sync wrappers** - Compatible (chosen)
4. **Separate sync implementation** - Duplication

### Rationale
- `insert()` wraps `ainsert()`
- `query()` wraps `aquery()`
- Backward compatible with sync code
- No code duplication

### Consequences
- Event loop creation overhead
- Cannot be called from async context (nested loop)
- Two entry points to document
- Sync wrappers slightly slower

---

## ADR-012: Cascade Deletion

**Status:** Accepted  
**Date:** Implementation  

### Context
When documents are deleted, related data must be cleaned up.

### Decision
Implement cascade deletion from documents through chunks to orphaned entities.

### Options Considered
1. **Document only** - Fast but orphans data
2. **Full cascade** - Clean but slow (chosen)
3. **Soft delete** - Recoverable but complex
4. **Background cleanup** - Async but eventual

### Rationale
- Deleting document removes all derived data
- Orphaned entities (no source) are deleted
- VDB entries updated to reflect source changes
- Graph consistency maintained

### Consequences
- Deletion is slow for large documents
- Must track entity sources accurately
- Partial failures may leave orphans
- No soft delete/recovery option

---

## Decision Template

For future decisions, use this template:

```markdown
## ADR-XXX: [Title]

**Status:** [Proposed | Accepted | Deprecated | Superseded]
**Date:** [Date of decision]

### Context
[What is the issue we're addressing?]

### Decision
[What have we decided to do?]

### Options Considered
1. [Option 1] - [Pros/Cons]
2. [Option 2] - [Pros/Cons]
3. [Chosen option] - [Why chosen]

### Rationale
[Why was this decision made?]

### Consequences
[What are the positive and negative consequences?]
```

---

## Cross-References

- [Architecture Overview](../02-architecture.md) - System design
- [Technical Debt](../12-technical-debt.md) - Known issues
- [Rebuild Checklist](../11-rebuild-checklist.md) - Implementation guidance
