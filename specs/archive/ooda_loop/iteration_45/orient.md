# OODA-45: Orient - Refactoring Strategy

## Date: 2026-02-05

## Analysis

The 1362-line file violates SRP (Single Responsibility Principle) because it handles:

1. Character → Span grouping
2. Span → Line grouping
3. Line → Block grouping
4. Column detection
5. Block classification
6. Configuration

---

## Decision: Minimal Disruption Approach

Instead of creating a subdirectory (which would require significant import changes), we'll:

1. **Keep pymupdf_grouper.rs** as the main file
2. **Extract specific functions** to helper modules
3. **Re-export** from the main module

This preserves existing imports while improving organization.

---

## Implementation Plan

```text
Current:
└── layout/
    ├── mod.rs
    └── pymupdf_grouper.rs  (1362 lines)

After:
└── layout/
    ├── mod.rs
    ├── pymupdf_grouper.rs  (~800 lines) - Core grouper
    ├── column_splitter.rs  (~250 lines) - Column detection
    └── block_classifier.rs (~200 lines) - Type classification
```

---

## Risk Assessment

- **Low risk**: Keep public API unchanged
- **Testing**: All existing tests should pass
- **Imports**: No changes needed for external users

---

## First Step

Extract `classify_blocks()` and related functions to `block_classifier.rs`.
This is the clearest SRP violation (classification ≠ grouping).
