# Decision - Iteration 14

## Changes
1. Add `DocumentFullLineage` and `ChunkLineageInfo` types to `operations.rs`
2. Add `get_lineage()` and `get_metadata()` to `DocumentsResource`
3. Add `get_lineage()` to `ChunksResource`

## Expected usage
```rust
let lineage = client.documents().get_lineage(&doc_id).await?;
let metadata = client.documents().get_metadata(&doc_id).await?;
let chunk_lineage = client.chunks().get_lineage(&chunk_id).await?;
```
