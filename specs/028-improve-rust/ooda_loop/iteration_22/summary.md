# OODA Loop Iteration 22 - Add WHY Comments to Query Engine

**Date:** 2025-01-04
**Focus:** Add WHY comments to query modes and truncation
**Status:** ✅ Complete

## Observe

The query engine has complex mode selection and token budgeting logic that needs WHY documentation to explain:
- Why multiple query modes exist
- When to use each mode
- Why token budgeting matters
- How context is allocated

## Orient

Query mode selection and token budgeting are critical for:
1. **Query quality**: Wrong mode = poor results
2. **API costs**: Wasted tokens = higher costs
3. **Latency**: Over-fetching = slower responses
4. **Reliability**: Exceeding limits = errors

## Decide

Add WHY documentation to:
1. `modes.rs` - Explain when to use each query mode
2. `truncation.rs` - Explain token budgeting strategy

## Act

### Added WHY to Query Modes

[modes.rs](edgequake/crates/edgequake-query/src/modes.rs#L1-L32)

Key points documented:
- Question type → best mode mapping
- Performance vs accuracy trade-offs
- Why Hybrid is the default

### Added WHY to Token Truncation

[truncation.rs](edgequake/crates/edgequake-query/src/truncation.rs#L1-L34)

Key points documented:
- Why token limits matter (API errors, quality degradation)
- Budget allocation strategy (50/50 entities/relationships)
- Why order matters (relevance-sorted before truncation)

## Verify

```bash
cargo build --package edgequake-query
# Finished `dev` profile in 3.94s
```

## Code Added

| File | Lines | Content |
|------|-------|---------|
| modes.rs | 30 | Query mode selection guidelines |
| truncation.rs | 30 | Token budgeting rationale |
| **Total** | **60** | Design decision documentation |

## Knowledge Graph

```text
WHY Comments Added (OODA 21-22)
├── normalizer.rs (OODA-21)
│   └── Why normalization prevents graph fragmentation
├── parser.rs (OODA-21)
│   └── Why tuple format is more robust than JSON
├── modes.rs (OODA-22)
│   └── Why multiple query modes and when to use each
└── truncation.rs (OODA-22)
    └── Why token budgeting is critical
```
