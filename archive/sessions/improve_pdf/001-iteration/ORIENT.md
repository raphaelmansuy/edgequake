# ORIENT.md - Iteration 001

**Directory:** `crates/edgequake-pdf/src/backend`

## Root Cause Analysis

### Issue 1: Inverted Reading Order

**Location:** `sota_backend.rs` lines 2374-2379

```rust
// Sort blocks by Y (top to bottom) - PDF Y is up, so sort descending by max Y (top)
blocks.sort_by(|a, b| b.bbox.y2.partial_cmp(&a.bbox.y2).unwrap_or(std::cmp::Ordering::Equal));
```

**Problem:** This sort is applied AFTER the multi-column layout processing at line 2367-2370:

```rust
// Group into lines (handles two-column layouts) and get column bounding boxes
let (lines, columns) = self.group_into_lines(non_table_elements, page_width, page_height);
```

The `group_two_column_layout()` function (lines 1820-1970) correctly:

1. Separates spanning elements (titles)
2. Processes left column fully
3. Processes right column fully
4. Combines them in correct reading order

But then the final sort at line 2379 re-sorts ALL blocks by Y position, destroying the column-based reading order and mixing content from both columns.

**Evidence:** Looking at the generated output:

- Paragraphs from different columns are interleaved
- Content appears in reverse Y-order within each column
- The final output reads bottom-to-top

### Issue 2: Ligature Glyph Handling

**Location:** `sota_backend.rs` lines 30-250 (encoding tables)

The WIN_ANSI_ENCODING table doesn't include standard typographic ligatures:

- U+FB01 (fi ligature)
- U+FB02 (fl ligature)
- U+FB00 (ff ligature)
- U+FB03 (ffi ligature)
- U+FB04 (ffl ligature)

When a font uses these glyphs and there's no ToUnicode CMap, the extractor fails to decode them properly.

### Issue 3: Space Detection Threshold

**Location:** `sota_backend.rs` lines 2088-2107 (`merge_line()`)

```rust
let avg_char_width = avg_font_size * 0.5;
let space_threshold = avg_char_width * 1.1;
```

The threshold of `1.1 * avg_char_width` may be:

- Too large for tightly-kerned fonts (missing spaces)
- Too small for wide fonts (extra spaces)

Academic PDFs often use fonts with tight kerning for inline URLs and code.

## Priority Ranking

1. **Reading Order (P0)** - Affects 468/628 drifts (74.5%). Single-line fix.
2. **Ligature Handling (P1)** - Affects words throughout documents. Medium complexity.
3. **Space Detection (P2)** - Affects word concatenation. Requires tuning.

## Research Notes

PDF Y-coordinate system:

- Origin is at bottom-left (0,0)
- Y increases upward
- Page height is typically 792 points (11 inches)
- Top of page has highest Y value

The current sort assumes we want higher Y first (correct for reading top-to-bottom), but it ignores the column structure that was already established.
