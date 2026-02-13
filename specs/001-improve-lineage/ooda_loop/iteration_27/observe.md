# Observation - Iteration 27

## Mission Re-read
Re-read mission file. Focus: Entity provenance quality — F8 "chain is traceable in both directions."

## Files Examined
- `lineage.rs:325-435` — `get_entity_provenance` handler: resolves entity from graph, parses source_id, builds provenance response
- `lineage_types.rs:290-315` — `ChunkSourceInfo` with `start_line`/`end_line` as `Option<usize>`

## Current State
- Entity provenance returns `document_name: None` for all sources — UUIDs not user-friendly
- Chunk positions (`start_line`/`end_line`) not resolved from KV storage
- `get_all_edges()` call could be expensive on large graphs
