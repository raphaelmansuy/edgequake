# EdgeQuake Use Cases Registry

> Central registry of all use cases supported by EdgeQuake.
> Use UCXXXX references in API handlers for traceability.

**Version**: 1.2.0 | **Last Updated**: 2026-01-09

---

## Quick Reference Index

| Category                                                   | ID Range      | Count |
| ---------------------------------------------------------- | ------------- | ----- |
| [Document Management](#document-management-uc00xx)         | UC0001-UC0020 | 8     |
| [Knowledge Graph](#knowledge-graph-uc01xx)                 | UC0101-UC0120 | 7     |
| [Query Execution](#query-execution-uc02xx)                 | UC0201-UC0220 | 8     |
| [Workspace Management](#workspace-management-uc03xx)       | UC0301-UC0320 | 5     |
| [Conversation Management](#conversation-management-uc04xx) | UC0401-UC0420 | 6     |
| [Administration](#administration-uc05xx)                   | UC0501-UC0520 | 4     |
| [WebUI Interactions](#webui-interactions-uc06xx)           | UC0601-UC0620 | 10    |
| [PDF Processing](#pdf-processing-uc10xx)                   | UC1001-UC1020 | 8     |

---

## Document Management (UC00XX)

### UC0001 - Upload Text Document

| Attribute            | Value                                   |
| -------------------- | --------------------------------------- |
| **ID**               | UC0001                                  |
| **Name**             | Upload Text Document                    |
| **Actor**            | API Client / WebUI User                 |
| **Preconditions**    | Authenticated, valid workspace selected |
| **Endpoint**         | `POST /api/v1/documents`                |
| **Related Features** | FEAT0001, FEAT0401                      |
| **Related Rules**    | BR0001, BR0008                          |

**Steps:**

1. Client sends JSON with `content` and optional `document_id`
2. Server validates content is non-empty
3. Server generates document ID if not provided
4. Document created with status `pending`
5. Background task queued for processing
6. Response returned with `track_id` for status polling

**Success Response:**

```json
{
  "document_id": "doc_abc123",
  "track_id": "task_xyz789",
  "status": "pending"
}
```

**Error Scenarios:**
| Error | HTTP Code | Cause |
|-------|-----------|-------|
| Empty content | 400 | BR0105 violation |
| Duplicate ID | 409 | BR0001 violation |
| Rate limited | 429 | BR0204 violation |

---

### UC0002 - Upload File Document

| Attribute            | Value                                   |
| -------------------- | --------------------------------------- |
| **ID**               | UC0002                                  |
| **Name**             | Upload File Document                    |
| **Actor**            | API Client / WebUI User                 |
| **Preconditions**    | Authenticated, valid workspace selected |
| **Endpoint**         | `POST /api/v1/documents/upload`         |
| **Related Features** | FEAT0001, FEAT0402, FEAT0501            |
| **Related Rules**    | BR0402, BR0403                          |

**Steps:**

1. Client sends multipart form with file
2. Server validates file type (PDF, TXT, MD)
3. Server validates file size (max 100MB)
4. PDF files → Extract text via FEAT0501
5. Document created with extracted content
6. Processing queued same as UC0001

**Supported File Types:**
| Extension | MIME Type | Max Size |
|-----------|-----------|----------|
| .pdf | application/pdf | 100MB |
| .txt | text/plain | 50MB |
| .md | text/markdown | 50MB |

**Error Scenarios:**
| Error | HTTP Code | Cause |
|-------|-----------|-------|
| Invalid type | 415 | BR0402 violation |
| File too large | 413 | BR0403 violation |
| PDF extraction failed | 422 | Corrupted PDF |

---

### UC0003 - List Documents

| Attribute            | Value                                   |
| -------------------- | --------------------------------------- |
| **ID**               | UC0003                                  |
| **Name**             | List Documents in Workspace             |
| **Actor**            | API Client / WebUI User                 |
| **Preconditions**    | Authenticated, valid workspace selected |
| **Endpoint**         | `GET /api/v1/documents`                 |
| **Related Features** | FEAT0001                                |
| **Related Rules**    | BR0201                                  |

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| page | int | 1 | Page number |
| per_page | int | 20 | Items per page |
| status | string | all | Filter by status |

**Response:**

```json
{
  "documents": [
    {
      "id": "doc_abc123",
      "title": "Document Title",
      "status": "completed",
      "created_at": "2026-01-09T10:00:00Z",
      "chunk_count": 15,
      "entity_count": 42
    }
  ],
  "pagination": {
    "page": 1,
    "per_page": 20,
    "total": 150
  }
}
```

---

### UC0004 - Get Document Details

| Attribute            | Value                                 |
| -------------------- | ------------------------------------- |
| **ID**               | UC0004                                |
| **Name**             | Get Document Details                  |
| **Actor**            | API Client / WebUI User               |
| **Preconditions**    | Authenticated, document exists        |
| **Endpoint**         | `GET /api/v1/documents/{document_id}` |
| **Related Features** | FEAT0001, FEAT0011                    |
| **Related Rules**    | BR0201, BR0203                        |

**Response:**

```json
{
  "id": "doc_abc123",
  "title": "Document Title",
  "status": "completed",
  "content_preview": "First 500 characters...",
  "metadata": {
    "source_file": "report.pdf",
    "file_size": 1048576,
    "page_count": 10
  },
  "stats": {
    "chunk_count": 15,
    "entity_count": 42,
    "relationship_count": 28
  },
  "created_at": "2026-01-09T10:00:00Z",
  "processed_at": "2026-01-09T10:05:00Z"
}
```

---

### UC0005 - Delete Document

| Attribute            | Value                                    |
| -------------------- | ---------------------------------------- |
| **ID**               | UC0005                                   |
| **Name**             | Delete Document                          |
| **Actor**            | API Client / WebUI User                  |
| **Preconditions**    | Authenticated, document exists           |
| **Endpoint**         | `DELETE /api/v1/documents/{document_id}` |
| **Related Features** | FEAT0001                                 |
| **Related Rules**    | BR0201, BR0007                           |

**Steps:**

1. Validate document belongs to tenant/workspace
2. Remove document from KV storage
3. Remove associated chunks and embeddings
4. Mark entities/relationships for cleanup (may be shared)
5. Return success

**Cascading Deletions:**

```
Document
├── Chunks (all deleted)
├── Chunk Embeddings (all deleted)
├── Entities (deleted if orphaned)
└── Relationships (deleted if orphaned)
```

---

### UC0006 - Re-process Failed Document

| Attribute            | Value                                            |
| -------------------- | ------------------------------------------------ |
| **ID**               | UC0006                                           |
| **Name**             | Re-process Failed Document                       |
| **Actor**            | API Client / WebUI User                          |
| **Preconditions**    | Document exists with status `failed`             |
| **Endpoint**         | `POST /api/v1/documents/{document_id}/reprocess` |
| **Related Features** | FEAT0001, FEAT0019                               |
| **Related Rules**    | BR0008                                           |

**Steps:**

1. Verify document status is `failed`
2. Reset status to `pending`
3. Clear previous partial results
4. Queue new processing task
5. Return new `track_id`

---

### UC0007 - Get Processing Status

| Attribute            | Value                          |
| -------------------- | ------------------------------ |
| **ID**               | UC0007                         |
| **Name**             | Get Document Processing Status |
| **Actor**            | API Client / WebUI User        |
| **Preconditions**    | Valid track_id                 |
| **Endpoint**         | `GET /api/v1/tasks/{track_id}` |
| **Related Features** | FEAT0012, FEAT0406             |
| **Related Rules**    | None                           |

**Response:**

```json
{
  "track_id": "task_xyz789",
  "status": "processing",
  "progress": {
    "stage": "entity_extraction",
    "current": 12,
    "total": 15,
    "percentage": 80
  },
  "cost": {
    "prompt_tokens": 45000,
    "completion_tokens": 8000,
    "estimated_cost_usd": 0.35
  }
}
```

---

### UC0008 - Batch Upload Documents

| Attribute            | Value                           |
| -------------------- | ------------------------------- |
| **ID**               | UC0008                          |
| **Name**             | Batch Upload Multiple Documents |
| **Actor**            | API Client                      |
| **Preconditions**    | Authenticated, valid workspace  |
| **Endpoint**         | `POST /api/v1/documents/batch`  |
| **Related Features** | FEAT0001, FEAT0401              |
| **Related Rules**    | BR0303                          |

**Request:**

```json
{
  "documents": [
    { "content": "Document 1 content..." },
    { "content": "Document 2 content..." }
  ]
}
```

**Response:**

```json
{
  "results": [
    { "document_id": "doc_1", "track_id": "task_1", "status": "pending" },
    { "document_id": "doc_2", "track_id": "task_2", "status": "pending" }
  ],
  "batch_track_id": "batch_abc"
}
```

---

## Knowledge Graph (UC01XX)

### UC0101 - View Graph Visualization

| Attribute            | Value                              |
| -------------------- | ---------------------------------- |
| **ID**               | UC0101                             |
| **Name**             | View Knowledge Graph Visualization |
| **Actor**            | WebUI User                         |
| **Preconditions**    | Authenticated, workspace has data  |
| **Endpoint**         | `GET /api/v1/graph`                |
| **Related Features** | FEAT0405, FEAT0603                 |
| **Related Rules**    | BR0201                             |

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| limit | int | 100 | Max nodes to return |
| depth | int | 2 | Relationship traversal depth |
| center | string | null | Center entity name |

**Response:**

```json
{
  "nodes": [
    { "id": "ENTITY_A", "label": "Entity A", "type": "PERSON", "size": 5 }
  ],
  "edges": [
    { "source": "ENTITY_A", "target": "ENTITY_B", "label": "WORKS_FOR" }
  ],
  "stats": {
    "total_nodes": 1500,
    "total_edges": 2800,
    "returned_nodes": 100,
    "returned_edges": 150
  }
}
```

---

### UC0102 - Search Entities

| Attribute            | Value                                       |
| -------------------- | ------------------------------------------- |
| **ID**               | UC0102                                      |
| **Name**             | Search Entities by Name                     |
| **Actor**            | API Client / WebUI User                     |
| **Preconditions**    | Authenticated                               |
| **Endpoint**         | `GET /api/v1/graph/entities?search={query}` |
| **Related Features** | FEAT0405                                    |
| **Related Rules**    | BR0003                                      |

**Response:**

```json
{
  "entities": [
    {
      "name": "SARAH_CHEN",
      "type": "PERSON",
      "description": "AI researcher at...",
      "source_count": 3
    }
  ],
  "total": 5
}
```

---

### UC0103 - Get Entity Details

| Attribute            | Value                                      |
| -------------------- | ------------------------------------------ |
| **ID**               | UC0103                                     |
| **Name**             | Get Entity Details and Relationships       |
| **Actor**            | API Client / WebUI User                    |
| **Preconditions**    | Entity exists                              |
| **Endpoint**         | `GET /api/v1/graph/entities/{entity_name}` |
| **Related Features** | FEAT0405, FEAT0011                         |
| **Related Rules**    | BR0201                                     |

**Response:**

```json
{
  "name": "SARAH_CHEN",
  "type": "PERSON",
  "description": "Dr. Sarah Chen is...",
  "relationships": {
    "outgoing": [{ "target": "MIT", "type": "WORKS_AT", "description": "..." }],
    "incoming": [
      {
        "source": "JOHN_DOE",
        "type": "COLLABORATES_WITH",
        "description": "..."
      }
    ]
  },
  "sources": [
    {
      "document_id": "doc_123",
      "chunk_id": "chunk_45",
      "line_start": 10,
      "line_end": 15
    }
  ]
}
```

---

### UC0104 - Create Manual Entity

| Attribute            | Value                         |
| -------------------- | ----------------------------- |
| **ID**               | UC0104                        |
| **Name**             | Create Entity Manually        |
| **Actor**            | API Client / WebUI User       |
| **Preconditions**    | Authenticated                 |
| **Endpoint**         | `POST /api/v1/graph/entities` |
| **Related Features** | FEAT0405                      |
| **Related Rules**    | BR0003                        |

**Request:**

```json
{
  "name": "New Entity",
  "type": "ORGANIZATION",
  "description": "Description of the entity..."
}
```

---

### UC0105 - Create Manual Relationship

| Attribute            | Value                              |
| -------------------- | ---------------------------------- |
| **ID**               | UC0105                             |
| **Name**             | Create Relationship Manually       |
| **Actor**            | API Client / WebUI User            |
| **Preconditions**    | Both entities exist                |
| **Endpoint**         | `POST /api/v1/graph/relationships` |
| **Related Features** | FEAT0405                           |
| **Related Rules**    | BR0004                             |

**Request:**

```json
{
  "source": "ENTITY_A",
  "target": "ENTITY_B",
  "relationship_type": "RELATED_TO",
  "description": "How they are related..."
}
```

---

### UC0106 - Get Graph Statistics

| Attribute            | Value                          |
| -------------------- | ------------------------------ |
| **ID**               | UC0106                         |
| **Name**             | Get Knowledge Graph Statistics |
| **Actor**            | API Client / WebUI User        |
| **Preconditions**    | Authenticated                  |
| **Endpoint**         | `GET /api/v1/graph/stats`      |
| **Related Features** | FEAT0405                       |
| **Related Rules**    | BR0201                         |

**Response:**

```json
{
  "entity_count": 1500,
  "relationship_count": 2800,
  "entity_types": {
    "PERSON": 450,
    "ORGANIZATION": 300,
    "CONCEPT": 750
  },
  "relationship_types": {
    "WORKS_AT": 200,
    "RELATED_TO": 1500,
    "COLLABORATES_WITH": 300
  },
  "avg_relationships_per_entity": 1.87,
  "community_count": 25
}
```

---

### UC0107 - Explore Entity Neighborhood

| Attribute            | Value                                                |
| -------------------- | ---------------------------------------------------- |
| **ID**               | UC0107                                               |
| **Name**             | Explore Entity Neighborhood                          |
| **Actor**            | WebUI User                                           |
| **Preconditions**    | Entity exists                                        |
| **Endpoint**         | `GET /api/v1/graph/entities/{entity_name}/neighbors` |
| **Related Features** | FEAT0405, FEAT0603                                   |
| **Related Rules**    | BR0010                                               |

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| depth | int | 1 | Traversal depth |
| limit | int | 50 | Max neighbors |

---

## Query Execution (UC02XX)

### UC0201 - Execute Simple Query

| Attribute            | Value                             |
| -------------------- | --------------------------------- |
| **ID**               | UC0201                            |
| **Name**             | Execute RAG Query                 |
| **Actor**            | API Client / WebUI User           |
| **Preconditions**    | Authenticated, workspace has data |
| **Endpoint**         | `POST /api/v1/query`              |
| **Related Features** | FEAT0007, FEAT0403                |
| **Related Rules**    | BR0101, BR0103, BR0105            |

**Request:**

```json
{
  "query": "What is the relationship between X and Y?",
  "mode": "hybrid",
  "max_tokens": 1000
}
```

**Response:**

```json
{
  "answer": "Based on the knowledge graph, X and Y are...",
  "sources": [
    { "document_id": "doc_123", "chunk_id": "chunk_45", "relevance": 0.92 }
  ],
  "context": {
    "entities_used": ["X", "Y"],
    "relationships_used": 3,
    "chunks_used": 5
  },
  "stats": {
    "retrieval_time_ms": 45,
    "generation_time_ms": 1200,
    "total_tokens": 850
  }
}
```

---

### UC0202 - Stream Query Response

| Attribute            | Value                         |
| -------------------- | ----------------------------- |
| **ID**               | UC0202                        |
| **Name**             | Stream Query Response via SSE |
| **Actor**            | WebUI User                    |
| **Preconditions**    | Authenticated                 |
| **Endpoint**         | `POST /api/v1/query/stream`   |
| **Related Features** | FEAT0008, FEAT0404            |
| **Related Rules**    | BR0104                        |

**SSE Event Sequence:**

```
event: start
data: {"query_id": "q_123"}

event: content
data: {"content": "Based on "}

event: content
data: {"content": "the knowledge graph..."}

event: sources
data: {"sources": [...]}

event: done
data: {"stats": {...}}
```

---

### UC0203 - Query with Mode Selection

| Attribute            | Value                    |
| -------------------- | ------------------------ |
| **ID**               | UC0203                   |
| **Name**             | Query with Specific Mode |
| **Actor**            | API Client / WebUI User  |
| **Preconditions**    | Authenticated            |
| **Endpoint**         | `POST /api/v1/query`     |
| **Related Features** | FEAT0101-FEAT0106        |
| **Related Rules**    | BR0103                   |

**Mode Options:**
| Mode | Description | Best For |
|------|-------------|----------|
| naive | Vector similarity only | Simple factual queries |
| local | Entity-centric + neighbors | Specific entity questions |
| global | Community-based | Broad topic overviews |
| hybrid | Local + global | General purpose (default) |
| mix | Weighted naive + graph | Tunable balance |
| bypass | No RAG, direct LLM | Creative/chat |

---

### UC0204 - Query with Conversation Context

| Attribute            | Value                           |
| -------------------- | ------------------------------- |
| **ID**               | UC0204                          |
| **Name**             | Query with Conversation History |
| **Actor**            | WebUI User                      |
| **Preconditions**    | Conversation exists             |
| **Endpoint**         | `POST /api/v1/query`            |
| **Related Features** | FEAT0007, FEAT0017              |
| **Related Rules**    | BR0107                          |

**Request:**

```json
{
  "query": "Tell me more about that",
  "conversation_id": "conv_abc123"
}
```

---

### UC0205 - Query Specific Workspace

| Attribute            | Value                      |
| -------------------- | -------------------------- |
| **ID**               | UC0205                     |
| **Name**             | Query Specific Workspace   |
| **Actor**            | API Client / WebUI User    |
| **Preconditions**    | Workspace exists, has data |
| **Endpoint**         | `POST /api/v1/query`       |
| **Related Features** | FEAT0007, FEAT0016         |
| **Related Rules**    | BR0201, BR0206             |

**Request:**

```json
{
  "query": "What are the key findings?",
  "workspace_id": "ws_research_2026"
}
```

---

### UC0206 - Get Query History

| Attribute            | Value                         |
| -------------------- | ----------------------------- |
| **ID**               | UC0206                        |
| **Name**             | Get Recent Query History      |
| **Actor**            | WebUI User                    |
| **Preconditions**    | Authenticated                 |
| **Endpoint**         | `GET /api/v1/queries/history` |
| **Related Features** | FEAT0017                      |
| **Related Rules**    | BR0201                        |

---

### UC0207 - Explain Query Retrieval

| Attribute            | Value                             |
| -------------------- | --------------------------------- |
| **ID**               | UC0207                            |
| **Name**             | Explain Query Retrieval Process   |
| **Actor**            | Developer / Power User            |
| **Preconditions**    | Query executed                    |
| **Endpoint**         | `POST /api/v1/query?explain=true` |
| **Related Features** | FEAT0109                          |
| **Related Rules**    | None                              |

**Response includes:**

```json
{
  "answer": "...",
  "explanation": {
    "keywords_extracted": ["term1", "term2"],
    "entities_matched": ["ENTITY_A"],
    "vector_search_results": 10,
    "graph_traversal_steps": 3,
    "context_truncation_applied": true,
    "token_budget": {
      "entities": 2000,
      "relationships": 1500,
      "chunks": 1500
    }
  }
}
```

---

### UC0208 - Query with Custom Parameters

| Attribute            | Value                                  |
| -------------------- | -------------------------------------- |
| **ID**               | UC0208                                 |
| **Name**             | Query with Custom Retrieval Parameters |
| **Actor**            | Advanced API Client                    |
| **Preconditions**    | Authenticated                          |
| **Endpoint**         | `POST /api/v1/query`                   |
| **Related Features** | FEAT0109                               |
| **Related Rules**    | BR0101, BR0108                         |

**Request:**

```json
{
  "query": "...",
  "params": {
    "top_k": 10,
    "similarity_threshold": 0.7,
    "max_graph_depth": 2,
    "include_community_context": true,
    "rerank_results": true
  }
}
```

---

## Workspace Management (UC03XX)

### UC0301 - Create Workspace

| Attribute            | Value                     |
| -------------------- | ------------------------- |
| **ID**               | UC0301                    |
| **Name**             | Create New Workspace      |
| **Actor**            | API Client / WebUI User   |
| **Preconditions**    | Authenticated             |
| **Endpoint**         | `POST /api/v1/workspaces` |
| **Related Features** | FEAT0016                  |
| **Related Rules**    | BR0206                    |

**Request:**

```json
{
  "name": "Research Project 2026",
  "description": "AI safety research documents"
}
```

---

### UC0302 - List Workspaces

| Attribute            | Value                    |
| -------------------- | ------------------------ |
| **ID**               | UC0302                   |
| **Name**             | List All Workspaces      |
| **Actor**            | API Client / WebUI User  |
| **Preconditions**    | Authenticated            |
| **Endpoint**         | `GET /api/v1/workspaces` |
| **Related Features** | FEAT0016, FEAT0604       |
| **Related Rules**    | BR0201                   |

---

### UC0303 - Get Workspace Statistics

| Attribute            | Value                                         |
| -------------------- | --------------------------------------------- |
| **ID**               | UC0303                                        |
| **Name**             | Get Workspace Statistics                      |
| **Actor**            | API Client / WebUI User                       |
| **Preconditions**    | Workspace exists                              |
| **Endpoint**         | `GET /api/v1/workspaces/{workspace_id}/stats` |
| **Related Features** | FEAT0016                                      |
| **Related Rules**    | BR0201                                        |

**Response:**

```json
{
  "workspace_id": "ws_abc123",
  "name": "Research Project 2026",
  "document_count": 150,
  "chunk_count": 2500,
  "entity_count": 1200,
  "relationship_count": 2800,
  "total_tokens_processed": 1500000,
  "estimated_cost_usd": 12.5,
  "created_at": "2026-01-01T00:00:00Z",
  "last_activity": "2026-01-09T10:00:00Z"
}
```

---

### UC0304 - Delete Workspace

| Attribute            | Value                                      |
| -------------------- | ------------------------------------------ |
| **ID**               | UC0304                                     |
| **Name**             | Delete Workspace and All Data              |
| **Actor**            | API Client / WebUI User                    |
| **Preconditions**    | Workspace exists, confirmation required    |
| **Endpoint**         | `DELETE /api/v1/workspaces/{workspace_id}` |
| **Related Features** | FEAT0016                                   |
| **Related Rules**    | BR0206                                     |

**Cascading Deletions:**

```
Workspace
├── Documents (all)
├── Chunks (all)
├── Embeddings (all)
├── Entities (all)
├── Relationships (all)
└── Conversations (all)
```

---

### UC0305 - Switch Active Workspace

| Attribute            | Value                         |
| -------------------- | ----------------------------- |
| **ID**               | UC0305                        |
| **Name**             | Switch Active Workspace in UI |
| **Actor**            | WebUI User                    |
| **Preconditions**    | Multiple workspaces exist     |
| **Endpoint**         | (Client-side state change)    |
| **Related Features** | FEAT0604                      |
| **Related Rules**    | BR0201                        |

---

## Conversation Management (UC04XX)

### UC0401 - Create Conversation

| Attribute            | Value                             |
| -------------------- | --------------------------------- |
| **ID**               | UC0401                            |
| **Name**             | Create New Conversation           |
| **Actor**            | WebUI User                        |
| **Preconditions**    | Authenticated, workspace selected |
| **Endpoint**         | `POST /api/v1/conversations`      |
| **Related Features** | FEAT0017                          |
| **Related Rules**    | BR0201                            |

**Request:**

```json
{
  "title": "Research Discussion",
  "workspace_id": "ws_abc123"
}
```

---

### UC0402 - List Conversations

| Attribute            | Value                       |
| -------------------- | --------------------------- |
| **ID**               | UC0402                      |
| **Name**             | List Conversations          |
| **Actor**            | WebUI User                  |
| **Preconditions**    | Authenticated               |
| **Endpoint**         | `GET /api/v1/conversations` |
| **Related Features** | FEAT0017                    |
| **Related Rules**    | BR0201                      |

---

### UC0403 - Add Message to Conversation

| Attribute            | Value                                      |
| -------------------- | ------------------------------------------ |
| **ID**               | UC0403                                     |
| **Name**             | Add Message to Conversation                |
| **Actor**            | API/WebUI (automated after query)          |
| **Preconditions**    | Conversation exists                        |
| **Endpoint**         | `POST /api/v1/conversations/{id}/messages` |
| **Related Features** | FEAT0017                                   |
| **Related Rules**    | BR0107                                     |

**Request:**

```json
{
  "role": "user",
  "content": "What is machine learning?"
}
```

---

### UC0404 - Get Conversation History

| Attribute            | Value                                     |
| -------------------- | ----------------------------------------- |
| **ID**               | UC0404                                    |
| **Name**             | Get Full Conversation History             |
| **Actor**            | WebUI User                                |
| **Preconditions**    | Conversation exists                       |
| **Endpoint**         | `GET /api/v1/conversations/{id}/messages` |
| **Related Features** | FEAT0017                                  |
| **Related Rules**    | BR0201, BR0107                            |

---

### UC0405 - Delete Conversation

| Attribute            | Value                               |
| -------------------- | ----------------------------------- |
| **ID**               | UC0405                              |
| **Name**             | Delete Conversation                 |
| **Actor**            | WebUI User                          |
| **Preconditions**    | Conversation exists                 |
| **Endpoint**         | `DELETE /api/v1/conversations/{id}` |
| **Related Features** | FEAT0017                            |
| **Related Rules**    | BR0201                              |

---

### UC0406 - Rename Conversation

| Attribute            | Value                              |
| -------------------- | ---------------------------------- |
| **ID**               | UC0406                             |
| **Name**             | Rename Conversation                |
| **Actor**            | WebUI User                         |
| **Preconditions**    | Conversation exists                |
| **Endpoint**         | `PATCH /api/v1/conversations/{id}` |
| **Related Features** | FEAT0017                           |
| **Related Rules**    | BR0201                             |

---

## Administration (UC05XX)

### UC0501 - View System Health

| Attribute            | Value                     |
| -------------------- | ------------------------- |
| **ID**               | UC0501                    |
| **Name**             | View System Health Status |
| **Actor**            | Admin / Monitoring System |
| **Preconditions**    | None (public endpoint)    |
| **Endpoint**         | `GET /health`             |
| **Related Features** | None                      |
| **Related Rules**    | None                      |

**Response:**

```json
{
  "status": "healthy",
  "components": {
    "database": "healthy",
    "vector_storage": "healthy",
    "graph_storage": "healthy",
    "llm_provider": "healthy"
  },
  "uptime_seconds": 86400
}
```

---

### UC0502 - View Metrics

| Attribute            | Value                     |
| -------------------- | ------------------------- |
| **ID**               | UC0502                    |
| **Name**             | View System Metrics       |
| **Actor**            | Admin / Monitoring System |
| **Preconditions**    | Authenticated (admin)     |
| **Endpoint**         | `GET /metrics`            |
| **Related Features** | None                      |
| **Related Rules**    | None                      |

---

### UC0503 - View Audit Logs

| Attribute            | Value                          |
| -------------------- | ------------------------------ |
| **ID**               | UC0503                         |
| **Name**             | View Audit Logs                |
| **Actor**            | Admin                          |
| **Preconditions**    | Admin role                     |
| **Endpoint**         | `GET /api/v1/admin/audit-logs` |
| **Related Features** | FEAT0020                       |
| **Related Rules**    | BR0405                         |

---

### UC0504 - Manage Rate Limits

| Attribute            | Value                                        |
| -------------------- | -------------------------------------------- |
| **ID**               | UC0504                                       |
| **Name**             | Configure Tenant Rate Limits                 |
| **Actor**            | Admin                                        |
| **Preconditions**    | Admin role                                   |
| **Endpoint**         | `PUT /api/v1/admin/tenants/{id}/rate-limits` |
| **Related Features** | FEAT0018                                     |
| **Related Rules**    | BR0204                                       |

---

## WebUI Interactions (UC06XX)

> Use cases specific to the EdgeQuake WebUI (Next.js/React application).

### UC0601 - Visualize Knowledge Graph

| Attribute            | Value                                                      |
| -------------------- | ---------------------------------------------------------- |
| **ID**               | UC0601                                                     |
| **Name**             | Visualize Knowledge Graph                                  |
| **Actor**            | WebUI User                                                 |
| **Preconditions**    | Authenticated, workspace with indexed documents            |
| **Component**        | [KnowledgeGraph](../edgequake_webui/src/components/graph/) |
| **Related Features** | FEAT0601, FEAT0602                                         |
| **Related Rules**    | BR0603                                                     |

**Steps:**

1. User navigates to Knowledge Graph view
2. Component fetches nodes/edges from API
3. Sigma.js renders graph with force-directed layout
4. Initial display limited to 500 nodes (BR0603)
5. User can pan, zoom, and hover for tooltips
6. Click on node expands its relationships

**Success Outcome:**

- Interactive graph rendered within 2 seconds
- Smooth 60fps pan/zoom interactions

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| No data | Empty state with message | No documents indexed |
| Render failure | Fallback to table view | WebGL not supported |

---

### UC0602 - Execute RAG Query

| Attribute            | Value                                                    |
| -------------------- | -------------------------------------------------------- |
| **ID**               | UC0602                                                   |
| **Name**             | Execute RAG Query via Chat                               |
| **Actor**            | WebUI User                                               |
| **Preconditions**    | Authenticated, workspace selected                        |
| **Component**        | [ChatInterface](../edgequake_webui/src/components/chat/) |
| **Related Features** | FEAT0609, FEAT0611, FEAT0612                             |
| **Related Rules**    | BR0604, BR0612                                           |

**Steps:**

1. User types query in chat input
2. Submit triggers streaming API call
3. Loading indicator appears within 100ms (BR0612)
4. Streaming chunks rendered incrementally
5. Source citations displayed as collapsible blocks
6. Response added to conversation history

**Success Outcome:**

- First chunk visible within 500ms
- Complete response with citations and sources

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Timeout | Retry button with error message | API overload |
| Stream failure | Partial response preserved | Connection drop |

---

### UC0603 - Upload Document via UI

| Attribute            | Value                                                          |
| -------------------- | -------------------------------------------------------------- |
| **ID**               | UC0603                                                         |
| **Name**             | Upload Document via Drag-and-Drop                              |
| **Actor**            | WebUI User                                                     |
| **Preconditions**    | Authenticated, workspace selected                              |
| **Component**        | [DocumentUpload](../edgequake_webui/src/components/documents/) |
| **Related Features** | FEAT0605                                                       |
| **Related Rules**    | BR0606                                                         |

**Steps:**

1. User drags file(s) onto drop zone
2. Client validates file type (PDF, TXT, MD)
3. Client validates file size < 50MB (BR0606)
4. Upload progress displayed
5. Processing status tracked via polling/websocket
6. Document appears in list when complete

**Success Outcome:**

- File uploaded and processing started
- Real-time progress feedback

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Invalid type | Toast with allowed types | Wrong file format |
| Size exceeded | Toast with size limit | File > 50MB |
| Upload failed | Retry button | Network error |

---

### UC0604 - Manage Conversation History

| Attribute            | Value                                                                |
| -------------------- | -------------------------------------------------------------------- |
| **ID**               | UC0604                                                               |
| **Name**             | Browse and Resume Conversations                                      |
| **Actor**            | WebUI User                                                           |
| **Preconditions**    | Authenticated                                                        |
| **Component**        | [ConversationList](../edgequake_webui/src/components/conversations/) |
| **Related Features** | FEAT0610, FEAT0613                                                   |
| **Related Rules**    | BR0602, BR0611                                                       |

**Steps:**

1. User opens conversation sidebar
2. Conversations loaded from localStorage/API
3. List sorted by last activity (most recent first)
4. User clicks conversation to resume
5. Chat interface populated with history
6. User can delete or rename conversations

**Success Outcome:**

- Conversation history loaded within 200ms
- Seamless context restoration

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Storage quota | Prune oldest entries | BR0611 limit reached |
| Corrupted data | Graceful recovery | localStorage corruption |

---

### UC0605 - Switch Theme

| Attribute            | Value                                                   |
| -------------------- | ------------------------------------------------------- |
| **ID**               | UC0605                                                  |
| **Name**             | Toggle Light/Dark Theme                                 |
| **Actor**            | WebUI User                                              |
| **Preconditions**    | Application loaded                                      |
| **Component**        | [ThemeToggle](../edgequake_webui/src/components/theme/) |
| **Related Features** | FEAT0619                                                |
| **Related Rules**    | BR0601                                                  |

**Steps:**

1. User clicks theme toggle button
2. Theme state updated in Zustand store
3. CSS variables applied immediately
4. Preference persisted to localStorage
5. Theme restored on next session

**Success Outcome:**

- Theme change applied instantly (<16ms)
- Preference persists across sessions

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Storage blocked | In-memory fallback | Privacy mode |

---

### UC0606 - Configure Settings

| Attribute            | Value                                                        |
| -------------------- | ------------------------------------------------------------ |
| **ID**               | UC0606                                                       |
| **Name**             | Configure User Settings                                      |
| **Actor**            | WebUI User                                                   |
| **Preconditions**    | Authenticated                                                |
| **Component**        | [SettingsPanel](../edgequake_webui/src/components/settings/) |
| **Related Features** | FEAT0608                                                     |
| **Related Rules**    | BR0608                                                       |

**Steps:**

1. User opens settings panel
2. Current settings loaded from store
3. User modifies preferences (language, model, etc.)
4. Settings validated before save
5. Changes persisted to localStorage
6. Panel closed with confirmation toast

**Success Outcome:**

- Settings validated and saved
- Changes take effect immediately

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Invalid value | Inline validation error | Schema mismatch |
| Save failed | Retry with error toast | Storage full |

---

### UC0607 - Navigate with Keyboard

| Attribute            | Value                             |
| -------------------- | --------------------------------- |
| **ID**               | UC0607                            |
| **Name**             | Navigate Application via Keyboard |
| **Actor**            | WebUI User                        |
| **Preconditions**    | Application loaded                |
| **Component**        | All interactive components        |
| **Related Features** | FEAT0618                          |
| **Related Rules**    | BR0605, BR0610                    |

**Steps:**

1. User presses Tab to move focus
2. Focus ring visible on active element
3. Enter/Space activates buttons
4. Escape closes modals (focus restored)
5. Arrow keys navigate lists/menus
6. Shortcuts (Ctrl+K) open command palette

**Success Outcome:**

- All features accessible via keyboard
- Focus never trapped unexpectedly

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Focus lost | Auto-recover to main area | Dynamic content update |

---

### UC0608 - View API Errors

| Attribute            | Value                                                     |
| -------------------- | --------------------------------------------------------- |
| **ID**               | UC0608                                                    |
| **Name**             | View User-Friendly Error Messages                         |
| **Actor**            | WebUI User                                                |
| **Preconditions**    | API error occurred                                        |
| **Component**        | [ErrorBoundary](../edgequake_webui/src/components/error/) |
| **Related Features** | FEAT0615                                                  |
| **Related Rules**    | BR0607                                                    |

**Steps:**

1. API call fails with error response
2. Error intercepted by global handler
3. Error code mapped to user-friendly message
4. Toast notification displayed
5. Detailed info available via "Show Details"
6. Error logged for debugging

**Success Outcome:**

- User understands what went wrong
- Clear path to resolution

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Unknown error | Generic "Something went wrong" | Unmapped error code |

---

### UC0609 - Expand Graph Node

| Attribute            | Value                                                      |
| -------------------- | ---------------------------------------------------------- |
| **ID**               | UC0609                                                     |
| **Name**             | Expand Knowledge Graph Node                                |
| **Actor**            | WebUI User                                                 |
| **Preconditions**    | Graph visualization active                                 |
| **Component**        | [KnowledgeGraph](../edgequake_webui/src/components/graph/) |
| **Related Features** | FEAT0602, FEAT0603                                         |
| **Related Rules**    | BR0603                                                     |

**Steps:**

1. User clicks on collapsed node
2. API fetches connected nodes/edges
3. New nodes animated into view
4. Layout rebalances smoothly
5. Node marked as expanded
6. Double-click to collapse

**Success Outcome:**

- Related nodes revealed progressively
- Graph remains performant

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| No connections | Tooltip "No connections" | Isolated entity |
| Max nodes reached | Warning toast | Performance limit |

---

### UC0610 - Search Documents

| Attribute            | Value                                                          |
| -------------------- | -------------------------------------------------------------- |
| **ID**               | UC0610                                                         |
| **Name**             | Search Document Library                                        |
| **Actor**            | WebUI User                                                     |
| **Preconditions**    | Authenticated, documents exist                                 |
| **Component**        | [DocumentSearch](../edgequake_webui/src/components/documents/) |
| **Related Features** | FEAT0606, FEAT0607                                             |
| **Related Rules**    | BR0612                                                         |

**Steps:**

1. User types in search input
2. Debounced search after 300ms pause
3. Loading indicator during search
4. Results displayed with highlights
5. Click result opens document detail
6. Empty state if no matches

**Success Outcome:**

- Relevant results within 500ms
- Clear match highlighting

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Search failed | Retry button | API error |
| No results | Helpful empty state | No matches found |

---

## PDF Processing (UC10XX)

> Use cases specific to the EdgeQuake PDF extraction and conversion pipeline.

### UC1001 - Convert PDF to Markdown

| Attribute            | Value                                               |
| -------------------- | --------------------------------------------------- |
| **ID**               | UC1001                                              |
| **Name**             | Convert PDF Document to Markdown                    |
| **Actor**            | API Client / Pipeline                               |
| **Preconditions**    | Valid PDF file                                      |
| **Module**           | [edgequake-pdf](../edgequake/crates/edgequake-pdf/) |
| **Related Features** | FEAT1001, FEAT0501                                  |
| **Related Rules**    | BR1001, BR1002                                      |

**Steps:**

1. PDF file received via API or file path
2. SotaBackend parses PDF structure
3. Text extracted with font metadata
4. Processor chain applies transformations
5. Markdown rendered with headings/lists/tables
6. Output returned or saved to file

**Success Outcome:**

- Markdown preserves document structure
- Reading order accuracy > 95%

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Malformed PDF | Graceful fallback (BR1002) | Corrupted file |
| Encrypted PDF | Error with message | Password protected |

---

### UC1002 - Extract Tables from PDF

| Attribute            | Value                                                                  |
| -------------------- | ---------------------------------------------------------------------- |
| **ID**               | UC1002                                                                 |
| **Name**             | Extract and Format Tables from PDF                                     |
| **Actor**            | API Client / Pipeline                                                  |
| **Preconditions**    | PDF contains tabular data                                              |
| **Module**           | [lattice.rs](../edgequake/crates/edgequake-pdf/src/backend/lattice.rs) |
| **Related Features** | FEAT1002, FEAT0503                                                     |
| **Related Rules**    | BR1004                                                                 |

**Steps:**

1. LatticeEngine detects table regions
2. Cell boundaries identified via line detection
3. Text assigned to cells based on position
4. Column alignment determined
5. Markdown table generated

**Success Outcome:**

- Tables rendered with correct alignment
- Cell content preserved

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| No lines detected | Text-based reconstruction | Borderless table |
| Merged cells | Best-effort spanning | Complex layout |

---

### UC1003 - Detect Multi-Column Layout

| Attribute            | Value                                                    |
| -------------------- | -------------------------------------------------------- |
| **ID**               | UC1003                                                   |
| **Name**             | Detect and Linearize Multi-Column Text                   |
| **Actor**            | Pipeline                                                 |
| **Preconditions**    | PDF has multi-column layout                              |
| **Module**           | [layout/](../edgequake/crates/edgequake-pdf/src/layout/) |
| **Related Features** | FEAT1003                                                 |
| **Related Rules**    | BR1003                                                   |

**Steps:**

1. Geometric analysis identifies column gaps
2. Text blocks clustered by horizontal position
3. Reading order determined (left-to-right, top-to-bottom)
4. Columns linearized in correct sequence
5. Paragraph boundaries preserved

**Success Outcome:**

- Correct reading order for multi-column docs
- No text interleaving between columns

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Ambiguous columns | Conservative merge | Overlapping regions |

---

### UC1004 - Detect Document Headings

| Attribute            | Value                                                                                  |
| -------------------- | -------------------------------------------------------------------------------------- |
| **ID**               | UC1004                                                                                 |
| **Name**             | Identify and Format Headings                                                           |
| **Actor**            | Pipeline                                                                               |
| **Preconditions**    | PDF has styled headings                                                                |
| **Module**           | [processors/structure_detection.rs](../edgequake/crates/edgequake-pdf/src/processors/) |
| **Related Features** | FEAT1022                                                                               |
| **Related Rules**    | BR1001                                                                                 |

**Steps:**

1. Font size analysis identifies larger text
2. Bold/weight detection as heading signal
3. Section numbering patterns detected (1.2.3)
4. Heading level assigned (H1-H6)
5. Markdown heading syntax applied

**Success Outcome:**

- Headings correctly formatted as `#` syntax
- Hierarchy preserved

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| No font info | Pattern-based detection | Embedded fonts |

---

### UC1005 - Extract Document Metadata

| Attribute            | Value                                                              |
| -------------------- | ------------------------------------------------------------------ |
| **ID**               | UC1005                                                             |
| **Name**             | Extract PDF Metadata                                               |
| **Actor**            | API Client                                                         |
| **Preconditions**    | Valid PDF file                                                     |
| **Module**           | [extractor.rs](../edgequake/crates/edgequake-pdf/src/extractor.rs) |
| **Related Features** | FEAT1001                                                           |
| **Related Rules**    | BR1002                                                             |

**Steps:**

1. Parse PDF document info dictionary
2. Extract title, author, creation date
3. Count pages
4. Detect encryption status
5. Return structured metadata

**Success Outcome:**

- Metadata extracted without full conversion
- Fast info retrieval

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Missing info | Return partial data | No metadata embedded |

---

### UC1006 - Handle Malformed PDF

| Attribute            | Value                                                              |
| -------------------- | ------------------------------------------------------------------ |
| **ID**               | UC1006                                                             |
| **Name**             | Gracefully Handle Malformed PDF                                    |
| **Actor**            | Pipeline                                                           |
| **Preconditions**    | PDF fails standard parsing                                         |
| **Module**           | [extractor.rs](../edgequake/crates/edgequake-pdf/src/extractor.rs) |
| **Related Features** | FEAT1001                                                           |
| **Related Rules**    | BR1002                                                             |

**Steps:**

1. Initial parse fails with error
2. Fallback to lenient parsing mode
3. Extract whatever text is recoverable
4. Log warning with failure details
5. Return partial content with warning flag

**Success Outcome:**

- No crash on malformed input
- Maximum content recovered

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Complete corruption | Return empty with error | Unrecoverable file |

---

### UC1007 - Process Large PDF

| Attribute            | Value                                               |
| -------------------- | --------------------------------------------------- |
| **ID**               | UC1007                                              |
| **Name**             | Process Multi-Page PDF Efficiently                  |
| **Actor**            | Pipeline                                            |
| **Preconditions**    | PDF > 100 pages                                     |
| **Module**           | [edgequake-pdf](../edgequake/crates/edgequake-pdf/) |
| **Related Features** | FEAT1001                                            |
| **Related Rules**    | BR1001                                              |

**Steps:**

1. Parse PDF in streaming mode
2. Process pages in batches
3. Memory pressure monitored
4. Progress reported periodically
5. Results aggregated at end

**Success Outcome:**

- Consistent memory usage
- Linear time scaling

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Memory exhaustion | Reduce batch size | Very complex pages |

---

### UC1008 - Preserve Code Blocks

| Attribute            | Value                                                                                  |
| -------------------- | -------------------------------------------------------------------------------------- |
| **ID**               | UC1008                                                                                 |
| **Name**             | Detect and Format Code Blocks                                                          |
| **Actor**            | Pipeline                                                                               |
| **Preconditions**    | PDF contains code snippets                                                             |
| **Module**           | [processors/structure_detection.rs](../edgequake/crates/edgequake-pdf/src/processors/) |
| **Related Features** | FEAT1001                                                                               |
| **Related Rules**    | BR1001                                                                                 |

**Steps:**

1. Detect monospace font regions
2. Identify indentation patterns
3. Check for syntax-like content
4. Apply fenced code block formatting
5. Attempt language detection

**Success Outcome:**

- Code wrapped in triple backticks
- Language hint when detectable

**Error Scenarios:**
| Error | Handling | Cause |
|-------|----------|-------|
| Mixed fonts | Conservative detection | Inline code |

---

## Summary Statistics

| Category                | Total  | Implemented | Tested | Documented |
| ----------------------- | ------ | ----------- | ------ | ---------- |
| Document Management     | 8      | 8           | 8      | 8          |
| Knowledge Graph         | 7      | 7           | 6      | 7          |
| Query Execution         | 8      | 8           | 7      | 8          |
| Workspace Management    | 5      | 5           | 5      | 5          |
| Conversation Management | 6      | 6           | 5      | 6          |
| Administration          | 4      | 4           | 3      | 4          |
| WebUI Interactions      | 10     | 10          | 8      | 10         |
| PDF Processing          | 8      | 8           | 7      | 8          |
| **TOTAL**               | **56** | **56**      | **49** | **56**     |

---

## Related Documents

- [Features Registry](features.md)
- [Business Rules](business_rules.md)
- [API Reference](0003-api-reference.md)
- [Architecture Overview](0002-architecture-overview.md)
