# OODA-28 Orient: Gap Analysis and Quality Assessment

## Re-read Mission Status ✅

Re-read `specs/004-perfect-pdf-markdown-conversion.md` at iteration start.

## Key Findings from Observation

### 1. Font Encoding Gap (Apple-Sandbox-Guide)

**Issue:** Fonts F1.0 and F2.0 have 0 ToUnicode mappings

- PDF uses custom Type1 or TrueType fonts
- Character codes don't map to standard Unicode
- Result: Garbled text like `!"#$%` instead of "Table"

**Markitdown Solution (Hypothesis):**
Markitdown likely uses one of:

1. Adobe Glyph List (AGL) to map glyph names → Unicode
2. PDF.js/pdfminer approach with /Differences array parsing
3. Character shape recognition (OCR-like fallback)

**First Principles Analysis:**

```
WHY does markitdown work and we don't?

PDF Font Encoding Hierarchy:
1. ToUnicode CMap → Direct byte → Unicode mapping (✅ we handle)
2. /Encoding + /Differences → Glyph name remapping (⚠️ partial)
3. PostScript glyph names → Adobe Glyph List (❌ missing)
4. Character shape → OCR fallback (❌ not implemented)

The Apple-Sandbox PDF likely uses approach #3:
- Font embeds glyph shapes with PostScript names
- Character code 0x54 → glyph name "/T" → Unicode U+0054
- Without AGL lookup, we fall back to raw byte → wrong Unicode
```

### 2. Our Strength: Two-Column Layout

EdgeQuake consistently outperforms markitdown on arXiv papers:

- We detect column boundaries (305.0pt threshold)
- We read left column first, then right
- Markitdown produces character-by-character fragmentation

**Why we win:**

```
EdgeQuake Column Detection Pipeline:

┌─────────────────────────────────────────────┐
│ Page Text Elements                          │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│ X-coordinate Analysis                       │
│ - Min X, Max X per element                  │
│ - Cluster detection (left vs right zone)   │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│ Gap Detection                               │
│ - Find largest horizontal gap               │
│ - If gap > 50pt → two columns               │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│ Column Separation                           │
│ - Sort left column by Y (top→bottom)        │
│ - Sort right column by Y                    │
│ - Concatenate: left + right                 │
└─────────────────────────────────────────────┘
```

### 3. Test Speed Analysis

Current test timing:

```
quick_smoke.rs:     0.09s (4 tests) ✅
fast_quality.rs:    1.59s (5 tests) ✅
----------------------------------------
TOTAL:              1.68s            ✅ Under 5s target
```

**Room for improvement:**

- Can add 3-4 more PDFs without exceeding 5s
- Each small PDF (~100KB) takes ~300-500ms
- Should test diverse PDF types

### 4. Quality Metrics Baseline

From fast_quality tests:
| PDF | TPS | Jaccard | SFS | Time |
|-----|-----|---------|-----|------|
| AI_Services_Elitizon | 98.9% | 0.980 | 87.5% | 1609ms |
| 004_simple_table | - | - | - | 148ms |
| 003_two_columns | - | - | - | 234ms |

**Gap to target:**

- TPS: 98.9% vs 98% target → ✅ MET
- SFS: 87.5% vs 95% target → ⚠️ 7.5% gap

## Risk Assessment

| Approach                    | Benefit                    | Risk                          | Effort |
| --------------------------- | -------------------------- | ----------------------------- | ------ |
| Fix font encoding           | Major quality gain         | Complex, may break other PDFs | HIGH   |
| Add AGL lookup              | Better glyph name handling | 14k entries to handle         | MEDIUM |
| Add more fast tests         | Better coverage            | May slow tests                | LOW    |
| Create markitdown baselines | Better comparison          | Need to save gold files       | LOW    |

## Recommended Priority

1. **LOW RISK:** Add fast quality tests for new PDFs (immediate value)
2. **MEDIUM RISK:** Create markitdown gold standards for comparison
3. **DEFER:** Font encoding fixes (needs dedicated OODA iteration)

## Key Gaps to Address This Iteration

1. ❌ No tests for arXiv two-column papers (our strength)
2. ❌ No tests for font encoding edge cases
3. ❌ No automated markitdown comparison
4. ⚠️ SFS below 95% target

## Proposed Test Additions

| Test                           | PDF                           | Size  | Purpose                     | Expected Time |
| ------------------------------ | ----------------------------- | ----- | --------------------------- | ------------- |
| `test_arxiv_column_reading`    | stackplanner_2601.05890v1.pdf | 1.1MB | Two-column academic         | 500-800ms     |
| `test_small_business_doc`      | Scottish SMEs\*.pdf           | 283KB | Clean business PDF          | 200-300ms     |
| `test_font_encoding_challenge` | Apple-Sandbox-Guide           | 354KB | Font encoding (expect fail) | 400-600ms     |

**Estimated total:** ~3.5-4.5 seconds (still under 5s target)
