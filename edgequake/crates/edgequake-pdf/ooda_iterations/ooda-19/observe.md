# OODA-19: Rotated Text Detection

## Observe Phase

### Problem Statement

The `agent_2510.09244v1` document has the lowest quality score (80.1%). Investigation revealed that the arXiv identifier `arXiv:2510.09244v1 [cs.AI] 10 Oct 2025` appears inline within body paragraphs, incorrectly merged with "Today, one can develop remarkable systems..."

### Evidence Gathered

1. **Output Analysis** (line 41):

   ```
   arXiv:2510.09244v1 [cs.AI] 10 Oct 2025 Today, one can develop remarkable systems...
   ```

2. **Gold File Expectation** (line 3):

   ```markdown
   **arXiv:2510.09244v1 [cs.AI] 10 Oct 2025**
   ```

   The gold file expects the arXiv identifier as a separate line at the TOP of the document.

3. **Root Cause Discovery**:
   - arXiv watermarks are positioned in the left margin, **rotated 90 degrees**
   - The CTM (Current Transformation Matrix) encodes this rotation
   - Normal text: `[1, 0, 0, 1, tx, ty]`
   - 90° rotation: `[0, ±1, ∓1, 0, tx, ty]`

4. **Coordinates**:
   - Rotated text: Y=440.7, X=32.0 (left margin, mid-page vertically)
   - This Y coordinate matches body text on line 41, causing incorrect merging

### Key Insight

PDF text elements carry rotation information in the CTM matrix. By detecting `|ctm[0]| < 0.1 && |ctm[3]| < 0.1`, we can identify 90-degree rotated text.
