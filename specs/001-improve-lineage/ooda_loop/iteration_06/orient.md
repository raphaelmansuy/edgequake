# Analysis - Iteration 06

## Root Cause

Lineage tracking was designed as opt-in (`enable_lineage_tracking: false`) for performance reasons, but the overhead is minimal (in-memory tree built during processing). Without it, the entire lineage API returns empty data.

## Solution

1. **Enable by default** — Set `enable_lineage_tracking: true` in `PipelineConfig::default()`
2. **Persist to KV storage** — After processing, serialize `DocumentLineage` to `{document_id}-lineage` key
3. **Update tests** — Fix any tests that assert lineage is disabled by default

## Risk Assessment

- **Performance impact**: Negligible — lineage builder adds O(n) work where n = chunks + entities
- **Storage impact**: Low — lineage JSON is typically 5-20KB per document
- **Backward compatibility**: Full — old documents simply won't have a `-lineage` key
