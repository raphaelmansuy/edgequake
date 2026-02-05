# OODA-25 Orient: Analysis of Elements Module Testing Gap

## Context

The `elements.rs` module defines core data structures used throughout the PDF extraction pipeline. The helper methods are simple but fundamental:
- Layout algorithms depend on `width()` and `height()`
- Spatial indexing uses `center_x()` and `center_y()`

## Risk Assessment

| Factor | Risk | Mitigation |
|--------|------|------------|
| Simple math | Low | Tests will document expected behavior |
| Used everywhere | Medium | Catching edge cases prevents cascading bugs |
| Trivial implementation | Low | Tests serve as documentation |

## Decision Factors

**Worth testing because:**
1. The methods are used throughout the pipeline
2. Edge cases (zero-size, negative) could propagate
3. Tests document the expected behavior for future changes

**Test strategy:**
- Basic functionality tests
- Edge case: zero dimensions
- Edge case: large PDF coordinates (real docs use ~612x792 pts)

## Alignment with Mission

Mission 006 goals:
- ✅ Improve test coverage → Adding tests to untested module
- ✅ Clean code → Tests serve as documentation
- ✅ Quality extraction → Validates geometric helpers

## Decision

Add 4-5 unit tests for `RawChar` helper methods covering basic cases and edges.
