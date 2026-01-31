# Iteration 04: Orient

## Gap Analysis

| Current State | Desired State | Gap | Priority |
|--------------|---------------|-----|----------|
| `extract_to_markdown()` has no callback | `extract_to_markdown_with_progress()` calls backend with callback | Add new method | HIGH |
| Callers can't get page-level progress | Callers pass `Arc<dyn ProgressCallback>` | Wire through to backend | HIGH |

## Risk Assessment

- **Risk 1**: Breaking API change - Mitigation: Add new method, don't modify existing
- **Risk 2**: Callback not reaching backend - Mitigation: Test with CountingProgress

## First Principles Analysis

- **Core problem**: `PdfExtractor` is the public API but doesn't expose progress
- **Fundamental constraint**: Backend already supports progress (OODA-03)
- **Minimal solution**: Add thin wrapper method that passes callback through
- **Why this matters**: This is the API callers use; backend alone isn't enough

## Alternative Approaches

1. **Option A: Add callback to existing methods** ✅ CHOSEN
   - Pros: Direct, simple
   - Cons: New method, more API surface

2. **Option B: Builder pattern with progress**
   - Pros: Fluent API
   - Cons: More complex, breaks existing patterns

## Decision

Add `extract_to_markdown_with_progress()` as new public method.
