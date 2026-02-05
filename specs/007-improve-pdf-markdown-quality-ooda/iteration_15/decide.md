# OODA Iteration 15 - Decide

## Decision

### Chosen Solution
Post-process markdown output to convert standalone bold lines to section headers.

### Implementation Details

1. **Pattern Detection:** `^\*\*([^*]+)\*\*\s*$`
   - Matches lines containing ONLY bold text

2. **Header Criteria:**
   - Length < 60 characters
   - Starts with uppercase letter
   - Does NOT end with: `:`, `.`, `?`, `;`
   - NOT a figure/table caption (e.g., "Figure 1:", "Table 2")

3. **Allowed Exceptions:**
   - "Table of Contents" - despite starting with "Table"
   - "Appendix" - despite common caption patterns
   - "Acknowledgements/Acknowledgments"

4. **Output:** Prefix with `## ` for H2 level headers

### Risk Assessment

| Risk | Mitigation |
|------|------------|
| False positives | Strict criteria (length, punctuation, case) |
| Missing headers | Allowed exceptions list |
| Breaking captions | Regex checks for "Figure N", "Table N" patterns |

### Test Coverage

- 7 tests covering various patterns
- Caption exclusion verified
- Allowed exceptions tested
- Inline bold preserved
