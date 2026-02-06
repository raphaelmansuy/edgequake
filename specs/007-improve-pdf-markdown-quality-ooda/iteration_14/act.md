# OODA Iteration 14 - Act

## Actions Taken

### 1. Added `cleanup_toc_leader_dots()` Function

**File:** `src/renderers/markdown.rs`

Added new function to process text line-by-line and remove:

- Leader dots (4+ consecutive periods)
- Optional trailing page numbers
- Dots-only lines with formatting
- Standalone page numbers (2-3 digits)
- Empty bold patterns (`** **`)

### 2. Integrated into Cleanup Pipeline

Called from `cleanup_markdown_artifacts()` at the end of existing cleanup.

### 3. Added Test Coverage

7 new tests:

- `test_cleanup_toc_leader_dots_inline`
- `test_cleanup_toc_leader_dots_standalone`
- `test_cleanup_toc_leader_dots_page_number_only`
- `test_cleanup_toc_preserves_normal_dots`
- `test_cleanup_toc_preserves_ellipsis`
- `test_cleanup_toc_real_world_pattern`
- `test_cleanup_toc_preserves_line_breaks`

## Results

### Before

```markdown
5.1 - Actions ................................

5.2 - Operations ................................

**.............. 3**

**............. 5**

31

35
```

### After

```markdown
5.1 - Actions  
5.2 - Operations  
5.3 - Filters  
5.4 - Modifiers
5.5 Other keywords
```

### Test Results

```
test result: ok. 532 passed; 0 failed; 0 ignored
```

## Verification

1. ✅ Leader dots removed from all TOC entries
2. ✅ Section numbers preserved (5.1, 5.2, etc.)
3. ✅ Line breaks preserved between sections
4. ✅ Normal dots (ellipsis, etc.) preserved
5. ✅ All 532 tests passing
6. ✅ Apple-Sandbox-Guide-v1.0.pdf renders cleanly

## Commit

Ready for commit with message: "OODA-IT14: Remove TOC leader dots from PDF output"
