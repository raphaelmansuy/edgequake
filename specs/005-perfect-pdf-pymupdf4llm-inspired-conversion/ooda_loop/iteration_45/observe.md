# OODA-45: Observe - pymupdf_grouper.rs Analysis

## Date: 2026-02-05

## File Statistics

```
File: edgequake/crates/edgequake-pdf/src/layout/pymupdf_grouper.rs
Lines: 1362
Functions: ~25
Responsibility: Grouping RawChar → Span → Line → Block
```

---

## Current Structure

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    pymupdf_grouper.rs (1362 lines)                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TextGrouper struct                                                         │
│  ├─ chars_to_spans()     - Character → Span grouping (~150 lines)          │
│  ├─ spans_to_lines()     - Span → Line grouping (~100 lines)               │
│  ├─ lines_to_blocks()    - Line → Block grouping (~200 lines)              │
│  ├─ detect_columns()     - Column layout detection (~150 lines)            │
│  ├─ split_multi_col()    - Multi-column splitting (~100 lines)             │
│  ├─ classify_blocks()    - Block type classification (~200 lines)          │
│  └─ ... utility methods                                                     │
│                                                                             │
│  GroupingParams struct (~50 lines)                                          │
│  Tests (~350 lines)                                                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## SRP Violations

1. **Grouping logic mixed with classification**
   - `classify_blocks()` should be separate module

2. **Column detection embedded in grouper**
   - `detect_columns()`, `split_multi_col()` are layout concerns

3. **Large test suite at bottom**
   - Tests could be in separate file

---

## Proposed Split

```text
BEFORE: pymupdf_grouper.rs (1362 lines)

AFTER:
├── grouper/
│   ├── mod.rs             - Re-exports
│   ├── text_grouper.rs    - Core TextGrouper struct (~400 lines)
│   ├── span_builder.rs    - chars_to_spans logic (~150 lines)
│   ├── line_builder.rs    - spans_to_lines logic (~150 lines)
│   ├─ block_builder.rs   - lines_to_blocks logic (~200 lines)
│   ├── column_splitter.rs - detect_columns, split_multi_col (~250 lines)
│   └── classifier.rs      - classify_blocks logic (~200 lines)
│
├── tests/
│   └── grouper_tests.rs   - All grouper tests (~350 lines)
```

---

## Benefits

1. **Single Responsibility**: Each file has one job
2. **Testability**: Can test span building independently
3. **Maintainability**: Changes to columns don't affect classification
4. **Navigation**: Easier to find relevant code
5. **Parallel Development**: Different people can work on different parts
