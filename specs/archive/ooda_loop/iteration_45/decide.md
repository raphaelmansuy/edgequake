# OODA-45: Decide - Implementation Plan

## Date: 2026-02-05

## Decision: Modular Extraction Strategy

Based on orient.md analysis, the 1362-line file can be cleanly split into:

| Module                | Lines | Functions                                 |
| --------------------- | ----- | ----------------------------------------- |
| `block_classifier.rs` | ~200  | Pattern matching, classification logic    |
| `column_detector.rs`  | ~150  | detect_columns, column grouping           |
| `reading_order.rs`    | ~100  | sort_blocks_reading_order, smart_sort_key |
| `pymupdf_grouper.rs`  | ~850  | Core grouper (chars→spans→lines→blocks)   |

---

## Extraction Order (safest first)

### Step 1: Extract helper functions (no API change)

Extract standalone functions that don't need `self`:

- `is_roman_numeral_header`
- `is_letter_subsection_header`
- `is_numeric_section_header`
- `is_numeric_subsection_header`
- `is_abstract_header`
- `is_bullet_item`
- `is_numbered_list_item`

These move to `block_classifier.rs` as public functions.

### Step 2: Extract block classification methods

Move `classify_blocks`, `classify_block` methods to `block_classifier.rs`.
Create a `BlockClassifier` struct that takes grouping params.

### Step 3: Extract column detection

Move `detect_columns`, `group_lines_by_column` to `column_detector.rs`.
Create a `ColumnDetector` struct.

### Step 4: Extract reading order

Move `sort_blocks_reading_order`, `compute_smart_sort_key`, `has_vertical_overlap`
to `reading_order.rs`.

---

## Implementation Steps

1. Create `block_classifier.rs` with helper functions
2. Create `column_detector.rs` with column detection
3. Create `reading_order.rs` with sorting
4. Update `pymupdf_grouper.rs` to import and use new modules
5. Update `layout/mod.rs` to export new modules
6. Run tests to verify no regression

---

## Success Criteria

- All 445 tests pass
- No clippy warnings
- Main file reduced from 1362 to ~850 lines
- Each new module is < 200 lines
