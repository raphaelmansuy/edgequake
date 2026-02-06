# OODA Iteration 15 - Orient

## Analysis of Observation

### Pattern Analysis

Standalone bold lines that should be headers share these characteristics:

- Bold formatting only (no mixed styles)
- Short text (< 60 characters typically)
- Title-like text (capitalized start)
- No trailing punctuation (., :, ?)
- Not a figure/table caption

### Strategic Options

#### Option A: Add Post-Processing in Markdown Renderer

- **Pros:** Clean separation, doesn't affect block detection
- **Cons:** Works on rendered output, less semantic info

#### Option B: Enhance Structure Detection Processor

- **Pros:** Semantic detection at block level
- **Cons:** May interfere with existing logic

#### Option C: Create New Processor

- **Pros:** Modular, testable, clear purpose
- **Cons:** Additional processing step

### Chosen Approach

**Option A** - Post-processing in markdown renderer

**Rationale:**

1. Simplest implementation
2. Works on final output where pattern is clear
3. No risk of disrupting existing block detection
4. Easy to test and refine

### Implementation Plan

1. In `cleanup_markdown_artifacts()` or new function
2. Detect pattern: `^\*\*[^*]+\*\*$` (standalone bold line)
3. Validate as header candidate:
   - Length < 60 chars
   - Starts with uppercase
   - No trailing `:` or `.` or `?`
   - Not caption pattern
4. Prefix with `## ` for standard section heading

### Test Plan

1. Test basic conversion: `**Title**` → `## **Title**`
2. Test rejection of captions: `**Fig. 1**` stays as is
3. Test rejection of labels: `**Note:**` stays as is
4. Run full test suite
5. Verify on AI_Services\_\_Elitizon.pdf
