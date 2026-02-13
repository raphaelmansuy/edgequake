# Implementation - Iteration 06

## Changes Made

1. **File**: `edgequake/crates/edgequake-pipeline/src/pipeline.rs`
   - Line 125: Changed `enable_lineage_tracking: false` → `true` with WHY comment
   - Lines 2156: Updated `test_pipeline_config_defaults` assertion
   - Lines 2165-2198: Updated `test_pipeline_with_lineage_tracking` and `test_pipeline_without_lineage_tracking` for new defaults

2. **File**: `edgequake/crates/edgequake-api/src/processor.rs`
   - Lines ~1550-1585: Added lineage persistence block after `update_document_status_with_stats`
   - Persists `DocumentLineage` to `{document_id}-lineage` KV key
   - Includes info/warn logging for success/failure

## Verification

- `cargo build -p edgequake-api -p edgequake-pipeline`: ✅ Clean build
- `cargo test --workspace --lib`: ✅ 1698 passed, 0 failed
