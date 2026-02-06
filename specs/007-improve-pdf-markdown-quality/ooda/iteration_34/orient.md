# IT34 — Orient

## Analysis

Production logging should follow the tracing level hierarchy:
```
┌─────────────────────────────────────────────┐
│  ERROR  → Failures that need attention      │
│  WARN   → Degraded behavior                 │
│  INFO   → High-level milestones only        │
│  DEBUG  → Algorithmic decisions             │
│  TRACE  → Per-block/per-item details        │
└─────────────────────────────────────────────┘
```

Most of the current INFO logs are algorithmic decisions (column counts, block grouping, table scanning) that belong at DEBUG or TRACE level.

## Files Affected

| File | INFO logs | Should be |
|------|-----------|-----------|
| column_detector.rs | 6 | debug |
| geometric.rs | 2 | debug |
| reading_order.rs | 5 | debug/trace |
| table_detection.rs | 10 | debug |
| layout_processing.rs | 8 | debug |
| structure_detection.rs | 3 | debug |
| markdown.rs (renderer) | 1 | trace |

## Clippy Fixes

All 5 warnings are auto-fixable with idiomatic Rust patterns:
- `.skip(1).next()` → `.nth(1)` 
- Manual prefix strip → `strip_prefix()`
- Manual suffix strip → `strip_suffix()`
- Doc comment formatting → add blank line separator
