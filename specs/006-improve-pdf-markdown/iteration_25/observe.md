# OODA-25 Observe: Elements Module Test Coverage

## Current State

The `elements.rs` module (102 lines) has no unit tests despite containing helper methods that could be validated.

## File Analysis

- **Size**: 102 lines
- **Tests**: 0
- **Structures**: `RawChar`, `TextElement`, `PdfLine`
- **Methods**: `RawChar::width()`, `height()`, `center_x()`, `center_y()`

## Observations

1. The module is mostly data structures (derive Debug, Clone)
2. Has 4 helper methods on `RawChar` that compute geometric properties
3. These methods are used throughout the codebase for layout analysis
4. No edge case testing (e.g., zero-width chars, negative values)

## Current Test Count

- Total lib tests: 469
- elements.rs tests: 0

## Recommendation

Add unit tests for the helper methods:
- `width()` / `height()` computation
- `center_x()` / `center_y()` calculation
- Edge case: zero-width/height
- Edge case: large coordinates
