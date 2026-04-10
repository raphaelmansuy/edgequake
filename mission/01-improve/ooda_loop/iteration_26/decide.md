# OODA-26 Decide: Add WHY Comments + Edge Case Tests

## Changes Planned

1. **context.rs** — Add WHY comments explaining:
   - Separate collections design
   - Incremental token counting
   - Section ordering in `to_context_string`
   - Add 8 edge case tests (empty context, degree display, description variants)

2. **context_filter.rs** — Add WHY comment explaining strict/lenient strategy

3. **error.rs** — Add WHY comment + 5 tests for Display impls

4. **lib.rs** — Add WHY comment about re-export facade pattern

5. **vector_filter.rs** — Add WHY comment + 3 edge case tests (as_str, non-string metadata)

## Estimated: ~16 new tests
