# OODA-31 Observe: Reading Order Module Coverage

## Current State

| Metric          | Value |
| --------------- | ----- |
| Total Lib Tests | 490   |
| Clippy Warnings | 0     |
| Iteration       | 31    |

## Module Analysis

### reading_order.rs

- **Lines**: 743
- **Tests**: 6
- **WHY Comments**: 7
- **Tests per 100 lines**: 0.8 (low)

### Test Coverage Gap Analysis

Low coverage modules (tests/100 lines < 1.0):

| Module                 | Lines | Tests | Ratio |
| ---------------------- | ----- | ----- | ----- |
| pymupdf_structs.rs     | 1010  | 5     | 0.5   |
| extraction_engine.rs   | 1346  | 6     | 0.4   |
| structure_detection.rs | 882   | 4     | 0.5   |
| **reading_order.rs**   | 743   | 6     | 0.8   |

## Focus: reading_order.rs

The reading order module determines how PDF blocks should be ordered for reading.
It handles column detection, region merging, and topological sorting.

### Current Tests (6)

Need to identify what's tested and what gaps exist.

### Key Functionality

1. Column detection
2. Region creation and merging
3. Topological ordering
4. Edge case handling
