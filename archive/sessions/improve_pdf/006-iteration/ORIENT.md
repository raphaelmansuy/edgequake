# ORIENT.md - Iteration 006

**Directory:** `edgequake/crates/edgequake-pdf/src/processors`

**Timestamp:** 2026-01-02

## Root Cause Analysis

### Current Architecture

```
Block → HeaderDetectionProcessor → {
    1. Calculate body font size (statistical, GOOD ✅)
    2. Check font size ratio (font-based, GOOD ✅)
    3. Check for numbered patterns (structural, GOOD ✅)
    4. BUT: Require keyword match for single numbers (heuristic, BAD ❌)
}
```

### Why Keywords Were Added

Likely reason: To avoid false positives from numbered lists:

- "1. Explore the dashboard" (list item, not section)
- "1. Introduction" (section header)

**The keyword check was a shortcut** to distinguish these cases.

### First Principles Solution

**Distinguishing Features:**

| Property        | List Item             | Section Header                |
| --------------- | --------------------- | ----------------------------- |
| Font Size       | Body size             | Larger than body              |
| Font Weight     | Normal (400)          | Often bold (600+)             |
| Position        | Inline with paragraph | Isolated, has margin          |
| Length          | Can be long sentence  | Usually short (<100 chars)    |
| Capitalization  | Sentence case         | Title case (cap after number) |
| Subsequent Text | Continuous            | New paragraph below           |

**Detection Strategy:**

```rust
fn is_section_header(block: &Block, body_size: f32) -> Option<u8> {
    // 1. Check numbering pattern
    let numbering_level = detect_numbering_pattern(&block.text)?;

    // 2. Font size confirmation
    let font_size = block.spans.first()?.style.size?;
    let size_ratio = font_size / body_size;

    // 3. Structural signals (multiple indicators strengthen confidence)
    let is_short = block.text.len() < 100;
    let is_capitalized = is_title_case_after_number(&block.text);
    let is_bold = block.spans.first()?.style.weight? >= 600;
    let is_isolated = check_spatial_isolation(&block);

    // Decision logic (no keywords needed!)
    if numbering_level == 1 {
        // "1. Something"
        if size_ratio > 1.3 || (is_bold && is_short && is_capitalized) {
            return Some(2); // H2
        }
    } else if numbering_level > 1 {
        // "1.1 Something" or "1.1.1 Something"
        if size_ratio > 1.15 || is_bold {
            return Some((numbering_level + 1).min(6)); // H3+
        }
    }

    None
}
```

### Pattern Detection (No Keywords)

**Reliable Patterns:**

- `^\d+\.` → Level 1 (e.g., "1.", "2.")
- `^\d+\.\d+` → Level 2 (e.g., "1.1", "2.3")
- `^\d+\.\d+\.\d+` → Level 3 (e.g., "1.1.1")

**Capitalization Check:**

```rust
fn is_title_case_after_number(text: &str) -> bool {
    // After "1. " or "1.1 ", first letter should be uppercase
    let after_number = text.split_whitespace().skip(1).next()?;
    after_number.chars().next()?.is_uppercase()
}
```

**Spatial Isolation Check:**

```rust
fn check_spatial_isolation(block: &Block) -> bool {
    // Headers typically have vertical space before/after
    // This would need context from document, but can be checked
    // by BlockMergeProcessor later
    true // Placeholder
}
```

### Comparison: Keyword vs. First Principles

**Example: "1. Executive Summary"**

**Keyword Approach:**

```rust
if pattern.is_match("1.") && SECTION_KEYWORDS.contains("executive") {
    // ❌ Fails! "executive" not in keyword list
    return BlockType::Text;
}
```

**First Principles Approach:**

```rust
if pattern.is_match("1.") && font_size > body_size * 1.3 && is_short {
    // ✅ Succeeds! Based on font and structure
    return BlockType::SectionHeader;
}
```

### Implementation Plan

1. **Remove SECTION_KEYWORDS** constant and `starts_with_section_keyword()`
2. **Strengthen pattern confidence** with multi-signal detection:
   - Font size ratio (existing)
   - Bold weight (existing)
   - Title case (new)
   - Length constraint (existing)
3. **Trust numbered patterns** for subsections (1.1, 1.1.1)
4. **Keep font-based detection** for unnumbered headers

### Risk Mitigation

**Risk:** More false positives from numbered lists

**Mitigation:**

1. Font size threshold (lists use body size)
2. Length constraint (<100 chars)
3. Title case check (lists often sentence case)
4. Future: Use BlockMergeProcessor context (lists merge with paragraphs)

### Research: Industry Standards

PDF structure standards (PDF/UA, PDF/A) define headers by:

- Tagged structure tree (rarely used)
- Font size differences (common)
- Consistent styling (common)
- NOT by content keywords!

Academic paper structure extraction research:

- SciSpaCy, GROBID, etc. use font features + CRF/ML models
- Keywords used as features, NOT hard rules
- Our approach (font + numbering) is more principled
