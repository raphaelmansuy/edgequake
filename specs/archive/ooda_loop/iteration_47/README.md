# OODA-47: Reading Order Module Enhancement

## Date: 2026-02-05 (Planned)

## Observe

`reading_order.rs` (744 lines) implements pymupdf4llm's smart sorting algorithm.

### Current State

- Uses `ReadingOrderDetector` struct
- Implements smart sort key computation
- Has boundary normalization (OODA-41)

### Needs

- ASCII diagram explaining the algorithm
- WHY comments for tolerances
- Better documentation for edge cases

## Orient

The module is functional but could benefit from:

1. Visual documentation of the sorting algorithm
2. Explanation of why certain blocks need reordering
3. Examples showing left-column-first reading order

## Decide

Add comprehensive documentation without changing algorithm.

## Act

**Status:** PLANNED

Changes to make:

1. Add ASCII diagram to module header
2. Add WHY comments to `BOUNDARY_ALIGNMENT_TOLERANCE`
3. Add examples in docstrings
