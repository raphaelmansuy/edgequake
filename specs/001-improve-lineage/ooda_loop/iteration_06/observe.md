# Observation - Iteration 06

## Files Examined

- `edgequake/crates/edgequake-pipeline/src/pipeline.rs` (lines 40-135, 1500-1580, 2140-2198)
  - `PipelineConfig` default: `enable_lineage_tracking: false` — lineage is OFF by default
  - `process_with_resilience()` (line 1507): builds `DocumentLineage` only when enabled
  - Lineage builder records chunks, entities, relationships with source spans

- `edgequake/crates/edgequake-api/src/processor.rs` (lines 1500-1575)
  - `result.lineage` from `ProcessingResult` is NEVER persisted to KV storage
  - Lineage data exists only in memory during processing, then is lost

## Key Gaps

1. **Lineage tracking disabled by default** — `enable_lineage_tracking: false` means no lineage is computed for any document unless explicitly enabled
2. **Lineage data not persisted** — Even when computed, `DocumentLineage` is not stored anywhere after processing completes
3. This breaks success criteria F5 ("Single API call retrieves complete document lineage tree") and F8 ("chain is traceable")
