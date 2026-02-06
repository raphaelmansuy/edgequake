# OODA Iteration 03 - Orient

**Mission**: Improve PDF-to-Markdown Conversion Quality
**Date**: 2026-02-05

---

## Root Cause Analysis

### Primary Issue

Our bullet detection uses only 7 characters in a regex:

```rust
r"^[-–—*•◦▪]\s+"
```

PyMuPDF4LLM uses 530+ characters including the entire geometric shapes Unicode block (0x25A0-0x2600).

### Secondary Issue

Indentation calculation assumes 72pt left margin (US Letter), which may not be accurate for all PDFs. PyMuPDF4LLM uses actual clip boundary.

### Impact Assessment

- **Severity**: High - missing ~98.7% of bullet characters
- **Scope**: All list detection for PDFs using Unicode bullets
- **Risk**: Low - adding character detection won't break existing functionality

---

## Options Analysis

### Option A: Expand bullet character set (RECOMMENDED)

- **Effort**: 20 min
- **Risk**: Very low
- **Impact**: Detect 530+ bullet characters like PyMuPDF4LLM

### Option B: Also normalize all bullets to "- " dash

- **Effort**: 10 min (after Option A)
- **Risk**: Low - matches PyMuPDF4LLM behavior
- **Impact**: Consistent Markdown output

### Option C: Use actual page left margin for indentation

- **Effort**: 30 min
- **Risk**: Medium - requires structural changes
- **Impact**: More accurate nested list detection

---

## Decision Rationale

**Choose Options A + B**: Expand bullet detection AND normalize output.

1. **First Principles**: Match PyMuPDF4LLM's comprehensive character set
2. **Quick win**: Just need to expand the regex character class
3. **Consistency**: Normalizing to "- " matches gold standard output
4. **Skip Option C for now**: Current indentation is "good enough" for iteration

---

## Expected Outcome

After fix:

- Detect 530+ bullet characters (including geometric shapes)
- All bullet types output as "- " (markdown dash)
- Lists score improvement: 55 → 70+ (estimated)
