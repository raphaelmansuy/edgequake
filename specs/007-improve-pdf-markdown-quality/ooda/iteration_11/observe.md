# IT11 Observe: Code Quality and Dead Code Analysis

## Mission Re-read ✓

Re-read `specs/007-improve-pdf-markdown-quality.md` at start of iteration.

## Focus

Per mission constraints:

- "Eliminate dead code - Remove unused functions, imports, and modules"
- "Apply DRY principle - Don't Repeat Yourself; extract common patterns"
- "DEAD CODE: If it's not used, delete it"

## Observation: is_likely_table Analysis

### CORRECTION: Two Different Functions (Both Used)

1. `ColumnDetector::is_likely_table` in `layout/column_detector.rs` → Used by `layout_processing.rs`
2. `TableDetectionProcessor::is_likely_table` in `table_detection.rs` → Used internally

Both are in use - NOT dead code. Previous observation was incorrect.

## Observation: Test Coverage Gaps

### Current Test Count

- 517 tests passing
- Need to verify test coverage for recent changes

### Missing Tests

1. No test for `is_table_reference` logic (the "Table N presents..." detection)
2. No integration test for Table 4 reconstruction

## Observation: File Size Analysis

```
table_detection.rs: 1,200+ lines
```

This module handles:

- TableDetectionProcessor (spatial detection)
- TextTableReconstructionProcessor (text pattern detection)
- Multiple helper functions

Per SRP, could be split into:

- `processors/table_spatial.rs` - TableDetectionProcessor
- `processors/table_text.rs` - TextTableReconstructionProcessor
- `processors/table_common.rs` - Shared utilities

## Observation: Remaining Quality Gaps

From LightRAG markdown output analysis:

### Lists (55→85 target)

Looking at output, lists like:

- `•**Agriculture**: This domain focuses on...`

The bullet point handling looks OK but nested lists need review.

### Multi-column (60→85 target)

Tables 1-5 reconstruction status:

- Table 3: ✓ Reconstructed (24 children)
- Table 4: ✓ Reconstructed (10 children) - IT10 fix
- Table 5: ✗ "No table content found"

### Formatting artifacts

Some output has artifacts like:

- `**First**,` - comma attached to bold
- `**Second**,` - same issue
- Inconsistent spacing

## Next Steps

Priority for IT11:

1. Remove dead code (`is_likely_table`)
2. Add test for `is_table_reference`
3. Clean up debug logging
