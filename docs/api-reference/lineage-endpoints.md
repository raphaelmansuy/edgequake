---
title: 'Lineage API Reference'
---

# Lineage API Reference

> REST API endpoints for document lineage, chunk provenance, and entity traceability

---

## Base URL

```
http://localhost:8080/api/v1
```

---

## Endpoints Overview

| Method | Path                                       | Description                          |
| ------ | ------------------------------------------ | ------------------------------------ |
| GET    | `/documents/{id}/lineage`                  | Complete document lineage tree       |
| GET    | `/documents/{id}/metadata`                 | Document metadata (flat KV)          |
| GET    | `/chunks/{chunk_id}`                       | Chunk detail with entities           |
| GET    | `/chunks/{chunk_id}/lineage`               | Chunk lineage with parent refs       |
| GET    | `/entities/{entity_id}/provenance`         | Entity source traceability           |
| GET    | `/lineage/entities/{entity_name}`          | Entity lineage (all source docs)     |
| GET    | `/lineage/documents/{document_id}`         | Document graph lineage (entities+rels)|

---

## Document Lineage

### `GET /api/v1/documents/{document_id}/lineage`

Returns the complete lineage tree for a document: all chunks with position data and all entities extracted from them.

**Path Parameters**

| Parameter     | Type   | Description  |
| ------------- | ------ | ------------ |
| `document_id` | string | Document ID  |

**Response** `200 OK`

```json
{
  "document_id": "abc123def456",
  "chunks": [
    {
      "chunk_id": "abc123def456-chunk-0",
      "chunk_index": 0,
      "content_preview": "EdgeQuake is an advanced RAG framework...",
      "tokens": 150,
      "start_line": 1,
      "end_line": 25,
      "entity_count": 5
    },
    {
      "chunk_id": "abc123def456-chunk-1",
      "chunk_index": 1,
      "content_preview": "The pipeline processes documents through...",
      "tokens": 142,
      "start_line": 26,
      "end_line": 50,
      "entity_count": 3
    }
  ],
  "entities": [
    {
      "entity_id": "EDGEQUAKE",
      "entity_name": "EDGEQUAKE",
      "entity_type": "TECHNOLOGY",
      "chunk_ids": ["abc123def456-chunk-0", "abc123def456-chunk-1"]
    }
  ]
}
```

**Error Responses**

| Status | Description          |
| ------ | -------------------- |
| 404    | Document not found   |
| 500    | Internal server error|

---

## Document Metadata

### `GET /api/v1/documents/{document_id}/metadata`

Returns all stored metadata for a document as a flat JSON object. This data is from the KV storage key `{document_id}-metadata`.

**Path Parameters**

| Parameter     | Type   | Description  |
| ------------- | ------ | ------------ |
| `document_id` | string | Document ID  |

**Response** `200 OK`

```json
{
  "id": "abc123def456",
  "title": "paper.pdf",
  "document_type": "pdf",
  "file_size_bytes": 1048576,
  "sha256_checksum": "a1b2c3d4e5f6...",
  "page_count": 12,
  "status": "completed",
  "created_at": "2025-01-15T10:30:00Z",
  "processed_at": "2025-01-15T10:31:00Z",
  "llm_model": "gpt-5-nano",
  "embedding_model": "text-embedding-3-small",
  "chunk_count": 8
}
```

**Notes**: Fields vary depending on document source (PDF vs. text/markdown). The response includes all metadata stored during ingestion.

---

## Chunk Detail

### `GET /api/v1/chunks/{chunk_id}`

Returns detailed information about a specific chunk, including extracted entities and relationships.

**Path Parameters**

| Parameter  | Type   | Description                              |
| ---------- | ------ | ---------------------------------------- |
| `chunk_id` | string | Chunk ID (format: `{doc_id}-chunk-{N}`)  |

**Response** `200 OK`

```json
{
  "chunk_id": "abc123def456-chunk-0",
  "document_id": "abc123def456",
  "document_name": "paper.pdf",
  "content": "EdgeQuake is an advanced RAG framework implemented in Rust...",
  "index": 0,
  "char_range": {
    "start": 0,
    "end": 1024
  },
  "start_line": 1,
  "end_line": 25,
  "token_count": 150,
  "entities": [
    {
      "id": "EDGEQUAKE",
      "name": "EDGEQUAKE",
      "entity_type": "TECHNOLOGY",
      "description": "An advanced RAG framework"
    }
  ],
  "relationships": [
    {
      "source_name": "EDGEQUAKE",
      "target_name": "RUST",
      "relation_type": "implemented_in",
      "description": "EdgeQuake is implemented in Rust"
    }
  ],
  "extraction_metadata": null
}
```

---

## Chunk Lineage

### `GET /api/v1/chunks/{chunk_id}/lineage`

Returns a chunk's complete lineage chain — parent document info, position data, model provenance, and entity/relationship summary — in a single call.

**Path Parameters**

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `chunk_id` | string | Chunk ID    |

**Response** `200 OK`

```json
{
  "chunk_id": "abc123def456-chunk-0",
  "document_id": "abc123def456",
  "document_name": "paper.pdf",
  "document_type": "pdf",
  "chunk_index": 0,
  "content_preview": "EdgeQuake is an advanced RAG framework...",
  "tokens": 150,
  "start_line": 1,
  "end_line": 25,
  "start_offset": 0,
  "end_offset": 1024,
  "llm_model": "gpt-5-nano",
  "embedding_model": "text-embedding-3-small",
  "embedding_dimension": 1536,
  "entity_names": ["EDGEQUAKE", "RUST", "RAG"],
  "entity_count": 3,
  "relationship_count": 2,
  "file_path": "/uploads/paper.pdf",
  "created_at": "2025-01-15T10:30:00Z"
}
```

**Implements**
- **F3**: Every chunk contains parent_document_id and position info
- **F8**: PDF → Document → Chunk → Entity chain is traceable

---

## Entity Provenance

### `GET /api/v1/entities/{entity_id}/provenance`

Returns the full provenance trail for an entity: all source documents and chunks where it was extracted.

**Path Parameters**

| Parameter   | Type   | Description                         |
| ----------- | ------ | ----------------------------------- |
| `entity_id` | string | Entity ID (UPPERCASE with underscores) |

**Response** `200 OK`

```json
{
  "entity_id": "EDGEQUAKE",
  "entity_name": "EDGEQUAKE",
  "entity_type": "TECHNOLOGY",
  "description": "An advanced RAG framework implemented in Rust",
  "sources": [
    {
      "document_id": "abc123def456",
      "document_name": "paper.pdf",
      "chunks": [
        {
          "chunk_id": "abc123def456-chunk-0",
          "start_line": null,
          "end_line": null,
          "source_text": null
        }
      ],
      "first_extracted_at": null
    }
  ],
  "total_extraction_count": 2,
  "related_entities": [
    {
      "entity_id": "RUST",
      "entity_name": "RUST",
      "relationship_type": "implemented_in",
      "shared_documents": 1
    }
  ]
}
```

---

## Entity Lineage

### `GET /api/v1/lineage/entities/{entity_name}`

Returns source documents and chunks for a named entity.

**Path Parameters**

| Parameter     | Type   | Description  |
| ------------- | ------ | ------------ |
| `entity_name` | string | Entity name  |

**Response** `200 OK`

```json
{
  "entity_name": "SARAH_CHEN",
  "entity_type": "PERSON",
  "source_count": 2,
  "source_documents": [
    {
      "document_id": "abc123",
      "chunk_ids": ["abc123-chunk-0", "abc123-chunk-3"],
      "line_ranges": []
    }
  ],
  "description_versions": []
}
```

---

## Document Graph Lineage

### `GET /api/v1/lineage/documents/{document_id}`

Returns all entities and relationships extracted from a document, with extraction statistics.

**Path Parameters**

| Parameter     | Type   | Description  |
| ------------- | ------ | ------------ |
| `document_id` | string | Document ID  |

**Response** `200 OK`

```json
{
  "document_id": "abc123def456",
  "chunk_count": 8,
  "extraction_stats": {
    "total_entities": 20,
    "unique_entities": 15,
    "total_relationships": 12,
    "unique_relationships": 10,
    "processing_time_ms": null
  },
  "entities": [
    {
      "name": "EDGEQUAKE",
      "entity_type": "TECHNOLOGY",
      "source_chunks": ["abc123def456-chunk-0"],
      "is_shared": false
    }
  ],
  "relationships": [
    {
      "source": "EDGEQUAKE",
      "target": "RUST",
      "keywords": "implemented_in",
      "source_chunks": ["abc123def456-chunk-0"]
    }
  ]
}
```

---

## SDK Usage Examples

### Rust

```rust
use edgequake_sdk::EdgeQuake;

let client = EdgeQuake::new("http://localhost:8080")?;

// Document lineage
let lineage = client.documents().get_lineage("abc123").await?;
println!("Chunks: {}", lineage.chunks.len());

// Document metadata
let meta = client.documents().get_metadata("abc123").await?;

// Chunk lineage
let chunk = client.chunks().get_lineage("abc123-chunk-0").await?;
println!("Lines {}-{}", chunk.start_line.unwrap_or(0), chunk.end_line.unwrap_or(0));
```

### TypeScript

```typescript
import { EdgeQuake } from "@edgequake/sdk";

const client = new EdgeQuake({ baseUrl: "http://localhost:8080" });

// Document lineage
const lineage = await client.documents.getLineage("abc123");
console.log(`${lineage.chunks.length} chunks`);

// Document metadata
const meta = await client.documents.getMetadata("abc123");

// Chunk lineage
const chunk = await client.chunks.getLineage("abc123-chunk-0");
console.log(`Lines ${chunk.start_line}-${chunk.end_line}`);
```

### Python

```python
from edgequake import EdgeQuake

client = EdgeQuake(base_url="http://localhost:8080")

# Document lineage
lineage = client.documents.get_lineage("abc123")
print(f"{len(lineage.chunks)} chunks")

# Document metadata
meta = client.documents.get_metadata("abc123")

# Chunk lineage
chunk = client.chunks.get_lineage("abc123-chunk-0")
print(f"Lines {chunk.start_line}-{chunk.end_line}")
```

---

## Error Handling

All endpoints return standard error responses:

```json
{
  "error": "NotFound",
  "message": "Document 'abc123' not found"
}
```

| Status | Meaning              |
| ------ | -------------------- |
| 200    | Success              |
| 404    | Resource not found   |
| 500    | Internal server error|

---

## OpenAPI Documentation

Interactive API docs are available at:

```
http://localhost:8080/swagger-ui/
```

All lineage endpoints are tagged under the "Lineage" group with utoipa annotations.
