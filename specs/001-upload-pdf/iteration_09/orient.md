# OODA-09: Orient

## Gap Analysis

| Current State                            | Desired State                                 | Gap           | Priority |
| ---------------------------------------- | --------------------------------------------- | ------------- | -------- |
| `extract_to_markdown()` without progress | `extract_to_markdown_with_progress(callback)` | Wire callback | HIGH     |
| No import of PipelineProgressCallback    | Import and create adapter                     | Add import    | HIGH     |
| 3 code paths call extract_to_markdown    | All 3 should use with_progress                | Update all    | HIGH     |

## Risk Assessment

- **Risk 1**: Changing async API might break behavior - Mitigation: Method has same signature plus callback
- **Risk 2**: Three code paths to update - Mitigation: Create helper function
- **Risk 3**: Vision path doesn't support progress yet - Mitigation: Leave as-is for now, log TODO

## First Principles Analysis

- **Core problem**: Extraction calls don't emit progress events
- **Fundamental constraint**: Must not break existing functionality
- **Minimal solution**: Add callback creation before each extract call
- **Why this matters**: Users get real-time page-by-page feedback

## Alternative Approaches

1. **Option A: Inline callback creation at each call site**
   - Pros: Simple, explicit
   - Cons: Code duplication across 3 paths

2. **Option B: Create callback once before if-else branches**
   - Pros: Single creation, reused
   - Cons: Callback created even if not used (minor)

3. **Option C: Helper method that creates callback and calls extractor**
   - Pros: DRY, clean
   - Cons: More indirection

**Chosen: Option B** - Create callback once before branches, reuse it
