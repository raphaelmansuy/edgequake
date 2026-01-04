# ORIENT Phase: Root Cause Analysis

## Analysis Methodology

1. Deep code inspection of processor chain
2. Trace data flow through pipeline
3. First principles analysis of each issue

---

## ISSUE-001: Heading Level Miscalculation

### Root Cause

**Two competing heading detection systems with different thresholds:**

1. `StyleDetectionProcessor.detect_headers()`:

   - ratio > 1.5 → H1
   - ratio > 1.2 → H2
   - ratio > 1.1 → H3

2. `HeadingClassifier.calculate_level()` (called by `SectionPatternProcessor`):
   - ratio >= 1.8 → H2
   - ratio >= 1.5 → H3
   - ratio >= 1.3 → H4
   - else → H5

**Problem:** `SectionPatternProcessor` runs AFTER `StyleDetectionProcessor` and can override correct H1/H2/H3 classifications because it doesn't check if block is already marked as `SectionHeader`.

### Code Path

```
Extractor::apply_processors()
  → StyleDetectionProcessor (sets H1/H2/H3)
  → HeaderDetectionProcessor (...)
  → SectionPatternProcessor (overwrites with H2-H5!)
```

In `SectionPatternProcessor::process()` line 221-224:

```rust
if block.block_type != BlockType::Text && block.block_type != BlockType::Paragraph {
    continue;
}
```

This check skips blocks already marked as SectionHeader, BUT the blocks come in as Text and get marked by StyleDetectionProcessor, then SectionPatternProcessor overwrites them.

**Wait** - actually the check SHOULD skip already-classified headers. Let me re-analyze...

The issue is `StyleDetectionProcessor` marks blocks as `BlockType::SectionHeader`, so `SectionPatternProcessor` should skip them. But the output shows H4/H5 levels which come from `HeadingClassifier`.

**New hypothesis:** `StyleDetectionProcessor.detect_headers()` isn't detecting headers due to the `looks_like_section` guard.

```rust
let looks_like_section = text.starts_with(|c: char| c.is_ascii_digit())
    || text.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c.is_ascii_digit());
```

For "Simple Text Document", "Second Level Heading", "Third Level Heading":

- None start with a digit
- None are all uppercase

So `looks_like_section = false`, and the H2/H3 classification paths don't trigger.

Only the H1 path works: `if ratio > 1.5 && is_short` - but the font size ratio must be < 1.5.

**Final Root Cause:** The `looks_like_section` heuristic is too restrictive. Normal title case headings don't match.

---

## ISSUE-003: Spurious Page Number "1"

### Root Cause

The `MarginFilterProcessor.is_margin_content()` checks:

```rust
let in_footer = bbox.y1 <= bottom_margin;
if in_footer && trimmed.parse::<i32>().is_ok() {
    return true;
}
```

Where `bottom_margin = page_height * 0.05`.

For a standard letter page (792pt height), bottom_margin = 39.6pt.

**Hypothesis:** Pandoc places page numbers in a position where `bbox.y1 > 39.6pt`.

The page number might be at ~50pt from bottom, outside the 5% margin threshold.

**Fix:** Increase bottom margin threshold OR add explicit page number pattern detection anywhere in footer region.

---

## ISSUE-002: Tables Not Detected

### Console Output Analysis

```
Table grid: 5 rows (from lines), 0 cols (from lines), 0 cols (from clustering)
```

LatticeEngine detected 5 horizontal lines but 0 vertical lines.

**Root Cause:** Pandoc-generated tables use horizontal rules but may not have visible vertical lines. The lattice algorithm requires both horizontal AND vertical lines to form a grid.

**First Principles:**

- PDF tables can be rendered multiple ways:

  1. With explicit lines (vector graphics)
  2. With spacing/alignment only (text-based)
  3. Hybrid approaches

- Pandoc uses horizontal rules (booktabs style) which have:
  - \toprule, \midrule, \bottomrule (horizontal)
  - NO vertical lines

---

## ISSUE-004: Paragraph Boundary Loss

### Root Cause

`BlockMergeProcessor` merges blocks that are "close enough" based on Y-gap threshold.

Pandoc PDFs may have tighter line spacing between paragraphs than expected, causing them to merge.

**First Principles:**

- Paragraph breaks are signaled by:
  1. Larger vertical gap (1.5-2x line height)
  2. First line indentation
  3. Short last line of previous paragraph

The merge processor only uses (1) but threshold may be wrong.

---

## ISSUE-005: List Formatting (Bullet • vs -)

### Root Cause

PDF uses Unicode bullet character (U+2022) which gets preserved literally.

Markdown expects `-` or `*` for unordered lists.

The `ListDetectionProcessor` should convert `•` to `-` in markdown output.

---

## ISSUE-009: Numbered Lists as H2 Headings

### Root Cause

`StyleDetectionProcessor.detect_headers()` line 381-382:

```rust
let looks_like_section = text.starts_with(|c: char| c.is_ascii_digit())
```

Text "1. First numbered item" starts with a digit, so `looks_like_section = true`.

Combined with bold font (pandoc uses bold for list numbers), this triggers:

```rust
if is_bold && is_short && is_first_char_upper && looks_like_section {
    block.block_type = BlockType::SectionHeader;
    block.level = Some(2);
}
```

**First Principles Fix:** List detection should run BEFORE header detection, or header detection should exclude list patterns.

---

## Priority Fix Order

Based on impact and code complexity:

1. **ISSUE-001** - Fix `looks_like_section` heuristic (simple change)
2. **ISSUE-003** - Improve page number filtering (increase margin or add pattern)
3. **ISSUE-009** - Add list pattern exclusion to header detection
4. **ISSUE-005** - Add bullet normalization in markdown renderer
5. **ISSUE-002** - Add text-based table detection for borderless tables
6. **ISSUE-004** - Tune paragraph merge threshold
