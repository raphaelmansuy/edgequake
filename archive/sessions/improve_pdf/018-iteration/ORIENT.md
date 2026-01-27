# OODA Loop 018 - ORIENT

**Timestamp:** 2026-01-03 15:20:00

**Directory:** crates/edgequake-pdf/src/renderers

## Root Cause Analysis

After examining the code, I've identified the following about style accuracy (35.6%):

### Current Style Implementation

**Bold Detection (renderers/markdown.rs:248):**

```rust
let is_bold = span.style.weight.map(|w| w >= 600).unwrap_or(false) && !skip_bold;
```

- ✓ Uses font weight threshold of 600
- ✓ Properly checks for `Some(weight)` values
- ⚠️ Falls back to `false` if weight is `None`

**Italic Detection (renderers/markdown.rs:249):**

```rust
let is_italic = span.style.italic && !skip_italic;
```

- ✓ Uses boolean flag from span.style
- ⚠️ No fallback logic

**Heading Detection (processors/processor.rs:2036-2043):**

```rust
fn calculate_level(&self, section_num: &str) -> u8 {
    let dots = section_num.matches('.').count();
    // Minimum level 2, max level 6
    (dots + 1).min(6).max(2) as u8
}
```

- ✓ Calculates level from section numbers (e.g., "3.2." -> H3)
- ✓ Clamps to valid range (2-6)
- ⚠️ Relies on section numbers being detected first

### Problem Hypothesis

Looking at the per-document scores:

| Document              | Style Acc | Issue                                    |
| --------------------- | --------- | ---------------------------------------- |
| ccn_2512.21804v1      | 7.0%      | **VERY LOW** - Almost no styles detected |
| one_tool_2512.20957v2 | 23.5%     | Low - Missing many styles                |
| 2900_Goyal_et_al      | 37.6%     | Moderate - Some styles detected          |
| AlphaEvolve           | 50.3%     | Best - But still only half correct       |
| agent_2510.09244v1    | 58.8%     | Best - More than half correct            |

### First Principles Analysis

**Truth:** Style accuracy varies wildly between documents (7% to 58.8%)

This suggests the problem is NOT in the rendering logic (which is consistent), but in:

1. **Style Detection Upstream** - Font weight/italic flags may not be populated correctly
2. **Heading Detection Upstream** - Section numbers may not be detected
3. **Document-Specific Issues** - Some PDFs may have fonts that don't report weight/italic

### Investigation: Where Do Styles Come From?

Styles flow through this pipeline:

```
PDF Backend (lopdf/pdfium)
  → Extract font properties
  → Store in TextSpan.style
  → SectionPatternProcessor detects headings
  → MarkdownRenderer applies styles
```

The problem is likely in the **extraction phase** or **heading detection**, not rendering.

## Candidate Fixes

### Option 1: Improve Font Property Extraction (HIGH IMPACT)

- **Target:** backend/lopdf or backend/pdfium
- **Fix:** Enhance font weight and italic detection
- **Risk:** Medium (backend code is complex)
- **Expected gain:** +10-15% style accuracy

### Option 2: Improve Heading Detection (MEDIUM IMPACT)

- **Target:** processors/processor.rs (SectionPatternProcessor)
- **Fix:** Add more heading detection heuristics
- **Risk:** Low (processor is well-structured)
- **Expected gain:** +5-10% style accuracy

### Option 3: Add Fallback Heuristics (LOW IMPACT)

- **Target:** renderers/markdown.rs
- **Fix:** Use font name patterns to infer bold/italic
- **Risk:** Low (simple addition)
- **Expected gain:** +2-5% style accuracy

## Decision

Focus on **Option 2: Improve Heading Detection** because:

1. It's lower risk (processor layer is easier to test)
2. Headings are a significant component of style accuracy
3. We can add font-size based heading detection as a fallback
4. First principles: Headings have geometric properties (larger font, isolation)

## Next: DECIDE

Propose specific changes to SectionPatternProcessor to add font-size based heading detection.
