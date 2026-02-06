# OODA-05: Orient - Strategy Analysis

## Problem Framing

The main gap is in **Structure** (0.350) and **Format** (0.343) scores. These require:

1. Correct header level detection (H1-H6)
2. Correct bold/italic text marking

## Strategic Options

### Option A: Improve Font Name Pattern Matching (Quick Win)

**Approach**: Add more font name patterns to `is_bold()`:

- "medi" (Medium - often used for emphasis)
- "semi" (SemiBold)
- "demi" (DemiBold)

**Pros**:

- Quick implementation (1-2 lines of code)
- No API changes needed

**Cons**:

- Still heuristic-based, may not catch all cases
- Medium fonts aren't always intended as "bold"

**Expected Impact**: +0.02-0.05 on Format score

### Option B: Implement Size-Ranked Header Detection (Like pymupdf4llm)

**Approach**: Instead of ratio-based levels, rank font sizes:

1. Collect all unique font sizes in document
2. Identify body size (most frequent)
3. Sort larger sizes descending
4. Assign H1 to largest, H2 to second-largest, etc.

**Pros**:

- Matches pymupdf4llm behavior exactly
- More robust across different document styles

**Cons**:

- Requires refactoring header detection
- Need document-level font size collection first

**Expected Impact**: +0.10-0.15 on Structure score

### Option C: Extract Font Weight from PDFium

**Approach**: Use `FPDFText_GetFontWeight()` to get numeric weight:

- Weight >= 500 → Bold (Semibold starts at 600)
- Weight >= 700 → Bold (traditional bold)

**Pros**:

- Most accurate method
- Matches how pymupdf4llm works (via flags)

**Cons**:

- Requires adding fields to RawChar, Span structs
- Significant refactoring of extraction pipeline
- May need pdfium feature flags for the API

**Expected Impact**: +0.15-0.20 on Format score

## Decision Matrix

| Option                 | Effort | Impact  | Risk   |
| ---------------------- | ------ | ------- | ------ |
| A: Font patterns       | Low    | Low-Med | Low    |
| B: Size-ranked headers | Medium | Medium  | Low    |
| C: Font weight API     | High   | High    | Medium |

## Recommended Approach

**For OODA-05**: Implement Option A + partial Option B

1. Add "medi", "semi", "demi" to bold patterns (quick win)
2. Improve header level thresholds based on analysis

**For OODA-06+**: Implement Option B fully (size-ranked headers)

**Future**: Consider Option C if Format score remains low

## Risk Assessment

- Adding "medi" might cause false positives (some documents use Medium for body text)
- Mitigation: Only treat as bold if font size is also larger than body

---

**Timestamp**: 2025-01-27
