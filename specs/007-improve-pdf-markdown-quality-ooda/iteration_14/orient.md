# OODA Iteration 14 - Orient

## Analysis of Observation

### Pattern Analysis

TOC leader dots appear in several forms:

1. **Inline with text:** `Actions  ................................`
2. **Standalone dots:** `**.............. 3**`
3. **Dots with page numbers:** `...............  36`

### Strategic Options

#### Option A: Add Regex Cleanup in `cleanup_markdown_artifacts()`

- **Pros:** Simple, centralized, handles all cases
- **Cons:** Post-hoc cleanup, doesn't fix root cause
- **Risk:** Low - only affects formatting

#### Option B: Detect TOC Blocks Early

- **Pros:** Better semantic understanding
- **Cons:** Complex heuristics needed
- **Risk:** Medium - false positives possible

#### Option C: Both (Hybrid Approach)

- Add cleanup for dots patterns (immediate fix)
- Mark TOC blocks as artifacts (future enhancement)

### Chosen Approach

**Option A** - Add regex cleanup patterns for leader dots

**Rationale:**

1. Simple and effective
2. Low risk of side effects
3. Covers multiple document types
4. Can be enhanced later with semantic detection

### Patterns to Clean

1. `\.{4,}` - 4+ consecutive dots (leader pattern)
2. `\s+\d{1,3}\s*$` after dot removal - trailing page numbers

### Test Plan

1. Run on Apple-Sandbox-Guide-v1.0.pdf
2. Verify no leader dots in output
3. Ensure regular text with dots (e.g., "etc.") is preserved
4. Run full test suite
