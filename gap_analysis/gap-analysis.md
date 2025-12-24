# Gap Analysis: LightRAG (Python) → EdgeQuake (Rust)

**Generated:** 2024-12-24  
**Source Version:** LightRAG Python (lightrag/ directory)  
**Target Version:** EdgeQuake Rust (edgequake/ directory)  
**Analyst:** AI Gap Analysis System

---

## Executive Summary

### Overall Parity Score: 53.8%

| Status            | Count | Percentage |
| ----------------- | ----- | ---------- |
| ✅ Full Parity    | 42    | 53.8%      |
| ⚠️ Partial        | 14    | 17.9%      |
| ❌ Missing        | 21    | 26.9%      |
| ⬆️ Target Exceeds | 1     | 1.3%       |

### Critical Gaps Summary (P0)

1. **GAP-001: Query Mode: Global** - Essential query mode for high-level knowledge synthesis is not implemented
2. **GAP-002: Query Mode: Mix** - Primary recommended query mode combining KG and vector retrieval is missing
3. **GAP-003: Multi-tenancy Support** - Core multi-tenant features for production SaaS deployment incomplete
4. **GAP-004: Tenant RAG Manager** - Per-tenant/KB instance management not implemented

### Key Findings

1. **Core RAG Pipeline Functional**: Basic document ingestion, chunking, extraction, and naive/local queries work
2. **Advanced Query Modes Missing**: Global and Mix modes—LightRAG's signature features—are not implemented
3. **Multi-Tenancy Incomplete**: Basic auth exists, but tenant isolation and RAG instance management are partial
4. **LLM Provider Ecosystem Limited**: Only OpenAI and partial Mock providers; missing Anthropic, Gemini, Ollama, etc.
5. **Performance Optimizations Missing**: No priority queue, rate limiting, or embedding cache
6. **Storage Backends Limited**: Only Memory and PostgreSQL; missing Neo4j, Redis, MongoDB, Milvus, Qdrant

### Recommendation

**NOT READY FOR PRODUCTION** - EdgeQuake has solid architectural foundations but lacks critical query modes and multi-tenancy features that define LightRAG's value proposition. Addressing P0 gaps is essential before production deployment.

---

## Detailed Analysis by Category

---

## CORE: Core RAG Functionality

### Feature: F-001 Document Insert (sync)

**Status:** ✅ Full Parity

| Aspect     | Source                 | Target                      |
| ---------- | ---------------------- | --------------------------- |
| Location   | `lightrag.py:insert()` | `orchestrator.rs:insert()`  |
| Sync/Async | Both supported         | Async with blocking wrapper |
| Return     | track_id string        | InsertResult struct         |

**Notes:** EdgeQuake returns richer result struct with counts; functionally equivalent.

---

### Feature: F-015 Query Mode: Global

**Status:** ❌ Missing  
**Gap ID:** GAP-001  
**Severity:** P0 (Critical)

#### Source Implementation

**Location:** `lightrag/operate.py:kg_query()` (lines 2000-2500)

**Description:** Global query mode retrieves high-level concepts and relationships from the entire knowledge graph, enabling synthesis across documents.

**Behavior:**

- Extracts high-level keywords from query
- Searches relationship vector store for matching edges
- Retrieves connected entity clusters
- Synthesizes response using global context

#### Target Implementation

**Location:** NOT IMPLEMENTED

#### Gap Analysis

The global query mode is one of LightRAG's signature features, enabling users to ask broad conceptual questions that span the entire knowledge base. Without this, EdgeQuake can only answer document-specific questions.

**Impact:**

- Users cannot ask "What are the main themes across all documents?"
- No ability to find cross-document relationships
- Missing competitive feature vs. other RAG systems

**Remediation:**

1. Implement relationship vector search in query engine
2. Add high-level keyword extraction using LLM
3. Implement global context aggregation algorithm
4. Create global prompt template

**Effort:** High (5-7 days)

---

### Feature: F-017 Query Mode: Mix

**Status:** ❌ Missing  
**Gap ID:** GAP-002  
**Severity:** P0 (Critical)

#### Source Implementation

**Location:** `lightrag/operate.py:kg_query()` with mode="mix"

**Description:** Mix mode combines local entity-centric retrieval with naive chunk retrieval, providing the best of both approaches.

**Behavior:**

- Performs local query (entities + relationships)
- Performs naive query (chunk retrieval)
- Deduplicates and merges context
- Generates unified response

#### Target Implementation

**Location:** NOT IMPLEMENTED

#### Gap Analysis

Mix mode is the default recommended mode in LightRAG as it provides the most comprehensive context for question answering.

**Impact:**

- Users get suboptimal answers without combined context
- No fallback when entity retrieval misses relevant chunks

**Remediation:**

1. Implement context merging algorithm
2. Add deduplication logic for overlapping sources
3. Integrate with token budget management

**Effort:** Medium (3-4 days, after global mode)

---

### Feature: F-010 Entity Deduplication

**Status:** ⚠️ Partial  
**Gap ID:** GAP-005  
**Severity:** P1 (High)

#### Source Implementation

**Location:** `lightrag/operate.py:merge_nodes_and_edges()`

**Behavior:**

- Normalizes entity names (uppercase, underscores)
- Merges duplicate entities by name
- Combines descriptions using LLM summarization
- Tracks source_ids across merges

#### Target Implementation

**Location:** `edgequake-pipeline/src/merger.rs`

**Current State:** Basic upsert exists but lacks:

- LLM-based description merging
- Source ID tracking
- Proper deduplication statistics

**Remediation:**

1. Add description comparison logic
2. Integrate LLM summarizer for conflicts
3. Implement source_id management

**Effort:** Medium (2-3 days)

---

### Feature: F-011 Description Summarization

**Status:** ⚠️ Partial  
**Gap ID:** GAP-006  
**Severity:** P1 (High)

#### Source Implementation

**Location:** `lightrag/operate.py:_handle_entity_relation_summary()`

**Behavior:**

- Map-reduce approach for long descriptions
- Respects token limits
- Configurable `force_llm_summary_on_merge` threshold
- Caches LLM responses

#### Target Implementation

**Location:** `edgequake-pipeline/src/summarizer.rs`

**Current State:** SimpleSummarizer exists but LLMSummarizer may not implement map-reduce pattern.

**Remediation:**

1. Implement iterative map-reduce summarization
2. Add token counting and chunking
3. Integrate with LLM cache

**Effort:** Medium (2-3 days)

---

### Feature: F-021 Keyword Extraction (HL/LL)

**Status:** ❌ Missing  
**Gap ID:** GAP-007  
**Severity:** P1 (High)

#### Source Implementation

**Location:** `lightrag/operate.py` with LLM prompt

**Description:** Extracts high-level (conceptual) and low-level (specific) keywords from queries to improve retrieval.

**Behavior:**

- Uses LLM to analyze query intent
- Separates broad concepts from specific terms
- Uses HL keywords for global search
- Uses LL keywords for local search

#### Target Implementation

**Location:** NOT IMPLEMENTED

**Remediation:**

1. Create keyword extraction prompt
2. Implement GPTKeywordExtractionFormat parser
3. Integrate with query engine

**Effort:** Low-Medium (1-2 days)

---

### Feature: F-025 Reranking Support

**Status:** ⚠️ Partial  
**Gap ID:** GAP-008  
**Severity:** P1 (High)

#### Source Implementation

**Location:** `lightrag/rerank.py` (576 lines)

**Behavior:**

- Supports Jina, Cohere, Aliyun rerankers
- Document chunking for token limits
- Score aggregation (max, mean, first)
- Retry with exponential backoff
- Configurable thresholds

#### Target Implementation

**Location:** `edgequake-api/src/handlers/query.rs`

**Current State:** Placeholder implementation with simulated rerank time. No actual reranker integration.

**Remediation:**

1. Implement reranker trait
2. Add Cohere/Jina reranker providers
3. Implement document chunking for long texts
4. Add score aggregation

**Effort:** Medium (3-4 days)

---

### Feature: F-026 Token Budget Management

**Status:** ⚠️ Partial  
**Gap ID:** GAP-009  
**Severity:** P1 (High)

#### Source Implementation

**Location:** `lightrag/base.py:QueryParam` and `operate.py`

**Behavior:**

- `max_entity_tokens` (default 6000)
- `max_relation_tokens` (default 8000)
- `max_total_tokens` (default 30000)
- Truncates context to fit LLM limits

#### Target Implementation

**Current State:** Basic truncation exists but not unified token budget system.

**Remediation:**

1. Implement QueryParams with token limits
2. Add truncation logic in context building
3. Ensure all query modes respect limits

**Effort:** Medium (2 days)

---

## SEC: Security & Multi-Tenancy

### Feature: F-066 Multi-tenancy Support

**Status:** ⚠️ Partial  
**Gap ID:** GAP-003  
**Severity:** P0 (Critical)

#### Source Implementation

**Location:** `lightrag/tenant_rag_manager.py`, `lightrag/api/routers/tenant_routes.py`

**Behavior:**

- Per-tenant working directories
- Per-KB isolation
- Tenant service for config
- LRU cache for RAG instances
- User access verification

#### Target Implementation

**Location:** `edgequake-auth/src/tenant.rs` (feature-gated)

**Current State:**

- Basic TenantContext exists
- Feature flag `multi-tenant`
- Missing: TenantRAGManager, instance caching, isolation verification

**Remediation:**

1. Implement TenantRAGManager equivalent
2. Add per-tenant EdgeQuake instance management
3. Implement working directory isolation
4. Add tenant verification middleware

**Effort:** High (5-7 days)

---

### Feature: F-067 Tenant RAG Manager

**Status:** ❌ Missing  
**Gap ID:** GAP-004  
**Severity:** P0 (Critical)

#### Source Implementation

**Location:** `lightrag/tenant_rag_manager.py` (330 lines)

**Features:**

- Instance caching with LRU eviction
- Double-check locking for thread safety
- Template RAG configuration inheritance
- User access verification
- Automatic cleanup

#### Target Implementation

**Location:** NOT IMPLEMENTED

**Remediation:**
Full implementation of tenant-aware EdgeQuake manager.

**Effort:** High (4-5 days)

---

## INTG: External Integrations

### Feature: F-040 Anthropic Provider

**Status:** ❌ Missing  
**Gap ID:** GAP-010  
**Severity:** P1 (High)

#### Source Implementation

**Location:** `lightrag/llm/anthropic.py`

**Behavior:**

- Claude model support
- Streaming support
- Tool use support

#### Target Implementation

**Location:** NOT IMPLEMENTED

**Remediation:**

1. Add anthropic-rs crate dependency
2. Implement LLMProvider trait for Anthropic
3. Add configuration options

**Effort:** Medium (2-3 days)

---

### Feature: F-049 Async Rate Limiting

**Status:** ❌ Missing  
**Gap ID:** GAP-011  
**Severity:** P1 (High)

#### Source Implementation

**Location:** `lightrag/utils.py:priority_limit_async_func_call()`

**Behavior:**

- Priority-based queue (1-10 levels)
- Configurable max concurrent calls
- Timeout handling
- Separate queues for LLM and embedding

#### Target Implementation

**Location:** NOT IMPLEMENTED

**Remediation:**

1. Implement PriorityAsyncQueue
2. Add rate limiter wrapper
3. Integrate with LLM provider calls

**Effort:** Medium (2-3 days)

---

## DATA: Storage Backends

### Feature: F-033 Neo4j Storage

**Status:** ❌ Missing  
**Gap ID:** GAP-012  
**Severity:** P2 (Medium)

#### Source Implementation

**Location:** `lightrag/kg/neo4j_impl.py`

#### Target Implementation

**Location:** NOT IMPLEMENTED

**Remediation:**

1. Add neo4j-rust crate
2. Implement GraphStorage trait
3. Add Cypher query templates

**Effort:** Medium (3-4 days)

---

### Feature: F-036 Milvus/Qdrant Storage

**Status:** ❌ Missing  
**Gap ID:** GAP-013  
**Severity:** P2 (Medium)

#### Source Implementation

**Location:** `lightrag/kg/milvus_impl.py`, `lightrag/kg/qdrant_impl.py`

#### Target Implementation

**Location:** NOT IMPLEMENTED

**Remediation:**

1. Add qdrant-client crate
2. Implement VectorStorage trait

**Effort:** Medium (2-3 days per backend)

---

## API: API Surface

### Feature: F-076 Document Scan/Rescan

**Status:** ❌ Missing  
**Gap ID:** GAP-014  
**Severity:** P2 (Medium)

#### Source Implementation

**Location:** `lightrag/api/routers/document_routes.py`

**Behavior:**

- Scan input directory for new files
- Track already processed files
- Background processing

#### Target Implementation

**Location:** NOT IMPLEMENTED

**Remediation:**
Implement directory scanning and file tracking.

**Effort:** Low (1-2 days)

---

## PERF: Performance Features

### Feature: F-047 LLM Response Cache

**Status:** ⚠️ Partial  
**Gap ID:** GAP-015  
**Severity:** P1 (High)

#### Source Implementation

**Location:** `lightrag/utils.py`, integrated with KV storage

**Behavior:**

- Hash-based cache key (prompt + params)
- Configurable enable/disable
- Separate flags for entity extraction vs queries
- Namespace isolation

#### Target Implementation

**Current State:** Basic structure exists but not fully integrated with LLM calls.

**Remediation:**

1. Implement cache wrapper for LLM calls
2. Add hash computation for prompts
3. Integrate with KV storage

**Effort:** Low (1-2 days)

---

## Feature Parity Matrix

### Legend

- ✅ Full parity
- ⚠️ Partial implementation
- ❌ Not implemented
- 🔄 Different approach (functionally equivalent)
- ⬆️ Target exceeds source
- ➖ Not applicable

### Matrix

| ID    | Feature                    | Category | Source | Target | Status | Gap ID  |
| ----- | -------------------------- | -------- | ------ | ------ | ------ | ------- |
| F-001 | Document Insert (sync)     | CORE     | ✅     | ✅     | ✅     | -       |
| F-002 | Document Insert (async)    | CORE     | ✅     | ✅     | ✅     | -       |
| F-003 | Batch Document Insert      | CORE     | ✅     | ✅     | ✅     | -       |
| F-004 | Token-based Chunking       | CORE     | ✅     | ✅     | ✅     | -       |
| F-005 | Chunk Overlap              | CORE     | ✅     | ✅     | ✅     | -       |
| F-006 | Custom Chunking Function   | CORE     | ✅     | ❌     | ❌     | GAP-016 |
| F-007 | Split by Character         | CORE     | ✅     | ❌     | ❌     | GAP-017 |
| F-008 | Entity Extraction (LLM)    | CORE     | ✅     | ✅     | ✅     | -       |
| F-009 | Relationship Extraction    | CORE     | ✅     | ✅     | ✅     | -       |
| F-010 | Entity Deduplication       | CORE     | ✅     | ⚠️     | ⚠️     | GAP-005 |
| F-011 | Description Summarization  | CORE     | ✅     | ⚠️     | ⚠️     | GAP-006 |
| F-012 | Max Gleaning               | CORE     | ✅     | ❌     | ❌     | GAP-018 |
| F-013 | Query Mode: Naive          | CORE     | ✅     | ✅     | ✅     | -       |
| F-014 | Query Mode: Local          | CORE     | ✅     | ✅     | ✅     | -       |
| F-015 | Query Mode: Global         | CORE     | ✅     | ❌     | ❌     | GAP-001 |
| F-016 | Query Mode: Hybrid         | CORE     | ✅     | ⚠️     | ⚠️     | GAP-019 |
| F-017 | Query Mode: Mix            | CORE     | ✅     | ❌     | ❌     | GAP-002 |
| F-018 | Query Mode: Bypass         | CORE     | ✅     | ❌     | ❌     | GAP-020 |
| F-019 | Streaming Query Response   | CORE     | ✅     | ✅     | ✅     | -       |
| F-020 | Conversation History       | CORE     | ✅     | ✅     | ✅     | -       |
| F-021 | Keyword Extraction (HL/LL) | CORE     | ✅     | ❌     | ❌     | GAP-007 |
| F-022 | Context-only Query         | CORE     | ✅     | ✅     | ✅     | -       |
| F-023 | Prompt-only Query          | CORE     | ✅     | ❌     | ❌     | GAP-021 |
| F-024 | Reference List Support     | CORE     | ✅     | ⚠️     | ⚠️     | GAP-022 |
| F-025 | Reranking Support          | PERF     | ✅     | ⚠️     | ⚠️     | GAP-008 |
| F-026 | Token Budget Management    | CORE     | ✅     | ⚠️     | ⚠️     | GAP-009 |
| F-027 | KV Storage Trait           | DATA     | ✅     | ✅     | ✅     | -       |
| F-028 | Vector Storage Trait       | DATA     | ✅     | ✅     | ✅     | -       |
| F-029 | Graph Storage Trait        | DATA     | ✅     | ✅     | ✅     | -       |
| F-030 | Document Status Storage    | DATA     | ✅     | ⚠️     | ⚠️     | GAP-023 |
| F-031 | Memory Storage Backend     | DATA     | ✅     | ✅     | ✅     | -       |
| F-032 | PostgreSQL Storage         | DATA     | ✅     | ✅     | ✅     | -       |
| F-033 | Neo4j Storage              | DATA     | ✅     | ❌     | ❌     | GAP-012 |
| F-034 | Redis Storage              | DATA     | ✅     | ❌     | ❌     | GAP-024 |
| F-035 | MongoDB Storage            | DATA     | ✅     | ❌     | ❌     | GAP-025 |
| F-036 | Milvus/Qdrant Storage      | DATA     | ✅     | ❌     | ❌     | GAP-013 |
| F-037 | FAISS Storage              | DATA     | ✅     | ❌     | ❌     | GAP-026 |
| F-038 | NanoVectorDB Storage       | DATA     | ✅     | ❌     | ❌     | GAP-027 |
| F-039 | OpenAI LLM Provider        | INTG     | ✅     | ✅     | ✅     | -       |
| F-040 | Anthropic Provider         | INTG     | ✅     | ❌     | ❌     | GAP-010 |
| F-041 | Azure OpenAI Provider      | INTG     | ✅     | ❌     | ❌     | GAP-028 |
| F-042 | Ollama Provider            | INTG     | ✅     | ⚠️     | ⚠️     | GAP-029 |
| F-043 | Gemini Provider            | INTG     | ✅     | ❌     | ❌     | GAP-030 |
| F-044 | Bedrock Provider           | INTG     | ✅     | ❌     | ❌     | GAP-031 |
| F-045 | HuggingFace Provider       | INTG     | ✅     | ❌     | ❌     | GAP-032 |
| F-046 | Jina Embedding Provider    | INTG     | ✅     | ❌     | ❌     | GAP-033 |
| F-047 | LLM Response Cache         | PERF     | ✅     | ⚠️     | ⚠️     | GAP-015 |
| F-048 | Embedding Cache            | PERF     | ✅     | ❌     | ❌     | GAP-034 |
| F-049 | Async Rate Limiting        | PERF     | ✅     | ❌     | ❌     | GAP-011 |
| F-050 | Priority Queue for LLM     | PERF     | ✅     | ❌     | ❌     | GAP-035 |
| F-051 | Document Upload API        | API      | ✅     | ✅     | ✅     | -       |
| F-052 | File Upload (Multipart)    | API      | ✅     | ✅     | ✅     | -       |
| F-053 | Batch File Upload          | API      | ✅     | ✅     | ✅     | -       |
| F-054 | Document List API          | API      | ✅     | ✅     | ✅     | -       |
| F-055 | Document Delete API        | API      | ✅     | ✅     | ✅     | -       |
| F-056 | Track Status API           | API      | ✅     | ✅     | ✅     | -       |
| F-057 | Query API                  | API      | ✅     | ✅     | ✅     | -       |
| F-058 | Streaming Query API        | API      | ✅     | ✅     | ✅     | -       |
| F-059 | Graph Labels API           | API      | ✅     | ⚠️     | ⚠️     | GAP-036 |
| F-060 | Graph Knowledge API        | API      | ✅     | ✅     | ✅     | -       |
| F-061 | Entity CRUD API            | API      | ✅     | ✅     | ✅     | -       |
| F-062 | Relationship CRUD API      | API      | ✅     | ✅     | ✅     | -       |
| F-063 | Entity Merge API           | API      | ✅     | ✅     | ✅     | -       |
| F-064 | Pipeline Status API        | API      | ✅     | ✅     | ✅     | -       |
| F-065 | Pipeline Cancel API        | API      | ✅     | ✅     | ✅     | -       |
| F-066 | Multi-tenancy Support      | SEC      | ✅     | ⚠️     | ⚠️     | GAP-003 |
| F-067 | Tenant RAG Manager         | SEC      | ✅     | ❌     | ❌     | GAP-004 |
| F-068 | Tenant/KB Isolation        | SEC      | ✅     | ⚠️     | ⚠️     | GAP-037 |
| F-069 | JWT Authentication         | SEC      | ✅     | ✅     | ✅     | -       |
| F-070 | API Key Authentication     | SEC      | ✅     | ✅     | ✅     | -       |
| F-071 | RBAC Permissions           | SEC      | ✅     | ✅     | ✅     | -       |
| F-072 | Ollama API Emulation       | API      | ✅     | ❌     | ❌     | GAP-038 |
| F-073 | Health Check API           | API      | ✅     | ✅     | ✅     | -       |
| F-074 | Metrics API                | OBS      | ⚠️     | ✅     | ⬆️     | -       |
| F-075 | Tracing/Logging            | OBS      | ✅     | ✅     | ✅     | -       |
| F-076 | Document Scan/Rescan       | API      | ✅     | ❌     | ❌     | GAP-014 |
| F-077 | Reprocess Failed Docs      | API      | ✅     | ❌     | ❌     | GAP-039 |
| F-078 | Docling Integration        | INTG     | ✅     | ❌     | ❌     | GAP-040 |

---

## Gap Registry

| Gap ID  | Feature                   | Severity | Type    | Status | Effort |
| ------- | ------------------------- | -------- | ------- | ------ | ------ |
| GAP-001 | Query Mode: Global        | P0       | MISSING | Open   | High   |
| GAP-002 | Query Mode: Mix           | P0       | MISSING | Open   | Medium |
| GAP-003 | Multi-tenancy Support     | P0       | PARTIAL | Open   | High   |
| GAP-004 | Tenant RAG Manager        | P0       | MISSING | Open   | High   |
| GAP-005 | Entity Deduplication      | P1       | PARTIAL | Open   | Medium |
| GAP-006 | Description Summarization | P1       | PARTIAL | Open   | Medium |
| GAP-007 | Keyword Extraction        | P1       | MISSING | Open   | Low    |
| GAP-008 | Reranking Support         | P1       | PARTIAL | Open   | Medium |
| GAP-009 | Token Budget Management   | P1       | PARTIAL | Open   | Medium |
| GAP-010 | Anthropic Provider        | P1       | MISSING | Open   | Medium |
| GAP-011 | Async Rate Limiting       | P1       | MISSING | Open   | Medium |
| GAP-012 | Neo4j Storage             | P2       | MISSING | Open   | Medium |
| GAP-013 | Milvus/Qdrant Storage     | P2       | MISSING | Open   | Medium |
| GAP-014 | Document Scan/Rescan      | P2       | MISSING | Open   | Low    |
| GAP-015 | LLM Response Cache        | P1       | PARTIAL | Open   | Low    |

---

## Appendices

### A. Source Code File Mapping

| Source (Python)       | Target (Rust)             | Notes              |
| --------------------- | ------------------------- | ------------------ |
| lightrag.py           | orchestrator.rs           | Core orchestration |
| base.py               | traits/\*.rs              | Storage interfaces |
| operate.py            | pipeline/_.rs, query/_.rs | Split into modules |
| types.py              | types/\*.rs               | Domain types       |
| rerank.py             | NOT IMPLEMENTED           | Reranking          |
| tenant_rag_manager.py | NOT IMPLEMENTED           | Multi-tenancy      |
| api/routers/\*.py     | handlers/\*.rs            | API handlers       |
| kg/\*.py              | adapters/\*.rs            | Storage backends   |
| llm/\*.py             | providers/\*.rs           | LLM providers      |

### B. Algorithm Parity Notes

1. **Entity Extraction**: Both use LLM with similar prompts; EdgeQuake may need prompt tuning
2. **Chunking**: Both support token-based chunking with overlap
3. **Vector Search**: Both use cosine similarity; threshold configurable
4. **Graph Traversal**: EdgeQuake uses petgraph; equivalent to NetworkX for basic ops

### C. Methodology Notes

This analysis was conducted by:

1. Inventorying all source files in both implementations
2. Extracting public functions, classes, and APIs
3. Mapping features across implementations
4. Deep-diving into P0/P1 gaps
5. Estimating remediation effort

Analysis followed the protocol defined in `specs/008-gap-analysis.md`.
