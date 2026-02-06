# OODA Iteration 14 - Decide

## Decision

### Chosen Solution

Implement line-by-line TOC leader dots cleanup in `cleanup_toc_leader_dots()` function.

### Implementation Details

1. **Pattern Recognition:**
   - Leader dots: 4+ consecutive periods (`\.{4,}`)
   - Optional trailing page number (1-3 digits)
   - Dots-only lines with optional page numbers
   - Standalone page numbers (2-3 digits)

2. **Line-by-line Processing:**
   - Process each line independently
   - Avoid cross-line regex matching that consumes content
   - Skip empty/artifact lines
   - Preserve line breaks between sections

3. **Key Patterns:**

   ```rust
   // Remove dots and trailing page number
   let leader_dots_re = Regex::new(r"\.{4,}\s*\d{0,3}\s*$").unwrap();

   // Skip dots-only lines
   let dots_only_re = Regex::new(r"^\s*\**\.{3,}\**\s*\d*\s*\**\s*$").unwrap();

   // Skip standalone page numbers (2-3 digits only)
   let page_num_only_re = Regex::new(r"^\s*\d{2,3}\s*$").unwrap();
   ```

### Risk Assessment

| Risk                     | Mitigation                               |
| ------------------------ | ---------------------------------------- |
| Breaking normal dots     | Only match 4+ consecutive dots           |
| Removing section numbers | Only remove 2-3 digit standalone numbers |
| Cross-line merging       | Line-by-line processing                  |

### Test Coverage

- 7 new tests added
- All 532 tests passing
