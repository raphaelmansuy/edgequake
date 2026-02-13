# Decision - Iteration 06

## Changes

1. **pipeline.rs:125** — Change `enable_lineage_tracking: false` → `true` in default config
2. **pipeline.rs:2156** — Update `test_pipeline_config_defaults` to assert `enable_lineage_tracking` is true
3. **pipeline.rs:2165-2198** — Update lineage tracking tests for new default
4. **processor.rs:1550** — Add lineage persistence block: serialize `DocumentLineage` to `{doc_id}-lineage` KV key after processing

## Expected Outcome

Every processed document will have its complete lineage tree stored in KV storage, enabling lineage API endpoints to return full provenance data in a single query.
