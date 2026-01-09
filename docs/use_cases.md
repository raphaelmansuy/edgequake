# EdgeQuake Use Cases Registry

> Central registry of all use cases supported by EdgeQuake.
> Use UCXXXX references in API handlers for traceability.

**Version**: 1.0.0 | **Last Updated**: 2026-01-09

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

## Summary Statistics

| Category                | Total  | Implemented | Tested | Documented |
| ----------------------- | ------ | ----------- | ------ | ---------- |
| Document Management     | 8      | 8           | 8      | 8          |
| Knowledge Graph         | 7      | 7           | 6      | 7          |
| Query Execution         | 8      | 8           | 7      | 8          |
| Workspace Management    | 5      | 5           | 5      | 5          |
| Conversation Management | 6      | 6           | 5      | 6          |
| Administration          | 4      | 4           | 3      | 4          |
| **TOTAL**               | **38** | **38**      | **34** | **38**     |

---

## Related Documents

- [Features Registry](features.md)
- [Business Rules](business_rules.md)
- [API Reference](0003-api-reference.md)
- [Architecture Overview](0002-architecture-overview.md)
