# Analysis - Iteration 05

## Gap: Position Metadata Lost at Storage Boundary

The chunker computes precise position metadata (start_line, end_line, start_offset, end_offset, token_count), but the processor drops these when serializing chunks to KV and vector storage. This makes it impossible to:
- Map a chunk back to its exact location in the source document
- Display "found at lines 42-58" in search results
- Build lineage trees with source position context

## Solution: Enrich chunk storage JSON

Add all 5 position fields to both KV and vector storage metadata. This is a pure additive change — existing consumers ignore unknown fields (backward compatible).

## Risk Assessment

- **Risk**: Low — no schema migration needed, JSON is schemaless in KV storage
- **Impact**: High — enables complete source traceability for every chunk
- **Effort**: Low — single function call modification in processor.rs
