# OODA-05: Orient

## Root Cause Analysis

### Hypothesis 1: Y Coordinate Filtering

The extraction engine filters elements by Y bounds:

```rust
// extraction_engine.rs L343-351
.filter(|e| {
    e.x >= -x_margin
        && e.x <= page_width + x_margin
        && e.y >= y_lower_bound
        && e.y <= y_upper_bound
})
```

If the title is at Y=0 or negative (due to CTM transforms), it could be filtered out.

**Likelihood: MEDIUM**

### Hypothesis 2: Font Detection Failure

Only 1 font detected on page 1. This could mean:

- Title uses embedded subset font not being parsed
- Title uses Type3 font with missing ToUnicode CMap
- Content parser is missing font switches

**Likelihood: HIGH** - The title is likely in a different font than body text.

### Hypothesis 3: Letter-Spacing Not Being Collapsed

We implemented `fix_spaced_text()` in OODA-05, but it only works if:

1. The text is extracted first
2. The pattern matches (4+ uppercase letters with spaces)

If the text isn't extracted, the fix can't help.

**Likelihood: LOW** - Spaced text fix is correct, but text isn't reaching it.

### First Principles Analysis

1. **PDF Structure**: Title is definitely in the PDF (pdftotext extracts it)
2. **lopdf**: Our backend uses lopdf for parsing - need to verify it handles the specific encoding
3. **Content Stream**: The title must be in the content stream - we need to trace where it's lost

### Priority Decision Matrix

| Hypothesis     | Likelihood | Impact | Investigation Effort | Priority |
| -------------- | ---------- | ------ | -------------------- | -------- |
| Font Detection | HIGH       | HIGH   | MEDIUM               | **1**    |
| Y Filtering    | MEDIUM     | HIGH   | LOW                  | **2**    |
| Spaced Text    | LOW        | MEDIUM | DONE                 | 3        |

## Recommended Approach

1. Add debug logging to content parser to dump ALL text operands on page 1
2. Check if title text appears in raw extraction before filtering
3. If missing, investigate font handling for page 1
4. If present but filtered, fix Y coordinate bounds
