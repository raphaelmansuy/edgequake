# Observation - Iteration 05

## Files Examined

- `edgequake/crates/edgequake-api/src/processor.rs` (lines 1130-1230)
  - Chunk KV storage (line ~1136): Only stores `content`, `document_id`, `index`
  - Chunk vector storage (line ~1220): Only stores `type`, `document_id`, `index`, `content`
  - `TextChunk` struct has: `start_line`, `end_line`, `start_offset`, `end_offset`, `token_count` — all populated by chunker

- `edgequake/crates/edgequake-pipeline/src/chunker.rs` (lines 120-200)
  - `TextChunk` struct already has complete position metadata
  - `with_line_numbers()` constructor sets `start_line`, `end_line`
  - `token_count` estimated during construction

## Key Gap

Position metadata fields exist on the pipeline's `TextChunk` struct but are **dropped** when storing chunks:

```
TextChunk (pipeline)           KV Storage (actual)
───────────────────            ──────────────────
id                     →       id (key)
content                →       content
document_id            →       document_id
index                  →       index
start_line             ✗       MISSING
end_line               ✗       MISSING
start_offset           ✗       MISSING
end_offset             ✗       MISSING
token_count            ✗       MISSING
```

Same gap exists in vector storage metadata for chunk embeddings.
