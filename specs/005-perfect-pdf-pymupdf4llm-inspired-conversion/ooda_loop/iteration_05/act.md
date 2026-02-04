# OODA-05: Font Style Detection - CMSY Pattern

## Observe

### Quality Baseline (Post OODA-04)

| Metric      | Score | Target |
| ----------- | ----- | ------ |
| **Quality** | 0.752 | ≥0.95  |
| ROUGE-L     | 0.711 | -      |
| Word F1     | 0.915 | -      |
| Structure   | 0.650 | -      |
| Format      | 0.572 | -      |

### Problem Identified

The v2_2512.25072v1.pdf file has a low Format score (0.462) with italic detection at only 0.276 (27.6% match).

**Comparison with Gold Standard:**

- Gold standard: 229 italic markers (`_text_`)
- Our output: 68 italic markers (`*text*`)
- Match rate: ~30%

### Font Analysis via PyMuPDF

PyMuPDF detects these fonts as italic (via font flags bitmask):

```
  65 CMMI7
  64 NimbusRomNo9L-ReguItal
  64 CMMI10
  43 CMSY10     <-- Computer Modern Symbol - NOT in our patterns!
  27 CMMI8
  16 CMSY8
   8 CMMI6
   6 CMSY7
   3 CMSY6
   3 CMMI5
   2 NimbusRomNo9L-MediItal
   2 CMMIB8
```

**Key Finding:** `CMSY*` fonts (Computer Modern Symbol) are used for mathematical symbols like `∗`, `†`, `·` and were NOT being detected as italic.

## Orient

### Root Cause Analysis

1. PyMuPDF uses **font flags** (bitmask) for italic detection - more reliable
2. Our code uses **font name patterns** - works for most cases
3. `CMSY` (Computer Modern Symbol) fonts have italic styling but weren't in our pattern list
4. The pattern list was missing: `"cmsy"`

### Expected Impact

- CMSY fonts account for ~68 italic spans (43+16+6+3 from counts above)
- Adding the pattern should improve italic detection
- However, the main difference may be in span organization, not detection

## Decide

### Implementation Plan

1. Add `cmsy` pattern to `font_handling.rs` italic detection
2. Verify all LaTeX font patterns are covered:
   - `cmti` - Computer Modern Text Italic ✓
   - `cmmi` - Computer Modern Math Italic ✓
   - `cmsy` - Computer Modern Symbol ← ADD
   - `cmmib` - Computer Modern Math Italic Bold ✓

### Changes

**File:** `edgequake/crates/edgequake-pdf/src/backend/font_handling.rs`

Add `|| lower_name.contains("cmsy")` to the italic detection pattern.

## Act

### Code Change Applied

```rust
// OODA-05: Detect italic from font naming conventions.
let is_italic = lower_name.contains("italic")
    || lower_name.contains("oblique")
    || lower_name.contains("ital")
    || lower_name.contains("sfti")
    || lower_name.contains("cmti")
    || lower_name.contains("cmmi")
    || lower_name.contains("cmsy")   // OODA-05: Computer Modern Symbol
    || lower_name.contains("cmmib")
    || lower_name.contains("-italic");
```

### Verification

**Font Detection Test (all fonts now detected correctly):**

```
CMSY10: bold=False, italic=True ✓
CMSY8: bold=False, italic=True ✓
CMSY7: bold=False, italic=True ✓
CMSY6: bold=False, italic=True ✓
CMMI10: bold=False, italic=True ✓
NimbusRomNo9L-ReguItal: bold=False, italic=True ✓
NimbusRomNo9L-MediItal: bold=True, italic=True ✓
CMBX10: bold=True, italic=False ✓
```

### Results

**Post OODA-05 Italic Counts:**

- Gold standard: 229 italic markers
- Previous (OODA-04): 68 italic markers
- Current (OODA-05): 78 italic markers (+10)

**Improvement:** +10 italic markers detected (14.7% improvement in detection)

### Quality Metrics (Post OODA-05)

| Metric      | Pre   | Post  | Delta  |
| ----------- | ----- | ----- | ------ |
| **Quality** | 0.752 | 0.752 | +0.000 |
| ROUGE-L     | 0.711 | 0.711 | +0.000 |
| Word F1     | 0.915 | 0.915 | +0.000 |
| Structure   | 0.650 | 0.650 | +0.000 |
| Format      | 0.572 | 0.572 | +0.000 |

### Analysis

The quality score didn't change significantly because:

1. Italic detection improved (68→78) but is still only 34% of gold (229)
2. The main gap is NOT font detection (all fonts are detected correctly)
3. The remaining gap is likely due to:
   - **Span granularity**: PyMuPDF creates more fine-grained spans per-character
   - **Span merging**: Our consolidation may be merging italic/non-italic spans
   - **Text extraction**: Some italic text may be lost during extraction

### Next Steps (OODA-06)

1. Investigate span merging logic in `markdown.rs` `consolidate_spans()`
2. Check if `TextElement` to `TextSpan` conversion preserves all italics
3. Consider per-character style tracking for math-heavy documents
