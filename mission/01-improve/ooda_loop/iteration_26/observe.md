# OODA-26 Observe: WHY Comments & Edge Cases for Query Crate

## Territory Mapped

Five files in `edgequake-query/src/` lack WHY comments:
- `context.rs` (435 lines) — Query context building; has 6 tests
- `context_filter.rs` (165 lines) — Post-retrieval document ID filtering; 4 tests
- `error.rs` (40 lines) — Query error types; 0 tests
- `lib.rs` (100 lines) — Crate root with re-exports; 0 tests
- `vector_filter.rs` (160 lines) — Vector result type filtering; 7 tests

## Observations

1. **context.rs**: Has functional doc comments but no WHY explaining design decisions (e.g., why entities/chunks/relationships are separate collections, why token_count is tracked incrementally).
2. **context_filter.rs**: Has good inline comments explaining chunk vs entity filtering strategy (strict vs lenient) but no WHY at module level.
3. **error.rs**: Minimal — just thiserror derive. No WHY about error variant choices.
4. **lib.rs**: Good module-level docs with FEAT references and architecture table. Missing WHY about re-export strategy.
5. **vector_filter.rs**: Has `@implements FEAT0110` but no WHY about type-string metadata convention.

## Edge Case Gaps

- **context.rs**: No test for empty context `to_context_string()`, relationship with/without description rendering, chunk token_count estimation, entity degree display logic.
- **error.rs**: No tests at all — Display impls untested.
- **vector_filter.rs**: No test for `VectorType::as_str` roundtrip, no test for non-string type metadata value.
