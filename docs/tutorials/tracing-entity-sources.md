---
title: 'Tracing Entity Sources Tutorial'
---

# Tracing Entity Sources Tutorial

> Learn how to trace any entity in EdgeQuake's knowledge graph back to its source document and exact location

---

## Overview

EdgeQuake maintains a complete lineage chain for every entity in the knowledge graph:

```
PDF/Markdown File
  └─→ Document (with file metadata)
        └─→ Chunk (with line numbers)
              └─→ Entity (with extraction metadata)
```

This tutorial shows how to follow this chain in both directions — from entity back to source, and from document forward to entities.

---

## Prerequisites

- EdgeQuake backend running (`make dev` or `make dev-bg`)
- At least one document ingested
- An SDK installed (Rust, TypeScript, or Python) or use `curl`

---

## Step 1: Find an Entity

### Using the WebUI

1. Navigate to `http://localhost:3000/graph`
2. Click on any entity node in the graph visualization
3. Note the entity name (e.g., `SARAH_CHEN`)

### Using the API

List entities in a document:

```bash
curl http://localhost:8080/api/v1/lineage/documents/{document_id} | jq '.entities[].name'
```

---

## Step 2: Get Entity Provenance

The provenance endpoint tells you every source document and chunk where this entity was found.

### Using curl

```bash
curl http://localhost:8080/api/v1/entities/SARAH_CHEN/provenance | jq
```

### Using Python SDK

```python
from edgequake import EdgeQuake

client = EdgeQuake(base_url="http://localhost:8080")
provenance = client.operations.provenance.get("SARAH_CHEN")

print(f"Entity: {provenance.entity_name}")
print(f"Type: {provenance.entity_type}")
print(f"Extracted {provenance.total_extraction_count} times")

for source in provenance.sources:
    print(f"\nFrom document: {source.document_id}")
    for chunk in source.chunks:
        print(f"  Chunk: {chunk.chunk_id}")
        if chunk.start_line:
            print(f"  Lines: {chunk.start_line}-{chunk.end_line}")
```

### Expected Output

```
Entity: SARAH_CHEN
Type: PERSON
Extracted 3 times

From document: abc123def456
  Chunk: abc123def456-chunk-0
  Lines: 1-25
  Chunk: abc123def456-chunk-3
  Lines: 76-100
```

---

## Step 3: Trace to Source Chunk

Now drill into a specific chunk to see the exact text:

### Using curl

```bash
curl http://localhost:8080/api/v1/chunks/abc123def456-chunk-0 | jq
```

### Using TypeScript SDK

```typescript
import { EdgeQuake } from "@edgequake/sdk";

const client = new EdgeQuake({ baseUrl: "http://localhost:8080" });

// Get chunk detail
const chunk = await client.chunks.get("abc123def456-chunk-0");
console.log(`Content: ${chunk.content}`);
console.log(`Lines: ${chunk.char_range.start}-${chunk.char_range.end}`);
console.log(`Entities found: ${chunk.entities.length}`);

for (const entity of chunk.entities) {
  console.log(`  ${entity.name} (${entity.entity_type})`);
}
```

---

## Step 4: Get Full Chunk Lineage

The chunk lineage endpoint provides everything in one call — parent document info, position data, and model provenance:

### Using curl

```bash
curl http://localhost:8080/api/v1/chunks/abc123def456-chunk-0/lineage | jq
```

### Using Rust SDK

```rust
use edgequake_sdk::EdgeQuake;

let client = EdgeQuake::new("http://localhost:8080")?;
let lineage = client.chunks().get_lineage("abc123def456-chunk-0").await?;

println!("Document: {}", lineage.document_id);
println!("Document type: {:?}", lineage.document_type);
println!("Chunk index: {}", lineage.chunk_index.unwrap_or(0));
println!("Lines: {:?}-{:?}", lineage.start_line, lineage.end_line);
println!("LLM model: {:?}", lineage.llm_model);
println!("Embedding model: {:?}", lineage.embedding_model);
println!("Entities: {:?}", lineage.entity_names);
```

---

## Step 5: View Complete Document Lineage

Get the full picture — all chunks and entities for a document:

### Using curl

```bash
curl http://localhost:8080/api/v1/documents/abc123def456/lineage | jq
```

### Using Python SDK

```python
from edgequake import EdgeQuake

client = EdgeQuake(base_url="http://localhost:8080")
lineage = client.documents.get_lineage("abc123def456")

print(f"Document: {lineage.document_id}")
print(f"Total chunks: {len(lineage.chunks)}")
print(f"Total entities: {len(lineage.entities)}")

print("\nChunks:")
for chunk in lineage.chunks:
    print(f"  [{chunk.get('chunk_index', '?')}] {chunk.get('content_preview', '')[:50]}...")
    print(f"      Lines {chunk.get('start_line', '?')}-{chunk.get('end_line', '?')}, "
          f"Entities: {chunk.get('entity_count', 0)}")

print("\nEntities:")
for entity in lineage.entities:
    print(f"  {entity.get('entity_name', '?')} ({entity.get('entity_type', '?')})")
    print(f"      From chunks: {entity.get('chunk_ids', [])}")
```

---

## Step 6: Get Document Metadata

Additional metadata (file hash, size, processing info) is available:

```bash
curl http://localhost:8080/api/v1/documents/abc123def456/metadata | jq
```

Example response:

```json
{
  "id": "abc123def456",
  "title": "research_paper.pdf",
  "document_type": "pdf",
  "file_size_bytes": 2097152,
  "sha256_checksum": "e3b0c44298fc1c149...",
  "page_count": 24,
  "llm_model": "gpt-5-nano",
  "embedding_model": "text-embedding-3-small"
}
```

---

## Using the WebUI

### Metadata Sidebar

1. Open a document at `http://localhost:3000/documents/{id}`
2. The **Metadata Sidebar** shows:
   - **Extended Metadata** — All stored KV metadata fields
   - **Data Hierarchy** — Interactive tree: Document → Chunks → Entities
   - **Source Info** — Document type, page count, checksum, file size

### Lineage Explorer

1. Navigate to `http://localhost:3000/lineage`
2. Select a document from the list
3. Browse the entity graph with source traceability
4. Click any entity to see its provenance

---

## Complete Traceability Example

Here's a complete workflow tracing the entity `QUANTUM_COMPUTING` back to its source:

```
1. Entity: QUANTUM_COMPUTING (TECHNOLOGY)
   │
   ├─ Provenance: Extracted 5 times from 2 documents
   │
   ├─ Document A: "quantum_review.pdf"
   │   ├─ Chunk 0 (lines 1-30): "Quantum computing represents..."
   │   ├─ Chunk 3 (lines 91-120): "Recent advances in quantum..."
   │   └─ Chunk 7 (lines 211-240): "Applications of quantum..."
   │
   └─ Document B: "tech_survey.md"
       ├─ Chunk 1 (lines 15-45): "Among emerging technologies..."
       └─ Chunk 4 (lines 100-130): "Quantum supremacy was..."
```

Each level provides:
- **Entity**: Name, type, description, related entities
- **Document**: File metadata, processing info, models used
- **Chunk**: Exact text, line numbers, character offsets, token count
- **Models**: Which LLM extracted entities, which embedding model vectorized

---

## Summary

| What you want to know            | Endpoint                                | SDK Method                    |
| -------------------------------- | --------------------------------------- | ----------------------------- |
| Where was this entity found?     | `GET /entities/{id}/provenance`         | `operations.provenance.get()` |
| What entities are in this doc?   | `GET /lineage/documents/{id}`           | `documents.get_lineage()`     |
| What's in this chunk?            | `GET /chunks/{id}`                      | `chunks.get()`                |
| Full chunk lineage chain?        | `GET /chunks/{id}/lineage`              | `chunks.get_lineage()`        |
| All document metadata?           | `GET /documents/{id}/metadata`          | `documents.get_metadata()`    |
| Complete lineage tree?           | `GET /documents/{id}/lineage`           | `documents.get_lineage()`     |
