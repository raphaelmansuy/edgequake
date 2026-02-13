# Lineage Tracking Architecture

> Complete traceability from source documents to extracted entities

---

## Overview

EdgeQuake implements a comprehensive lineage tracking system that records the full provenance chain for every piece of knowledge in the graph. This enables:

- **Reproducibility**: Know exactly which models and parameters produced each entity
- **Auditability**: Trace any entity back to its source document and line number
- **Quality assurance**: Compare extraction results across different models
- **Debugging**: Identify pipeline failures at any stage

---

## Data Model

### Lineage Chain

```
┌─────────────────────────────────────────────────────────────────────┐
│                     LINEAGE INTEGRITY CHAIN                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  PDF Upload (file, sha256, size)                                    │
│    │                                                                │
│    ▼                                                                │
│  PdfDocument (pdf_id, document_id, filename, page_count)            │
│    │                                                                │
│    ▼                                                                │
│  Document (document_id, pdf_id, document_type, llm_model,          │
│            embedding_model, file_size, sha256_checksum)             │
│    │                                                                │
│    ▼                                                                │
│  Chunk (chunk_id, full_doc_id, chunk_order_index,                   │
│         start_line, end_line, start_offset, end_offset,             │
│         llm_model, embedding_model, embedding_dimension)            │
│    │                                                                │
│    ▼                                                                │
│  Entity (entity_id, chunk_ids[], source_documents[])                │
│                                                                     │
│  INVARIANTS:                                                        │
│  - Every chunk MUST have a valid parent document_id (full_doc_id)   │
│  - Every entity MUST reference at least one source chunk            │
│  - Every PDF-sourced document MUST link back to pdf_id              │
│  - All timestamps are UTC ISO-8601                                  │
│  - All IDs are immutable once created                               │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Level-by-Level Metadata

| Level        | Fields                                                                              |
| ------------ | ------------------------------------------------------------------------------------ |
| **PDF**      | pdf_id, document_id, filename, file_size_bytes, sha256_checksum, page_count          |
| **Document** | document_id, file_path, file_size, document_type, sha256_checksum, pdf_id,           |
|              | llm_model, embedding_model, processed_at, created_at, updated_at                    |
| **Chunk**    | chunk_id, full_doc_id, chunk_order_index, start_line, end_line,                      |
|              | start_offset, end_offset, llm_model, embedding_model, embedding_dimension, tokens    |
| **Lineage**  | extraction_provider, extraction_model, embedding_provider, embedding_model, dims     |
| **Entity**   | entity_id, chunk_ids[], source_documents[], extraction_metadata                      |

---

## Core Types

### Document (`edgequake-core/src/types/document.rs`)

```rust
pub struct Document {
    pub id: String,                          // MD5 of content
    pub file_path: Option<String>,           // Source file path
    pub content: String,                     // Document text
    pub content_length: usize,               // Text length
    pub status: DocumentStatus,              // Pending/Processing/Completed/Failed
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub chunk_ids: Option<Vec<String>>,      // Linked chunk IDs

    // Lineage fields (added OODA-03)
    pub document_type: Option<String>,       // "pdf", "markdown", "text"
    pub file_size: Option<u64>,              // Original file size in bytes
    pub sha256_checksum: Option<String>,     // File integrity hash
    pub pdf_id: Option<String>,              // Link to source PdfDocument
    pub llm_model: Option<String>,           // Extraction model used
    pub embedding_model: Option<String>,     // Vectorization model used
    pub processed_at: Option<DateTime<Utc>>, // Processing completion time
}
```

### Chunk (`edgequake-core/src/types/chunk.rs`)

```rust
pub struct Chunk {
    pub id: String,                          // MD5 of content
    pub content: String,                     // Chunk text
    pub tokens: u32,                         // Token count
    pub chunk_order_index: u32,              // Position in document (0-indexed)
    pub full_doc_id: String,                 // Parent document ID
    pub file_path: Option<String>,           // Source file path

    // Position metadata (added OODA-01)
    pub start_line: Option<usize>,           // Start line (1-indexed)
    pub end_line: Option<usize>,             // End line (1-indexed)
    pub start_offset: Option<usize>,         // Start character offset
    pub end_offset: Option<usize>,           // End character offset

    // Model metadata (added OODA-02)
    pub llm_model: Option<String>,           // LLM used for extraction
    pub embedding_model: Option<String>,     // Embedding model used
    pub embedding_dimension: Option<usize>,  // Vector dimension
}
```

### DocumentLineage (`edgequake-pipeline/src/lineage.rs`)

```rust
pub struct DocumentLineage {
    pub document_id: String,
    pub document_name: String,
    pub job_id: String,
    pub chunks: Vec<ChunkLineage>,
    pub entities: HashMap<String, EntityLineage>,
    pub relationships: HashMap<String, RelationshipLineage>,
    pub total_chunks: usize,
    pub total_entities: usize,
    pub total_relationships: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // SPEC-032: Provider tracking
    pub extraction_provider: Option<String>,
    pub extraction_model: Option<String>,
    pub embedding_provider: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_dimension: Option<usize>,
}
```

---

## Storage Architecture

### KV Storage Keys

Lineage data is persisted in KV storage alongside documents and chunks:

```
┌──────────────────────────────────────────────────────────────┐
│  KV Storage Layout                                            │
├────────────────────────────┬─────────────────────────────────┤
│  Key Pattern               │  Value                           │
├────────────────────────────┼─────────────────────────────────┤
│  {document_id}             │  Document JSON                   │
│  {document_id}-metadata    │  Metadata JSON blob              │
│  {document_id}-lineage     │  DocumentLineage JSON            │
│  {document_id}-chunk-{N}   │  Chunk JSON (with position data) │
├────────────────────────────┼─────────────────────────────────┤
│  Vector Storage             │                                 │
├────────────────────────────┼─────────────────────────────────┤
│  {chunk_id}                │  Embedding + Chunk metadata      │
└────────────────────────────┴─────────────────────────────────┘
```

### Metadata Propagation Flow

```
PDF Upload
  │
  ▼
processor.rs ─── process_task()
  │
  ├─ 1. Extract text from PDF (pdfium)
  ├─ 2. Create Document with lineage metadata:
  │     - document_type = "pdf"
  │     - file_size = PDF file size
  │     - sha256_checksum = SHA-256 of PDF bytes
  │     - pdf_id = PdfDocument ID
  ├─ 3. Store metadata: KV.set("{doc_id}-metadata", metadata_json)
  ├─ 4. Chunk document with position tracking:
  │     - start_line, end_line per chunk
  │     - chunk_order_index
  ├─ 5. Store each chunk:
  │     - KV.set("{doc_id}-chunk-{N}", chunk_json)
  │     - Vector.upsert(chunk_id, embedding, metadata)
  ├─ 6. Entity extraction (LLM):
  │     - llm_model stamped on each chunk
  │     - embedding_model stamped on each chunk
  ├─ 7. Persist lineage:
  │     - KV.set("{doc_id}-lineage", lineage_json)
  │
  └─ 8. Update document status → Completed
```

---

## API Endpoints

### Document Lineage

```
GET /api/v1/documents/{document_id}/lineage
```

Returns complete lineage tree: document metadata + all chunks + all entities.

**Response** (`DocumentFullLineageResponse`):
```json
{
  "document_id": "abc123",
  "chunks": [
    {
      "chunk_id": "chunk-...",
      "chunk_index": 0,
      "content_preview": "First 200 chars...",
      "tokens": 150,
      "start_line": 1,
      "end_line": 25,
      "entity_count": 5
    }
  ],
  "entities": [
    {
      "entity_id": "entity-...",
      "entity_name": "SARAH_CHEN",
      "entity_type": "PERSON",
      "chunk_ids": ["chunk-..."]
    }
  ]
}
```

### Document Metadata

```
GET /api/v1/documents/{document_id}/metadata
```

Returns merged metadata from KV storage (`{doc_id}-metadata` key).

**Response**: Flat JSON object with all stored metadata fields.

### Chunk Lineage

```
GET /api/v1/chunks/{chunk_id}/lineage
```

Returns chunk with parent document references and position info.

**Response** (`ChunkLineageResponse`):
```json
{
  "chunk_id": "chunk-...",
  "document_id": "abc123",
  "chunk_index": 0,
  "content_preview": "...",
  "tokens": 150,
  "start_line": 1,
  "end_line": 25,
  "start_offset": 0,
  "end_offset": 1024,
  "llm_model": "gpt-5-nano",
  "embedding_model": "text-embedding-3-small",
  "embedding_dimension": 1536,
  "file_path": "/uploads/paper.pdf",
  "document_type": "pdf",
  "document_name": "paper.pdf",
  "created_at": "2025-01-15T10:30:00Z",
  "entity_count": 5
}
```

### Entity Provenance

```
GET /api/v1/entities/{entity_id}/provenance
```

Returns source chunks and documents for an entity.

---

## SDK Integration

All three SDKs expose identical lineage methods:

### Rust SDK

```rust
let lineage = client.documents().get_lineage(&doc_id).await?;
let metadata = client.documents().get_metadata(&doc_id).await?;
let chunk_lineage = client.chunks().get_lineage(&chunk_id).await?;
```

### TypeScript SDK

```typescript
const lineage = await client.documents.getLineage(docId);
const metadata = await client.documents.getMetadata(docId);
const chunkLineage = await client.chunks.getLineage(chunkId);
```

### Python SDK

```python
# Sync
lineage = client.documents.get_lineage(doc_id)
metadata = client.documents.get_metadata(doc_id)
chunk_lineage = client.chunks.get_lineage(chunk_id)

# Async
lineage = await client.documents.get_lineage(doc_id)
metadata = await client.documents.get_metadata(doc_id)
chunk_lineage = await client.chunks.get_lineage(chunk_id)
```

---

## WebUI Components

### MetadataSidebar

The metadata sidebar (`metadata-sidebar.tsx`) displays lineage information in three sections:

1. **Extended Metadata** (`enhanced-metadata.tsx`) — Fetches `/documents/:id/metadata` and displays KV fields not shown in the standard view
2. **Data Hierarchy** (`document-hierarchy-tree.tsx`) — Visual tree: Document → Chunks → Entities with collapsible nodes
3. **Source Info Grid** (`source-info-grid.tsx`) — Shows document_type, page_count, sha256_checksum, file_size_bytes

### React Query Hooks

```typescript
// Fetch full lineage tree
const { data } = useDocumentFullLineage(documentId);

// Fetch flat metadata
const { data } = useDocumentMetadata(documentId);

// Existing lineage explorer hook
const { data } = useDocumentLineage(documentId);
```

---

## Pipeline Integration

### Lineage Tracking Configuration

```rust
// Pipeline configuration (pipeline.rs)
pub struct PipelineConfig {
    pub enable_lineage_tracking: bool, // default: true (OODA-06)
    // ...
}
```

When `enable_lineage_tracking` is `true` (default), the pipeline:
1. Assigns position metadata (start_line, end_line) to each chunk
2. Records LLM/embedding model info on each chunk
3. Builds `DocumentLineage` with entity/relationship provenance
4. Persists lineage to KV storage

### SPEC-032: Provider Lineage

The lineage system tracks which LLM/embedding providers were used:

```
extraction_provider: "openai"       # e.g., openai, ollama
extraction_model: "gpt-5-nano"      # specific model version
embedding_provider: "openai"        # may differ from extraction
embedding_model: "text-embed-3-sm"  # embedding-specific model
embedding_dimension: 1536           # vector dimensions
```

This supports hybrid mode (SPEC-033) where extraction and embedding use different providers.

---

## Backward Compatibility

All lineage fields are `Option<T>` with `#[serde(default)]`:
- Documents created before lineage enhancement will have `None` for new fields
- API responses omit `null` fields via `skip_serializing_if = "Option::is_none"`
- SDKs use optional types in all languages
- No migration required — new fields are populated on next ingestion

---

## Performance Considerations

- **Single-call lineage**: `/documents/:id/lineage` returns complete tree (no N+1 queries)
- **KV storage**: Metadata stored alongside documents for O(1) lookup
- **Denormalized chunks**: Position metadata embedded in chunk, not separate table
- **Target latency**: P95 < 200ms for lineage queries on typical documents

---

## Related Specifications

- **SPEC-002**: Unified Ingestion Pipeline
- **SPEC-007**: PDF Upload Support with Vision LLM
- **SPEC-032**: Workspace-specific LLM/embedding providers
- **SPEC-033**: Hybrid provider mode
- **FEAT0011**: Document-Chunk-Entity Lineage tracking
- **FEAT0019**: Source span tracking with line numbers
- **BR0019**: Source spans must include line numbers
- **BR0701**: Lineage preserved for all entities
