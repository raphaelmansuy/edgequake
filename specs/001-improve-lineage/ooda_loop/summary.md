# OODA Loop Summary — Lineage Audit Mission

## Mission: Comprehensive Lineage Extraction & Metadata Audit
**Branch**: `feat/improve-lineage`
**Started**: OODA-01
**Last Updated**: OODA-09

---

## Current State Overview

### Data Flow Diagram

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  PDF Upload  │     │   Markdown   │     │  Text Insert │     │   Scan Dir   │
│  (multipart) │     │   Upload     │     │   (JSON)     │     │  (batch)     │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │                    │
       ▼                    ▼                    ▼                    ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                         DOCUMENT METADATA (KV)                              │
│  Key: {document_id}-metadata                                                │
│  Fields: id, title, status, source_type, content_length,                    │
│          file_size_bytes, sha256_checksum, document_type, page_count        │
│          (OODA-04: Added file_size, checksum, doc_type, page_count)         │
└──────────────────────────────┬───────────────────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                         CHUNKING (Pipeline)                                  │
│  TextChunk: content, start_line, end_line, start_offset, end_offset,        │
│            token_count, embedding                                           │
└──────────────────────────────┬───────────────────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                         CHUNK STORAGE (KV + Vector)                          │
│  KV Key: {doc_id}-chunk-{N}                                                 │
│  Fields: content, document_id, index, start_line, end_line,                 │
│          start_offset, end_offset, token_count                              │
│          (OODA-05: Added all 5 position fields to KV & vector metadata)     │
└──────────────────────────────┬───────────────────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                    ENTITY EXTRACTION (Pipeline + Graph)                       │
│  Graph: node.source_id = "doc_id-chunk-N|doc_id-chunk-M"                    │
│  Graph: edge.source_id = "doc_id-chunk-N"                                   │
│  (OODA-06: Lineage tracking enabled by default)                             │
└──────────────────────────────┬───────────────────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                     LINEAGE STORAGE (KV)                                     │
│  Key: {document_id}-lineage                                                 │
│  Value: DocumentLineage JSON (chunks, entities, relationships,              │
│         extraction provider/model, embedding provider/model)                │
│  (OODA-06: Persistence added — was in-memory only)                          │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Metadata Tracking Status

| Level | Field | Status | OODA |
|---|---|---|---|
| **Document** | document_id | ✅ Pre-existing | — |
| **Document** | file_path | ✅ Pre-existing | — |
| **Document** | file_size_bytes | ✅ Added | OODA-04 |
| **Document** | document_type | ✅ Added | OODA-04 |
| **Document** | sha256_checksum | ✅ Added | OODA-04 |
| **Document** | page_count (PDF) | ✅ Added | OODA-04 |
| **Document** | processed_at | ✅ Added | OODA-03 |
| **Document** | llm_model | ✅ Core type | OODA-03 |
| **Document** | embedding_model | ✅ Core type | OODA-03 |
| **Chunk** | chunk_id | ✅ Pre-existing | — |
| **Chunk** | parent_document_id | ✅ Pre-existing | — |
| **Chunk** | index | ✅ Pre-existing | — |
| **Chunk** | start_line | ✅ Added to KV | OODA-05 |
| **Chunk** | end_line | ✅ Added to KV | OODA-05 |
| **Chunk** | start_offset | ✅ Added to KV | OODA-05 |
| **Chunk** | end_offset | ✅ Added to KV | OODA-05 |
| **Chunk** | token_count | ✅ Added to KV | OODA-05 |
| **Chunk** | llm_model | ✅ Core type | OODA-02 |
| **Chunk** | embedding_model | ✅ Core type | OODA-02 |
| **Lineage** | extraction_provider | ✅ Persisted | OODA-06 |
| **Lineage** | extraction_model | ✅ Persisted | OODA-06 |
| **Lineage** | embedding_provider | ✅ Persisted | OODA-06 |
| **Lineage** | embedding_model | ✅ Persisted | OODA-06 |
| **Entity** | entity_id | ✅ Pre-existing | — |
| **Entity** | chunk_ids (source_id) | ✅ Pre-existing | — |

---

## API Endpoint Status

| Endpoint | Status | OODA |
|---|---|---|
| `GET /documents/:id/lineage` | ✅ Implemented | OODA-07 |
| `GET /documents/:id/metadata` | ✅ Implemented | OODA-07 |
| `GET /chunks/:id` | ✅ Enhanced | OODA-07 |
| `GET /chunks/:id/lineage` | ✅ Implemented | OODA-08 |
| `GET /entities/:id/provenance` | ✅ Pre-existing | — |
| `GET /lineage/entities/:name` | ✅ Pre-existing | — |
| `GET /lineage/documents/:id` | ✅ Pre-existing | — |

---

## Iteration Log

| # | Focus | Commit | Tests |
|---|---|---|---|
| 01 | Chunk position metadata (core type) | 3bc77469 | 1698 |
| 02 | Chunk model tracking (core type) | c66a048a | 1698 |
| 03 | Document lineage metadata (core type) | 7fcf2f26 | 1698 |
| 04 | PDF↔Doc bidirectional link (processor) | 686d83c5 | 1698 |
| 05 | Chunk metadata propagation (KV+vector) | 3bc46176 | 1698 |
| 06 | Lineage persistence (KV storage) | a8730ff4 | 1698 |
| 07 | API endpoints: /documents/:id/lineage+metadata | 73ed518a | 1698 |
| 08 | API endpoint: /chunks/:id/lineage | 364f09da | 1698 |
| 09 | Gap analysis + DTO tests | (pending) | 1702 |

---

## Remaining Work

### Phase 3 (Iterations 10-14): WebUI Enhancement
- Update `MetadataSidebar` to show all lineage fields
- Add document lineage tree visualization
- Add chunk position display
- Source traceability click-through

### Phase 4 (Iterations 15-17): SDK Updates
- Rust SDK: `get_lineage()`, `get_metadata()`, `get_chunk_lineage()`
- TypeScript SDK: same methods
- Python SDK: same methods

### Phase 5 (Iterations 18-20): Documentation
- `docs/architecture/lineage-tracking.md`
- `docs/api-reference/lineage-endpoints.md`
- `docs/tutorials/tracing-entity-sources.md`
- `docs/operations/metadata-debugging.md`

### Phase 6 (Iterations 21-30): Validation & Polish
- Performance benchmarks
- E2E tests
- Migration guide
- CHANGELOG update
