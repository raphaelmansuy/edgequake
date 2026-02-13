# Analysis - Iteration 01

## Gaps Identified

1. **GAP-01: Chunk struct lacks position metadata** — `Chunk` in core types has no start_line/end_line/start_offset/end_offset. ChunkLineage has them, but they aren't stored AT the chunk level for direct retrieval.
2. **GAP-02: Chunk struct lacks model metadata** — No embedding_model or llm_model on the Chunk struct itself; only in DocumentLineage at doc level.
3. **GAP-03: Document struct lacks type-safe lineage fields** — document_type, sha256_checksum, file_size are not first-class fields. The JSON metadata blob is underutilized.
4. **GAP-04: No bidirectional PDF↔Document link** — PdfDocument.document_id points to Document, but Document has no pdf_id pointing back.
5. **GAP-05: API returns None for extraction_metadata** — ChunkDetailResponse.extraction_metadata is hardcoded to None. Metadata not stored during extraction for retrieval.
6. **GAP-06: No consolidated lineage API** — Need single GET /documents/{id}/lineage returning full tree. Current implementation requires multiple calls.
7. **GAP-07: SDKs have zero lineage methods** — Types exist in TS SDK but no methods to call lineage endpoints.
8. **GAP-08: WebUI lineage display depends on populated data** — If API doesn't populate lineage in document responses, UI shows nothing.

## Possible Solutions

### Solution A: Incremental Enhancement (Bottom-Up)

- Add missing fields to Chunk and Document structs
- Ensure pipeline populates them
- Then enhance API and SDKs

Pros: Safe, backward compatible, testable at each step
Cons: Many iterations before visible impact
Risk: Low

### Solution B: API-First Enhancement (Top-Down)

- Design ideal API responses first
- Then work backward to populate them
- SDKs and UI benefit immediately

Pros: Clear target, UI benefits early
Cons: May need placeholder data initially
Risk: Medium

### Solution C: Full Rewrite of Lineage System

- Redesign from scratch with all fields
- Replace existing lineage types

Pros: Clean architecture
Cons: High risk, breaks existing functionality
Risk: High

## Recommendation

**Solution A: Incremental Enhancement (Bottom-Up)** — Start from core types, work outward. This approach ensures data integrity at each layer before exposing it through API and UI. Each step is independently testable and committable.

## Priority Order for Next Iterations

1. Add position metadata to Chunk struct (GAP-01) — enables chunk-level traceability
2. Add model tracking to Chunk struct (GAP-02) — enables per-chunk lineage
3. Add type-safe metadata to Document (GAP-03) — explicit fields > JSON blob
4. Add pdf_id to Document for bidirectional link (GAP-04) — complete lineage chain
5. Store extraction_metadata during pipeline processing (GAP-05) — populate real data
6. Add consolidated lineage API endpoint (GAP-06) — single-call retrieval
7. Add SDK lineage methods (GAP-07) — developer access
8. Validate WebUI displays all fields (GAP-08) — end-user visibility
