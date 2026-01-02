# ORIENT - Loop 012

## Timestamp

Fri Jan 2, 2026 19:25:00 HKT

## Directory Scope

**Target: crates/edgequake-pdf/src/renderers/markdown.rs**

After analyzing drift patterns and comparing gold vs generated outputs, the primary issue is in the **MarkdownRenderer** layer.

## Root Cause Analysis

### 1. Heading Detection Failure (PRIMARY ISSUE)

**Evidence:**

```markdown
# Gold:

# LLMS4OL 2025: The 2nd Large Language Models...

# Generated:

LLMs4OL 2025: The 2nd Large Language Models... (NO HEADING MARKUP)
```

**Diagnosis:**

- The heading-level detection in `backend/sota_backend.rs` (lines 2300-2400) is not working correctly
- TextBlock.heading_level is not being set properly
- MarkdownRenderer is receiving blocks with heading_level=0 when they should be heading_level=1

**Impact:** This causes ~82 heading:mismatch drifts and contributes to content mismatches when the validator expects headings but gets plain text.

### 2. Missing Horizontal Rules (SECONDARY ISSUE)

**Evidence:**

```markdown
# Gold:

---

### Abstract

## ...

# Generated:

**Abstract.**
...
(no horizontal rules)
```

**Diagnosis:**

- The gold files use `---` as semantic section separators
- These are likely not in the original PDF
- They may be an artifact of the gold annotation process OR
- They indicate page breaks or section boundaries that should be detected

**Impact:** Minor - contributes to whitespace/structure mismatches but doesn't affect core content.

### 3. Header/Footer Artifacts (TERTIARY ISSUE)

**Evidence:**

```markdown
# Generated includes:

LLMs4OL 2025 Task Participant Long Papers https://doi.org/10.52825/ocp.v6i.2900
© Authors. This work is licensed under a Creative Commons Attribution 4.0 International License
Published: 01 Oct. 2025
```

These appear at the top of the generated document but are filtered out in the gold reference.

**Diagnosis:**

- `backend/sota_backend.rs` has header/footer detection logic
- It's not aggressive enough or not correctly identifying these elements
- The patterns look like journal article metadata that should be filtered

**Impact:** Moderate - adds ~20-30 content:mismatch drifts per document.

### 4. Style Markup Issues (ONGOING)

**Evidence:**

```markdown
# Gold:

`silp_nlp` team... (inline code markup)
`pranav-s/MaterialsBERT` (inline code markup)

# Generated:

silpnlp team... (no markup)
pranav-s/MaterialsBERT (no markup)
```

**Diagnosis:**

- Font-based style detection is working (31.5% style accuracy vs 16.9% baseline)
- But specific monospace/code fonts are not being detected
- The renderer may need to map certain font families to inline code markup

**Impact:** Contributes to 470 style:mismatch drifts.

## Architecture Analysis

### Text Extraction Flow

```
PDF (lopdf)
  → SotaBackend.extract_text_blocks() [backend/sota_backend.rs]
    → TextProcessor [processors/processor.rs]
      → WhitespaceNormalizationProcessor
        → StyleDetectionProcessor
          → TextTableReconstructionProcessor
            → MarkdownRenderer [renderers/markdown.rs]
              → Output MD file
```

### Critical Components

1. **SotaBackend.detect_heading()** (lines 2300-2400)

   - Currently uses font size comparison
   - Sets TextBlock.heading_level
   - **HYPOTHESIS**: Threshold is too conservative or font size data is incorrect

2. **MarkdownRenderer.render()** (lines 100-300)

   - Converts TextBlock → Markdown
   - Should render heading_level > 0 as `#` markers
   - **HYPOTHESIS**: Not checking heading_level or logic is bypassed

3. **Header/Footer Detection** (lines 1500-1600 in sota_backend.rs)
   - Uses position-based heuristics
   - **HYPOTHESIS**: Needs tighter thresholds or pattern-based filtering

## Target Directory Decision

**Primary target: `crates/edgequake-pdf/src/renderers/markdown.rs`**

**Rationale:**

1. Heading markup is a rendering issue (TextBlock → MD conversion)
2. Quick win: Fix renderer to properly emit `#` markers for heading_level > 0
3. High impact: Will fix ~82 heading mismatches immediately
4. Small scope: Single file, focused change

**Secondary target (next iteration): `crates/edgequake-pdf/src/backend/sota_backend.rs`**

- Improve heading detection thresholds
- Enhance header/footer filtering
- Fix font-to-style mapping

## Research: PDF Heading Detection

### First Principles

1. **Font Size**: Headings typically use larger font than body text
2. **Font Weight**: Headings often use bold fonts
3. **Vertical Spacing**: Headings have more space before/after
4. **Position**: Main title appears near top of first page
5. **Capitalization**: Headings may use title case or ALL CAPS

### Common PDF Patterns

- Title: Font size 18-24pt (body: 10-12pt)
- H1: 16-18pt
- H2: 14-16pt
- H3: 12-14pt

### Current Implementation Issue

Looking at the code, the `detect_heading()` method likely:

- Compares font sizes to body_size
- Sets heading level based on relative size difference
- **BUG HYPOTHESIS**: body_size calculation is incorrect or comparison threshold is too high

## Next Steps (DECIDE Phase)

1. Examine MarkdownRenderer.render() to verify heading_level rendering logic
2. Trace TextBlock objects to confirm heading_level values
3. Add debug logging to see what heading_level values are being passed
4. Propose minimal patch to fix heading rendering
