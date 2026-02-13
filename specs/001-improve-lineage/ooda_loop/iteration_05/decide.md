# Decision - Iteration 05

## Changes to Make

1. **processor.rs:1136** — Enhance chunk KV storage JSON to include `start_line`, `end_line`, `start_offset`, `end_offset`, `token_count`
2. **processor.rs:1220** — Enhance chunk vector storage metadata to include same position fields

## Expected Outcome

Every chunk stored in both KV and vector storage now carries complete position metadata, enabling:
- Entity → chunk → source line mapping
- "Found at lines X-Y" in search results
- Complete lineage tree with source positions
