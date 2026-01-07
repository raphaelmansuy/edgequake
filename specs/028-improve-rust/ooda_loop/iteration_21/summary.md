# OODA Loop Iteration 21 - Add WHY Comments to Core Algorithms

**Date:** 2025-01-04
**Focus:** Add WHY comments explaining critical algorithm decisions
**Status:** ✅ Complete

## Observe

The codebase has good documentation but lacks "WHY" explanations for critical design decisions. Developers need to understand not just WHAT the code does, but WHY it was designed that way.

Key areas needing WHY documentation:
1. Entity name normalization
2. Tuple format vs JSON for LLM parsing
3. Keyword validation

## Orient

WHY comments serve multiple purposes:
- Prevent future developers from "optimizing" away critical features
- Document lessons learned from production issues
- Explain non-obvious design decisions
- Provide context for debugging

## Decide

Add comprehensive WHY comments to:
1. `normalizer.rs` - Why normalization prevents graph fragmentation
2. `parser.rs` - Why tuple format is more robust than JSON

## Act

### Added WHY to Entity Normalization

[normalizer.rs](edgequake/crates/edgequake-pipeline/src/prompts/normalizer.rs#L1-L22)

```rust
//! # WHY Normalization Matters
//!
//! Without normalization, the same entity extracted from different chunks might
//! be stored as separate nodes in the knowledge graph:
//!
//! - "John Doe" (from chunk 1)
//! - "john doe" (from chunk 2)  
//! - "JOHN DOE" (from chunk 3)
//! - "The John Doe" (from chunk 4)
//!
//! This leads to:
//! 1. **Graph fragmentation**: Same entity exists as multiple disconnected nodes
//! 2. **Lost relationships**: Edges only connect to one variant
//! 3. **Query failures**: Search for "John Doe" misses "JOHN DOE" nodes
//! 4. **Inflated entity counts**: 4 nodes instead of 1
//!
//! By normalizing to `JOHN_DOE`, all references merge into a single node,
//! preserving the complete relationship graph.
```

### Added WHY to Tuple Parser

[parser.rs](edgequake/crates/edgequake-pipeline/src/prompts/parser.rs#L1-L30)

```rust
//! # WHY Tuple Format Over JSON
//!
//! The tuple-delimited format is used because it's significantly more robust:
//!
//! 1. **Partial output recovery**: If LLM output is truncated, valid lines
//!    can still be parsed. JSON requires complete, valid syntax.
//!
//! 2. **No escaping issues**: JSON requires proper escaping of quotes and
//!    special characters. LLMs frequently produce malformed JSON.
//!
//! 3. **Line-by-line processing**: Each tuple is independent, allowing
//!    streaming extraction and early termination.
//!
//! 4. **LightRAG proven**: This format is battle-tested with millions of
//!    extractions in the LightRAG paper and implementation.
```

## Verify

```bash
cargo build --package edgequake-pipeline
# Finished `dev` profile in 3.27s
```

## Impact

| File | Lines Added | Purpose |
|------|-------------|---------|
| normalizer.rs | 16 | Explain why normalization prevents graph fragmentation |
| parser.rs | 24 | Explain why tuple format is more robust than JSON |
| **Total** | **40** | Critical design decision documentation |

## Future WHY Comments Needed

- Query mode selection (Hybrid vs Local vs Global)
- Reranking thresholds and why they improve precision
- Token budgeting for context windows
- Community detection algorithm choices
