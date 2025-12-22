# API Comparison: EdgeQuake vs LightRAG

## Executive Summary

This document provides a comprehensive comparison between **EdgeQuake** (Rust/Axum implementation) and **LightRAG** (Python/FastAPI implementation) API endpoints. The analysis identifies feature parity, differences, and gaps between the two implementations.

**Status Date:** January 2025  
**EdgeQuake Version:** 0.1.0  
**LightRAG API Version:** Latest (Python)

---

## Overview

### EdgeQuake API

- **Framework:** Axum 0.8.8 (Rust)
- **Server Port:** 8080
- **Base Path:** `/api/v1`
- **Documentation:** OpenAPI/Swagger via utoipa 5.4.0
- **Authentication:** None currently implemented
- **Multi-tenant Support:** No (single workspace: "default")

### LightRAG API

- **Framework:** FastAPI (Python)
- **Server Port:** Configurable (default 8020)
- **Base Path:** `/api/v1`
- **Documentation:** OpenAPI/Swagger via FastAPI
- **Authentication:** OAuth2 + API Key (optional)
- **Multi-tenant Support:** Yes (optional multi-tenant mode)

---

## Core Feature Comparison Matrix

| Feature Category       | EdgeQuake  | LightRAG | Status            |
| ---------------------- | ---------- | -------- | ----------------- |
| Health/Status          | ✅ Yes     | ✅ Yes   | ✅ Parity         |
| Document Upload        | ✅ Yes     | ✅ Yes   | ⚠️ Partial Parity |
| Document Listing       | ✅ Yes     | ✅ Yes   | ⚠️ Partial Parity |
| Document Deletion      | ✅ Yes     | ✅ Yes   | ✅ Parity         |
| Text Insertion         | ❌ No      | ✅ Yes   | ❌ Missing        |
| Batch Text Insertion   | ❌ No      | ✅ Yes   | ❌ Missing        |
| Document Scanning      | ❌ No      | ✅ Yes   | ❌ Missing        |
| Query Execution        | ✅ Yes     | ✅ Yes   | ⚠️ Partial Parity |
| Streaming Query        | ✅ Yes     | ✅ Yes   | ✅ Parity         |
| Query with Context     | ⚠️ Partial | ✅ Yes   | ⚠️ Partial        |
| Graph Visualization    | ✅ Yes     | ✅ Yes   | ✅ Parity         |
| Node Search            | ✅ Yes     | ✅ Yes   | ✅ Parity         |
| Entity/Relation Edit   | ❌ No      | ✅ Yes   | ❌ Missing        |
| Entity/Relation Create | ❌ No      | ✅ Yes   | ❌ Missing        |
| Entity Merge           | ❌ No      | ✅ Yes   | ❌ Missing        |
| Multi-tenancy          | ❌ No      | ✅ Yes   | ❌ Missing        |
| Admin Functions        | ❌ No      | ✅ Yes   | ❌ Missing        |
| Membership Management  | ❌ No      | ✅ Yes   | ❌ Missing        |
| Ollama Proxy           | ❌ No      | ✅ Yes   | ❌ Missing        |
| Authentication         | ❌ No      | ✅ Yes   | ❌ Missing        |

**Legend:**

- ✅ Parity: Feature fully implemented and comparable
- ⚠️ Partial Parity: Feature exists but with differences
- ❌ Missing: Feature not implemented

---

## Detailed Endpoint Comparison

### 1. Health & Status Endpoints

#### EdgeQuake

```rust
GET  /health           // Health check with component status
GET  /ready            // Readiness probe
GET  /live             // Liveness probe
```

**Response Example:**

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "workspace_id": "default",
  "components": {
    "kv_storage": true,
    "vector_storage": true,
    "graph_storage": true,
    "llm_provider": true
  }
}
```

#### LightRAG

```python
GET  /health           // Basic health check
```

**Assessment:** ✅ **Parity** - EdgeQuake provides more detailed health information

---

### 2. Document Management Endpoints

#### 2.1 Document Upload

**EdgeQuake:**

```rust
POST /api/v1/documents
Body: {
  "content": "string",
  "title": "optional string",
  "metadata": "optional json"
}
Response: {
  "document_id": "uuid",
  "status": "processed",
  "chunk_count": 10,
  "entity_count": 25,
  "relationship_count": 40
}
```

**LightRAG:**

```python
POST /documents/upload
Form Data:
  - file: UploadFile
  - Uses background task processing
Response: {
  "status": "success|duplicated",
  "message": "...",
  "track_id": "upload-uuid"
}
```

**Key Differences:**

- EdgeQuake: JSON text input, synchronous processing
- LightRAG: File upload, background task processing with track_id
- LightRAG: Duplicate detection via doc_status storage
- LightRAG: Docling integration for PDF/Word parsing (optional)
- LightRAG: Path traversal sanitization

**Assessment:** ⚠️ **Partial Parity** - Different approaches (text vs file upload)

---

#### 2.2 Text Insertion (Direct Text)

**EdgeQuake:**

```
❌ Not Available
```

**LightRAG:**

```python
POST /documents/text
Body: {
  "text": "string",
  "file_source": "optional string"
}
Response: {
  "status": "success|duplicated",
  "message": "...",
  "track_id": "insert-uuid"
}

POST /documents/texts  // Batch version
Body: {
  "texts": ["string1", "string2"],
  "file_sources": ["optional", "list"]
}
```

**Assessment:** ❌ **Missing in EdgeQuake** - No direct text insertion endpoint

---

#### 2.3 Document Scanning

**EdgeQuake:**

```
❌ Not Available
```

**LightRAG:**

```python
POST /documents/scan
// Triggers background scan of input directory
Response: {
  "status": "scanning_started",
  "message": "...",
  "track_id": "scan-uuid"
}
```

**Assessment:** ❌ **Missing in EdgeQuake** - No directory scanning feature

---

#### 2.4 Document Listing

**EdgeQuake:**

```rust
GET /api/v1/documents
Response: {
  "documents": [
    {
      "id": "string",
      "title": "optional string",
      "chunk_count": 10
    }
  ],
  "total": 100,
  "page": 1,
  "page_size": 20
}
```

**LightRAG:**

```python
GET /documents/status
Query:
  - filter: "all|indexed|failed|processing|pending"
  - page, page_size, search
Response: {
  "status": "success",
  "data": {
    "documents": [...],
    "statistics": {
      "total": 100,
      "indexed": 80,
      "failed": 5,
      "processing": 10,
      "pending": 5
    },
    "pagination": {...}
  }
}
```

**Key Differences:**

- EdgeQuake: Simple document list with chunk counts
- LightRAG: Detailed status tracking (indexed/failed/processing/pending)
- LightRAG: Rich filtering and statistics
- LightRAG: Search capability

**Assessment:** ⚠️ **Partial Parity** - LightRAG provides more detailed status tracking

---

#### 2.5 Document Deletion

**EdgeQuake:**

```rust
DELETE /api/v1/documents/{document_id}
Response: 204 No Content
```

**LightRAG:**

```python
DELETE /documents/{doc_id}
Response: {
  "status": "success",
  "message": "...",
  "deleted_doc_id": "string"
}

DELETE /documents/file/{filename}  // Delete by filename
DELETE /documents/clear             // Clear all documents
DELETE /documents/failed            // Clear only failed documents
```

**Key Differences:**

- EdgeQuake: Delete by document ID only
- LightRAG: Multiple deletion methods (by ID, filename, all, failed only)
- LightRAG: Bulk delete operations

**Assessment:** ⚠️ **Partial Parity** - LightRAG provides more deletion options

---

### 3. Query Endpoints

#### 3.1 Standard Query

**EdgeQuake:**

```rust
POST /api/v1/query
Body: {
  "query": "string",
  "mode": "naive|local|global|hybrid|mix",  // optional, default: hybrid
  "context_only": false,                     // optional
  "max_results": 10                          // optional
}
Response: {
  "answer": "string",
  "mode": "hybrid",
  "sources": [
    {
      "source_type": "chunk|entity|relationship",
      "id": "string",
      "score": 0.95,
      "snippet": "optional string"
    }
  ],
  "stats": {
    "embedding_time_ms": 50,
    "retrieval_time_ms": 120,
    "generation_time_ms": 800,
    "total_time_ms": 970,
    "sources_retrieved": 15
  }
}
```

**LightRAG:**

```python
POST /query
Body: {
  "query": "string",
  "mode": "local|global|hybrid|naive|mix|bypass",
  "top_k": 60,                        // Number of entities/relations to retrieve
  "chunk_top_k": 5,                   // Number of chunks to retrieve
  "max_entity_tokens": 1000,          // Token budget for entities
  "max_relation_tokens": 1000,        // Token budget for relations
  "max_total_tokens": 4000,           // Total token budget
  "hl_keywords": [],                  // High-level keywords (optional)
  "ll_keywords": [],                  // Low-level keywords (optional)
  "conversation_history": [],         // Conversation context
  "user_prompt": "optional string",   // Custom prompt
  "enable_rerank": true,              // Enable chunk reranking
  "include_references": true,         // Include references in response
  "include_chunk_content": false      // Include full chunk content
}
Response: {
  "response": "string",
  "references": [...]  // Optional based on include_references
}
```

**Key Differences:**

| Feature              | EdgeQuake                             | LightRAG                         |
| -------------------- | ------------------------------------- | -------------------------------- |
| Query Modes          | 5 (naive, local, global, hybrid, mix) | 6 (+ bypass mode)                |
| Token Budget Control | ❌ No                                 | ✅ Yes (entity, relation, total) |
| Keyword Control      | ❌ No                                 | ✅ Yes (high-level, low-level)   |
| Conversation History | ❌ No                                 | ✅ Yes                           |
| Custom Prompts       | ❌ No                                 | ✅ Yes                           |
| Reranking            | ⚠️ Implicit                           | ✅ Explicit control              |
| Statistics           | ✅ Detailed timing                    | ❌ Not exposed                   |
| Source References    | ✅ Yes                                | ✅ Yes                           |

**Assessment:** ⚠️ **Partial Parity** - Both functional, but LightRAG provides more control

---

#### 3.2 Streaming Query

**EdgeQuake:**

```rust
POST /api/v1/query/stream
Body: {
  "query": "string",
  "mode": "optional string"
}
Response: Server-Sent Events (SSE)
```

**LightRAG:**

```python
POST /query/stream
Body: Same as /query with stream=true
Response: NDJSON streaming (newline-delimited JSON)

Stream format:
{"references": [...]}         // First chunk with references
{"response": "chunk1"}        // Response chunks
{"response": "chunk2"}
...
```

**Key Differences:**

- EdgeQuake: SSE (Server-Sent Events)
- LightRAG: NDJSON (Newline-Delimited JSON)
- LightRAG: References sent in first chunk
- EdgeQuake: Simpler streaming model

**Assessment:** ✅ **Parity** - Both support streaming with different formats

---

#### 3.3 Query with Context Data

**EdgeQuake:**

```
⚠️ Available via context_only parameter but not a separate endpoint
```

**LightRAG:**

```python
POST /query/data
Body: Same as /query
Response: {
  "status": "success",
  "message": "...",
  "data": {
    "entities": [...],
    "relationships": [...],
    "chunks": [...],
    "references": [...]
  },
  "metadata": {
    "mode": "hybrid",
    "hl_keywords": [...],
    "ll_keywords": [...],
    "retrieval_time_ms": 150
  }
}
```

**Assessment:** ⚠️ **Partial** - EdgeQuake returns sources but no dedicated endpoint

---

### 4. Knowledge Graph Endpoints

#### 4.1 Graph Visualization

**EdgeQuake:**

```rust
GET /api/v1/graph
Query:
  - start_node: "optional string"
  - depth: 2 (default)
  - max_nodes: 100 (default)
Response: {
  "nodes": [
    {
      "id": "string",
      "label": "string",
      "node_type": "string",
      "description": "string",
      "degree": 5,
      "properties": {}
    }
  ],
  "edges": [
    {
      "source": "string",
      "target": "string",
      "edge_type": "string",
      "weight": 1.0,
      "properties": {}
    }
  ],
  "is_truncated": false,
  "total_nodes": 500,
  "total_edges": 1200
}
```

**LightRAG:**

```python
GET /graphs
Query:
  - page, page_size, search
Response: {
  "entities": [...],
  "relationships": [...],
  "pagination": {...}
}
```

**Assessment:** ✅ **Parity** - Both support graph visualization with different approaches

---

#### 4.2 Node Operations

**EdgeQuake:**

```rust
GET /api/v1/graph/nodes/{node_id}  // Get specific node
GET /api/v1/graph/labels/search    // Search labels
  Query: q="search", limit=10
```

**LightRAG:**

```python
GET /graph/label/list              // List all labels
GET /graph/label/popular           // Get popular labels
GET /graph/label/search            // Search labels
  Query: query="search", limit=20
GET /graph/entity/exists           // Check if entity exists
  Query: entity_name="NAME"
```

**Assessment:** ✅ **Parity** - Similar functionality with different endpoints

---

#### 4.3 Graph Editing (Manual Knowledge Entry)

**EdgeQuake:**

```
❌ Not Available
```

**LightRAG:**

```python
// Entity Operations
POST /graph/entity/create
Body: {
  "entity_name": "string",
  "entity_type": "string",
  "description": "string",
  "source_id": "string"
}

POST /graph/entity/edit
Body: {
  "entity_name": "string",
  "entity_type": "optional string",
  "description": "optional string"
}

POST /graph/entities/merge
Body: {
  "source_entity": "string",
  "target_entity": "string"
}

// Relationship Operations
POST /graph/relation/create
Body: {
  "src_id": "string",
  "tgt_id": "string",
  "keywords": "string",
  "weight": 0.9,
  "description": "string",
  "source_id": "string"
}

POST /graph/relation/edit
Body: {
  "src_id": "string",
  "tgt_id": "string",
  "keywords": "optional string",
  "weight": "optional float",
  "description": "optional string"
}
```

**Assessment:** ❌ **Missing in EdgeQuake** - No manual graph editing capabilities

---

### 5. Multi-Tenancy & Admin Endpoints

**EdgeQuake:**

```
❌ Not Available - Single workspace "default" only
```

**LightRAG:**

```python
// Tenant Management
GET    /tenants                    // List all tenants (paginated)
GET    /tenants/me                 // Get current user's tenant
POST   /tenants                    // Create tenant
POST   /tenants/select             // Select active tenant

// Knowledge Base Management
GET    /knowledge-bases            // List knowledge bases
GET    /knowledge-bases/{kb_id}    // Get specific KB
PUT    /knowledge-bases/{kb_id}    // Update KB
DELETE /knowledge-bases/{kb_id}    // Delete KB
GET    /knowledge-bases/{kb_id}/stats  // KB statistics

// Admin Functions
POST   /admin/tenants              // Admin: Create tenant
GET    /admin/tenants              // Admin: List all tenants (full access)

// Membership Management
POST   /memberships                // Add user to tenant
GET    /memberships/{tenant_id}    // List tenant members
PUT    /memberships/{tenant_id}/users/{user_id}  // Update membership
DELETE /memberships/{tenant_id}/users/{user_id}  // Remove membership
GET    /users/me/tenants           // List user's tenants
```

**Assessment:** ❌ **Missing in EdgeQuake** - Entire multi-tenancy layer not implemented

---

### 6. Authentication & Authorization

**EdgeQuake:**

```
❌ Not Implemented - No authentication
```

**LightRAG:**

```python
POST /token                        // OAuth2 token login
// Dependencies: OAuth2 + API Key support
// Authentication: combined_auth (API key or OAuth2)
// Per-endpoint authorization via Depends(combined_auth)
```

**Assessment:** ❌ **Missing in EdgeQuake** - No authentication mechanism

---

### 7. Ollama Proxy API

**EdgeQuake:**

```
❌ Not Available
```

**LightRAG:**

```python
GET  /api/tags                     // List Ollama models
POST /api/generate                 // Generate completion
POST /api/chat                     // Chat completion
POST /api/embeddings               // Generate embeddings
```

**Assessment:** ❌ **Missing in EdgeQuake** - No Ollama proxy functionality

---

## Query Parameter Comparison

### Query Modes

| Mode   | EdgeQuake | LightRAG | Description                    |
| ------ | --------- | -------- | ------------------------------ |
| naive  | ✅ Yes    | ✅ Yes   | Simple vector search           |
| local  | ✅ Yes    | ✅ Yes   | Entity-focused retrieval       |
| global | ✅ Yes    | ✅ Yes   | Relationship-focused retrieval |
| hybrid | ✅ Yes    | ✅ Yes   | Combined entity + relationship |
| mix    | ✅ Yes    | ✅ Yes   | Mixed retrieval strategy       |
| bypass | ❌ No     | ✅ Yes   | Direct LLM without RAG         |

### Advanced Query Features

| Feature               | EdgeQuake   | LightRAG           |
| --------------------- | ----------- | ------------------ |
| Token Budget Control  | ❌          | ✅                 |
| Max Entity Tokens     | ❌          | ✅                 |
| Max Relation Tokens   | ❌          | ✅                 |
| Max Total Tokens      | ❌          | ✅                 |
| High-Level Keywords   | ❌          | ✅                 |
| Low-Level Keywords    | ❌          | ✅                 |
| Conversation History  | ❌          | ✅                 |
| Custom User Prompt    | ❌          | ✅                 |
| Rerank Control        | ⚠️ Implicit | ✅ Explicit        |
| Include References    | ✅ Yes      | ✅ Yes             |
| Include Chunk Content | ❌          | ✅                 |
| Context Only Mode     | ✅ Yes      | ⚠️ Via /query/data |

---

## Data Model Comparison

### Document Processing

**EdgeQuake:**

```rust
Input: content (string), title (optional), metadata (optional)
Processing: Synchronous pipeline
Output: chunk_count, entity_count, relationship_count
Storage: KV storage for chunks, Graph for entities/relationships
```

**LightRAG:**

```python
Input: file (upload) or text (string), file_source (optional)
Processing: Async background tasks with track_id
Output: track_id for status polling
Storage: doc_status tracking, KV storage, Vector DB, Graph DB
Status: pending → processing → indexed/failed
Features: Duplicate detection, Docling parsing, sanitization
```

### Query Response

**EdgeQuake:**

```rust
{
  "answer": "string",
  "mode": "string",
  "sources": [/* source references */],
  "stats": {/* timing statistics */}
}
```

**LightRAG:**

```python
{
  "response": "string",
  "references": [/* optional references */]
}
// OR for /query/data:
{
  "status": "success",
  "data": {/* entities, relationships, chunks */},
  "metadata": {/* mode, keywords, timing */}
}
```

---

## Missing Features in EdgeQuake

### High Priority (Core RAG Functionality)

1. **Background Task Processing**

   - LightRAG uses background tasks with track_id for async processing
   - EdgeQuake processes documents synchronously
   - Impact: Blocks API calls for large documents

2. **Document Status Tracking**

   - LightRAG: Detailed status (pending → processing → indexed/failed)
   - EdgeQuake: No status tracking beyond success/failure
   - Impact: No visibility into processing state

3. **Direct Text Insertion**

   - LightRAG: POST /documents/text and /documents/texts
   - EdgeQuake: Only via /documents with JSON content
   - Impact: Less flexible document input

4. **Token Budget Control**

   - LightRAG: max_entity_tokens, max_relation_tokens, max_total_tokens
   - EdgeQuake: No token budget controls
   - Impact: Less control over LLM costs and context window

5. **Conversation History**
   - LightRAG: Full conversation history support
   - EdgeQuake: Stateless queries only
   - Impact: No multi-turn conversations

### Medium Priority (Advanced Features)

6. **High-Level/Low-Level Keywords**

   - LightRAG: Explicit keyword control for retrieval
   - EdgeQuake: No keyword parameters
   - Impact: Less fine-grained retrieval control

7. **Custom Prompts**

   - LightRAG: user_prompt parameter
   - EdgeQuake: No custom prompt support
   - Impact: Less flexibility for specialized use cases

8. **Graph Editing Operations**

   - LightRAG: Entity/Relationship create, edit, merge
   - EdgeQuake: No manual graph editing
   - Impact: Cannot manually correct or enhance knowledge graph

9. **Bulk Operations**

   - LightRAG: Batch text insertion, clear all documents, clear failed
   - EdgeQuake: Individual operations only
   - Impact: Less efficient for bulk operations

10. **Document Scanning**
    - LightRAG: Automatic directory scanning
    - EdgeQuake: Manual document upload only
    - Impact: No automatic ingestion from filesystem

### Low Priority (Optional Features)

11. **Multi-Tenancy**

    - LightRAG: Full multi-tenant support with tenant/KB management
    - EdgeQuake: Single workspace "default"
    - Impact: Cannot serve multiple isolated tenants

12. **Authentication**

    - LightRAG: OAuth2 + API Key
    - EdgeQuake: No authentication
    - Impact: Not production-ready for public deployment

13. **Admin Functions**

    - LightRAG: Admin routes for tenant management
    - EdgeQuake: No admin functionality
    - Impact: No administrative controls

14. **Ollama Proxy**

    - LightRAG: Built-in Ollama API proxy
    - EdgeQuake: No Ollama proxy
    - Impact: Must connect directly to Ollama

15. **Membership Management**
    - LightRAG: User-tenant-role management
    - EdgeQuake: No membership system
    - Impact: No access control

---

## Recommendations for EdgeQuake v1.1

### Phase 1: Core RAG Enhancements

1. **Async Background Processing**

   - Implement background task queue (tokio tasks + channels)
   - Add track_id to responses for status polling
   - Create GET /api/v1/tasks/{track_id} endpoint

2. **Document Status Tracking**

   - Add doc_status table/collection
   - Track: pending, processing, indexed, failed
   - Implement GET /api/v1/documents/status endpoint

3. **Query Enhancements**

   - Add token budget parameters (max_entity_tokens, max_relation_tokens, max_total_tokens)
   - Implement conversation history support
   - Add hl_keywords/ll_keywords parameters
   - Add user_prompt parameter

4. **Direct Text Insertion**
   - POST /api/v1/documents/text endpoint
   - POST /api/v1/documents/texts for batch

### Phase 2: Graph Management

5. **Graph Editing Operations**

   - POST /api/v1/graph/entities (create)
   - PUT /api/v1/graph/entities/{id} (edit)
   - POST /api/v1/graph/entities/merge (merge duplicates)
   - POST /api/v1/graph/relationships (create)
   - PUT /api/v1/graph/relationships/{id} (edit)

6. **Bulk Operations**
   - DELETE /api/v1/documents/all
   - DELETE /api/v1/documents/failed
   - POST /api/v1/documents/scan (directory scanning)

### Phase 3: Production Readiness

7. **Authentication & Authorization**

   - Implement JWT-based authentication
   - Add API key support
   - Create auth middleware

8. **Multi-Tenancy** (Optional)

   - Add tenant_id to all operations
   - Implement tenant isolation in storage
   - Create tenant management endpoints

9. **Observability**
   - Add OpenTelemetry tracing
   - Metrics export (Prometheus)
   - Structured logging

---

## Performance Comparison

### Strengths

**EdgeQuake:**

- ✅ Native compiled (Rust) - faster execution
- ✅ Lower memory footprint
- ✅ Better concurrency (Tokio async runtime)
- ✅ Type safety at compile time

**LightRAG:**

- ✅ Mature Python ecosystem (Docling, rich libraries)
- ✅ Easier to extend and customize
- ✅ Faster development iteration
- ✅ More comprehensive feature set

### Weaknesses

**EdgeQuake:**

- ⚠️ Less feature-complete
- ⚠️ Smaller ecosystem for RAG-specific tools
- ⚠️ Longer development cycle for new features

**LightRAG:**

- ⚠️ Higher memory usage (Python runtime)
- ⚠️ GIL limitations for CPU-bound tasks
- ⚠️ Slower startup time

---

## API Design Quality

### EdgeQuake Strengths

- ✅ Clean RESTful design
- ✅ Consistent error responses
- ✅ Good OpenAPI documentation
- ✅ Type-safe request/response models
- ✅ Clear endpoint structure

### LightRAG Strengths

- ✅ Comprehensive endpoint coverage
- ✅ Rich query parameters
- ✅ Good separation of concerns (routers)
- ✅ Background task handling
- ✅ Multi-tenant aware design

### Areas for Improvement

**EdgeQuake:**

- ⚠️ No versioning strategy documented
- ⚠️ Missing rate limiting
- ⚠️ No pagination on graph endpoints (only documents)
- ⚠️ No filtering/sorting options

**LightRAG:**

- ⚠️ Inconsistent response formats (some use "status", some don't)
- ⚠️ Many optional parameters (can be overwhelming)
- ⚠️ Some endpoints lack clear error codes

---

## Migration Path (LightRAG → EdgeQuake)

### Directly Compatible Endpoints

1. **Health Check:** Direct mapping
2. **Document Upload:** Convert file upload to text content
3. **Query:** Basic parameters compatible (query, mode)
4. **Graph Visualization:** Similar response structure

### Requires Adaptation

1. **Document Management:**

   - Track background tasks externally
   - Poll document list for status
   - No duplicate detection built-in

2. **Advanced Queries:**

   - Remove token budget parameters
   - Remove conversation_history
   - Remove hl_keywords/ll_keywords
   - Simplify to basic mode selection

3. **Graph Operations:**
   - Read-only access
   - Cannot manually edit graph
   - Must re-process documents to update

### Not Supported

1. Multi-tenancy features
2. Authentication/Authorization
3. Admin functions
4. Membership management
5. Ollama proxy
6. Manual graph editing

---

## Conclusion

### Summary

**EdgeQuake** provides a **solid foundation** for core RAG functionality with excellent performance characteristics due to its Rust implementation. However, it **lacks many advanced features** present in LightRAG, particularly around:

- Background task processing
- Token budget controls
- Conversation history
- Graph editing
- Multi-tenancy
- Authentication

**LightRAG** offers a **feature-rich** API with comprehensive document management, advanced query controls, and production-ready multi-tenancy support, at the cost of potentially higher resource usage.

### Recommendations

**For New Deployments:**

- **Choose EdgeQuake** if: Performance and resource efficiency are critical, basic RAG is sufficient
- **Choose LightRAG** if: Feature richness, multi-tenancy, and faster customization are priorities

**For EdgeQuake Development:**

- **Prioritize:** Background tasks, token budgets, conversation history (Phase 1)
- **Consider:** Graph editing, bulk operations (Phase 2)
- **Future:** Authentication, multi-tenancy, observability (Phase 3)

### Feature Parity Status

**Current State:**

- Core Endpoints: ~60% parity
- Advanced Features: ~30% parity
- Production Features: ~10% parity

**Target v1.1:**

- Core Endpoints: 90% parity
- Advanced Features: 70% parity
- Production Features: 50% parity

---

## Appendix: Quick Reference

### EdgeQuake Endpoint Summary (11 endpoints)

```
Health:
  GET  /health
  GET  /ready
  GET  /live

Documents:
  POST   /api/v1/documents          # Upload
  GET    /api/v1/documents          # List
  GET    /api/v1/documents/{id}     # Get
  DELETE /api/v1/documents/{id}     # Delete

Query:
  POST /api/v1/query                # Execute
  POST /api/v1/query/stream         # Stream

Graph:
  GET /api/v1/graph                 # Visualize
  GET /api/v1/graph/nodes/{id}      # Get node
  GET /api/v1/graph/labels/search   # Search labels
```

### LightRAG Endpoint Summary (40+ endpoints)

```
Health:
  GET /health

Documents (13 endpoints):
  POST   /documents/scan
  POST   /documents/upload
  POST   /documents/text
  POST   /documents/texts
  DELETE /documents/{doc_id}
  GET    /documents/status
  GET    /documents/list
  DELETE /documents/file/{filename}
  POST   /documents/clear-cache
  DELETE /documents/clear
  DELETE /documents/failed
  GET    /documents/stats
  POST   /documents/reindex-failed

Query (3 endpoints):
  POST /query
  POST /query/stream
  POST /query/data

Graph (10 endpoints):
  GET  /graph/label/list
  GET  /graph/label/popular
  GET  /graph/label/search
  GET  /graphs
  GET  /graph/entity/exists
  POST /graph/entity/edit
  POST /graph/relation/edit
  POST /graph/entity/create
  POST /graph/relation/create
  POST /graph/entities/merge

Tenant (8 endpoints):
  GET    /tenants
  GET    /tenants/me
  POST   /tenants
  POST   /tenants/select
  GET    /knowledge-bases
  GET    /knowledge-bases/{kb_id}
  PUT    /knowledge-bases/{kb_id}
  DELETE /knowledge-bases/{kb_id}

Admin (2 endpoints):
  POST /admin/tenants
  GET  /admin/tenants

Membership (5 endpoints):
  POST   /memberships
  GET    /memberships/{tenant_id}
  PUT    /memberships/{tenant_id}/users/{user_id}
  DELETE /memberships/{tenant_id}/users/{user_id}
  GET    /users/me/tenants

Ollama (4 endpoints):
  GET  /api/tags
  POST /api/generate
  POST /api/chat
  POST /api/embeddings
```

---

**Document Version:** 1.0  
**Last Updated:** January 2025  
**Authors:** EdgeQuake Development Team
