# OODA Iteration 01: Orient

**Date**: 2026-02-06
**Previous Phase**: observe.md completed
**Mission Re-read**: ✅ Confirmed

---

## Objective

Analyze observations and define solution approaches using **First Principles thinking** to bridge the gap between current edgequake-pdf implementation and pymupdf4llm gold standard.

---

## First Principles Analysis

### Principle 1: PDFs Store Positioned Characters, Not Structured Content

**Fundamental Truth**: A PDF is a collection of (character, x, y, font, size) tuples, not a document tree.

**Implications**:

```
Ground Truth:         What We Want:
┌─────────────┐      ┌─────────────┐
│ 'H' @(10,20)│      │ # Header    │
│ 'i' @(16,20)│ ───▶ │             │
│ '!' @(22,20)│      │ Paragraph   │
│ 'T' @(10,40)│      │ - List      │
│ 'e' @(16,40)│      │ - Item      │
└─────────────┘      └─────────────┘
```

**Design Consequence**: Structure detection must work bottom-up:

```
Characters → Spans → Lines → Blocks → BlockTypes → Markdown
```

**Current State**: ✅ edgequake-pdf follows this pipeline correctly.

**Gap**: ❌ Block type classification is too coarse (5 types vs 10+).

---

### Principle 2: Reading Order Is Spatial, Not Sequential

**Fundamental Truth**: File order ≠ Reading order. Humans read spatially: left-to-right, top-to-bottom, respecting columns.

**Multi-Column Challenge**:

```
Visual Reading Order:        File Order (wrong):
┌──────────┬──────────┐     ┌────────────────┐
│ Para 1   │ Para 3   │     │ Para 1         │
│          │          │     │ Para 2         │
│ Para 2   │ Para 4   │     │ Para 3         │
└──────────┴──────────┘     │ Para 4         │
                             └────────────────┘
Correct: 1→2→3→4            Wrong: 1→2→3→4
```

**Design Consequence**: Must use spatial algorithms:

1. **Column Detection**: Find vertical boundaries
2. **Within-Column Sorting**: Y-coordinate ascending
3. **Cross-Column Ordering**: Left-to-right, then top-to-bottom

**Current State**: ✅ Has `column_detector.rs` and `xy_cut.rs`.

**Gap**: ⚠️ Not proven to match pymupdf4llm's reading order accuracy.

**Action**: Need comparative testing before declaring parity.

---

### Principle 3: Font Properties Signal Semantic Meaning

**Fundamental Truth**: Fonts encode structure hints:

- **Size**: Larger = Header
- **Weight**: Bold = Emphasis or Header
- **Pitch**: Monospace = Code
- **Style**: Italic = Citation or Emphasis
- **Position**: Superscript = Footnote or Math

**Font Flag Decoding** (from pymupdf4llm):

```rust
// From document_layout.py:367-373
superscript = flags & 1       // 0x01
italic      = flags & 2       // 0x02
mono        = flags & 8       // 0x08
bold        = flags & 16      // 0x10

// Char-level flags (from span['char_flags']):
strikeout   = char_flags & 1  // 0x01
bold_alt    = char_flags & 8  // 0x08 (alternative bold signal)
```

**Design Consequence**: Style detection must check multiple signals:

1. Font descriptor flags (primary)
2. Font name contains "Bold"/"Italic" (fallback)
3. Font size ratio > 1.2x body (for headers)

**Current State**: ✅ Has font flag detection in PDFium backend.

**Gap**: ❌ Missing strikeout and superscript handling in renderer.

---

### Principle 4: Structure Is Inferred, Not Guaranteed

**Fundamental Truth**: PDFs lack semantic tags. We must infer structure from:

- **Spatial proximity**: Nearby text = same block
- **Font consistency**: Same font/size = same paragraph
- **Alignment patterns**: Same x0 = list item continuation
- **Whitespace gaps**: Large vertical gap = block boundary

**Failure Modes**:

```
Ambiguous Case 1: Is this a list or paragraphs?
x0=20: • Item 1
x0=20: • Item 2

Ambiguous Case 2: Is this a header or bold text?
x0=10, size=18, bold: "Introduction"

Ambiguous Case 3: Is this a table or aligned text?
Col1    Col2    Col3
Val1    Val2    Val3
```

**Design Consequence**: Use heuristics with confidence scores, not rules:

```rust
fn classify_block(lines: &[Line]) -> (BlockType, f32) {
    let scores = vec![
        (BlockType::Header, score_as_header(lines)),
        (BlockType::ListItem, score_as_list(lines)),
        (BlockType::Code, score_as_code(lines)),
        // ...
    ];
    scores.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
}
```

**Current State**: ⚠️ edgequake-pdf uses deterministic rules, not scored heuristics.

**Gap**: ❌ No confidence scoring mechanism. May misclassify edge cases.

---

### Principle 5: Nested Lists Require Spatial Hierarchy

**Fundamental Truth**: List hierarchy is determined by **horizontal position** (x0 coordinate), not by PDF markup.

**Detection Algorithm** (from pymupdf4llm:97-151):

```
Step 1: Identify contiguous list-item segments
  - Break on non-list-item
  - Break on column change (x0 > prev_x1 or y1 < prev_y0)

Step 2: Sort segment by x0 (left edge)

Step 3: Assign levels
  First item: level = 1
  For each subsequent item:
    If x0 > prev_x0 + 10:  // 10pt threshold
      level = prev_level + 1
    Else:
      level = prev_level

Example:
x0=20: • Top 1      → level 1
x0=32: • Nested 1   → level 2 (x0 increased by 12 > 10)
x0=32: • Nested 2   → level 2 (x0 same as prev)
x0=20: • Top 2      → level 1 (x0 decreased)
```

**Design Consequence**: Must track:

1. Segment boundaries (contiguous regions)
2. x0 coordinates for each list item
3. Level assignment based on x0 deltas

**Current State**: ❌ edgequake-pdf has no list hierarchy detection.

**Gap**: **CRITICAL** - All lists render flat. Nested lists broken.

**Solution Priority**: **P0** - Implement `create_list_item_levels` equivalent.

---

## Root Cause Analysis

### Gap 1: Limited Block Types (5 vs 10+)

**Root Cause**: Insufficient classification heuristics.

**Why It Exists**:

- Block classification in `pymupdf_grouper.rs` checks:
  1. Header (font size ratio)
  2. Code (all lines monospace)
  3. ListItem (starts with bullet/number)
  4. Table (detected separately)
  5. Else: Paragraph (default)

**Why Insufficient**:

- No **Footnote** detection (superscript check)
- No **Caption** detection (near images)
- No **PageHeader/PageFooter** detection (y-position extremes)
- No **Title** detection (page 1, very large font)
- No **SectionHeader** detection (not as large as title, but larger than body)

**First Principles Solution**:

```rust
// Add to BlockType enum:
enum BlockType {
    Title,          // First page, font_size > 1.5x body
    Header(u8),     // font_size = 1.2-1.5x body, level = log2(ratio)
    SectionHeader,  // font_size = 1.1-1.2x body
    Paragraph,      // Default text
    ListItem(u8),   // Starts with bullet/number, level from x0
    Code,           // All lines monospace
    Footnote,       // First span has superscript flag
    Caption,        // Near image block
    PageHeader,     // y0 < 50 (top margin)
    PageFooter,     // y1 > page_height - 50 (bottom margin)
    Table,          // Detected by table finder
}
```

**Risk**: Higher false-positive rate with more types.

**Mitigation**: Use confidence scoring and allow fallback to simpler types.

---

### Gap 2: No List Hierarchy (P0)

**Root Cause**: Missing spatial hierarchy detection algorithm.

**Why It Exists**: Original implementation focused on basic structure, not nested lists.

**Impact Severity**: HIGH

- **User Pain**: Hierarchical lists (common in documents) render incorrectly
- **Data Loss**: Semantic meaning lost (indentation = hierarchy)
- **Competitive Gap**: pymupdf4llm handles this correctly

**First Principles Solution**:
Port `create_list_item_levels` algorithm (document_layout.py:97-151):

```rust
// In pymupdf_grouper.rs or new list_hierarchy.rs
pub fn create_list_item_levels(blocks: &[Block]) -> HashMap<usize, u8> {
    let mut levels = HashMap::new();
    let mut segments = vec![];
    let mut current_segment = vec![];

    // Step 1: Create contiguous segments
    for (i, block) in blocks.iter().enumerate() {
        if !matches!(block.block_type, BlockType::ListItem) {
            if !current_segment.is_empty() {
                segments.push(current_segment);
                current_segment = vec![];
            }
            continue;
        }

        // Check for column break
        if let Some(&(_, prev_idx)) = current_segment.last() {
            let prev_block = &blocks[prev_idx];
            let breaks_column = block.bbox.x0 > prev_block.bbox.x1
                || block.bbox.y1 < prev_block.bbox.y0;
            if breaks_column {
                segments.push(current_segment);
                current_segment = vec![];
            }
        }

        current_segment.push((i, block));
    }
    if !current_segment.is_empty() {
        segments.push(current_segment);
    }

    // Step 2: Assign levels per segment
    for segment in segments {
        let mut sorted_segment = segment.clone();
        sorted_segment.sort_by(|a, b| {
            a.1.bbox.x0.partial_cmp(&b.1.bbox.x0).unwrap()
        });

        let mut prev_level = 1;
        let mut prev_x0 = sorted_segment[0].1.bbox.x0;

        levels.insert(sorted_segment[0].0, 1);

        for &(idx, block) in &sorted_segment[1..] {
            let level = if block.bbox.x0 > prev_x0 + 10.0 {
                prev_level + 1
            } else {
                prev_level
            };
            levels.insert(idx, level);
            prev_level = level;
            prev_x0 = block.bbox.x0;
        }
    }

    levels
}
```

**Complexity**: O(n log n) due to sorting.

**Dependencies**: Requires Block to have bbox field.

---

### Gap 3: Limited Style Preservation (P0)

**Root Cause**: Renderer only handles bold, italic, mono.

**Why It Exists**: MVP focused on most common styles.

**Missing Styles**:

1. **Superscript**: Footnote markers, math exponents
2. **Strikeout**: Edited text, crossed-out content
3. **PUA characters**: Custom bullets, should be omitted

**First Principles Solution**:

```rust
// In pymupdf_renderer.rs

fn render_span_styled(span: &Span) -> String {
    let mut prefix = String::new();
    let mut suffix = String::new();

    // Check font flags (priority order matters!)
    let is_mono = span.flags & 0x08 != 0 && span.font != "GlyphLessFont";
    let is_bold = span.flags & 0x10 != 0 || span.char_flags & 0x08 != 0;
    let is_italic = span.flags & 0x02 != 0;
    let is_strikeout = span.char_flags & 0x01 != 0;
    let is_superscript = span.flags & 0x01 != 0;

    // Build prefix (innermost first)
    if is_mono { prefix.push('`'); }
    if is_bold { prefix.push_str("**"); }
    if is_italic { prefix.push('_'); }
    if is_strikeout { prefix.push_str("~~"); }

    // Suffix is reverse of prefix
    suffix = prefix.chars().rev().collect();

    // Handle PUA characters
    let text = if span.text.len() == 1 && is_pua_char(span.text.chars().next().unwrap()) {
        String::new()
    } else {
        span.text.clone()
    };

    // Handle superscript (footnote markers)
    if is_superscript && text.len() < 5 {
        format!("[{}]", text)  // Wrap in brackets
    } else {
        format!("{}{}{}", prefix, text, suffix)
    }
}

fn is_pua_char(c: char) -> bool {
    let o = c as u32;
    (0xE000..=0xF8FF).contains(&o)
        || (0xF0000..=0xFFFFD).contains(&o)
        || (0x100000..=0x10FFFD).contains(&o)
}
```

**Risk**: Over-styling may make markdown harder to read.

**Mitigation**: Make style preservation configurable via MarkdownConfig.

---

### Gap 4: No Hyphenation Resolution (P1)

**Root Cause**: Line grouping doesn't check for trailing hyphens.

**Example**:

```
PDF:       exam-
           ple

Current:   exam- ple

Desired:   example
```

**First Principles Solution**:

```rust
// In pymupdf_grouper.rs line merging logic

fn should_join_hyphenated(prev_line: &Line, next_line: &Line) -> bool {
    // Get last span of previous line
    let Some(last_span) = prev_line.spans.last() else {
        return false;
    };

    // Check if ends with hyphen
    if !last_span.text.ends_with('-') {
        return false;
    }

    // Check if hyphenated word is long enough (avoid "- list" false positive)
    let last_word = last_span.text.trim_end_matches('-').split_whitespace().last();
    if let Some(word) = last_word {
        if word.len() < 3 {
            return false;  // Too short to be real word
        }
    }

    // Check if lines are close vertically (same paragraph)
    let y_gap = (next_line.bbox.y0 - prev_line.bbox.y1).abs();
    if y_gap > 5.0 {
        return false;  // Different paragraphs
    }

    true
}

fn merge_hyphenated_lines(prev: &mut Line, next: &Line) {
    // Remove trailing hyphen and space
    if let Some(last_span) = prev.spans.last_mut() {
        last_span.text = last_span.text.trim_end_matches("- ").to_string();
    }

    // Append next line's first word without space
    prev.spans.extend(next.spans.clone());
}
```

**Risk**: False positives on lists starting with "- ".

**Mitigation**: Check word length and context.

---

## Solution Approaches: Comparative Analysis

### Approach A: Incremental Port (RECOMMENDED)

**Strategy**: Port pymupdf4llm algorithms one-by-one, testing after each.

**Phases**:

1. **Phase 1 (Iteration 02-05)**: List hierarchy + style preservation
2. **Phase 2 (Iteration 06-10)**: Hyphenation + block type expansion
3. **Phase 3 (Iteration 11-20)**: Table structure completion
4. **Phase 4 (Iteration 21-30)**: Reading order refinement
5. **Phase 5 (Iteration 31-50)**: OCR integration + edge cases

**Pros**:

- ✅ Low risk: Each change is tested independently
- ✅ Clear progress: Each iteration adds measurable value
- ✅ Debuggable: Easy to isolate regressions
- ✅ Parallelizable: Can work on multiple features across iterations

**Cons**:

- ⏳ Slower to complete all features
- 🔄 May require refactoring as architecture evolves

**Risk Level**: LOW

---

### Approach B: Wholesale Rewrite

**Strategy**: Rewrite entire extraction pipeline to match pymupdf4llm exactly.

**Pros**:

- ✅ Guaranteed parity with gold standard
- ✅ Clean architecture from scratch

**Cons**:

- ❌ HIGH RISK: Might break existing functionality
- ❌ Longer testing cycle before any value delivered
- ❌ Harder to debug: Too many changes at once
- ❌ Loses existing strengths (R-tree indexing, Rayon parallelism)

**Risk Level**: HIGH

**Verdict**: ❌ Rejected - Violates First Principle of "Preserve what works well"

---

### Approach C: Hybrid (Adapter Pattern)

**Strategy**: Keep existing pipeline, add pymupdf4llm algorithms as optional processors.

**Architecture**:

```
PDFium → RawChars → Spans → Lines → Blocks
                                      ↓
                        ┌─────────────┴─────────────┐
                        ↓                           ↓
              Current Classifier          Pymupdf4llm Classifier
                        ↓                           ↓
                    Renderer ←────────────────── Renderer
```

**Pros**:

- ✅ Preserves existing code paths
- ✅ Easy A/B testing
- ✅ Gradual migration path

**Cons**:

- ⚠️ Code duplication
- ⚠️ Two pipelines to maintain
- ⚠️ Eventually need to choose one

**Risk Level**: MEDIUM

**Verdict**: ⚠️ Consider if Approach A proves too disruptive

---

## Decision Criteria

### How to Prioritize Improvements

**Impact Matrix**:

```
         │ High User Value │ Medium Value │ Low Value
─────────┼─────────────────┼──────────────┼───────────
High     │ P0: DO NOW      │ P1: NEXT     │ P2: Later
Effort   │ (List Hier)     │ (Hyphen)     │ (OCR)
─────────┼─────────────────┼──────────────┼───────────
Medium   │ P0: DO NOW      │ P1: NEXT     │ P3: Maybe
Effort   │ (Style Preserv) │ (Block Types)│ (Pg Hdr)
─────────┼─────────────────┼──────────────┼───────────
Low      │ P0: DO NOW      │ P1: QUICK WIN│ P3: Easy
Effort   │ (None here)     │ (PUA Filter) │ (Config)
```

**P0 Features (Must Have)**:

1. List item hierarchy (High Value, High Effort)
2. Extended style preservation (High Value, Medium Effort)
3. PUA character filtering (Medium Value, Low Effort)

**P1 Features (Should Have)**:

1. Expanded block types (Medium Value, Medium Effort)
2. Hyphenation resolution (Medium Value, Medium Effort)
3. Table structure completion (High Value, High Effort - defer to Phase 3)

**P2 Features (Nice to Have)**:

1. OCR integration (Low Value for most users, High Effort)
2. Page header/footer filtering (Low Value, Medium Effort)

---

## Risk Assessment

### Implementation Risks

| Risk                              | Probability | Impact | Mitigation                             |
| --------------------------------- | ----------- | ------ | -------------------------------------- |
| Break existing tests              | Medium      | High   | Incremental changes + regression suite |
| Performance regression            | Low         | Medium | Benchmark after each change            |
| False positives in classification | Medium      | Medium | Confidence scoring + fallbacks         |
| Incompatible with PDFium flags    | Low         | High   | Verify flag mappings early             |
| Complexity explosion              | Medium      | High   | Keep modules focused (SRP)             |

### Technical Debt Risks

**Current Debt**:

- `pymupdf_` prefix on modules (confusing since we use PDFium, not PyMuPDF)
- Block type enum mixed with hierarchy levels (ListItem(u8) adds state to type)

**Proposed Refactoring** (Iteration 10+):

```rust
// Separate type from state
enum BlockClass {
    Header, Paragraph, ListItem, Code, Table, Footnote, ...
}

struct Block {
    class: BlockClass,
    level: Option<u8>,  // For lists and headers
    confidence: f32,     // For ambiguous cases
    // ...
}
```

---

## Success Metrics

### Quantitative

**Before** (Baseline - to be measured):

- Text extraction: ~95% (claimed)
- Table detection: Unknown
- Reading order: Unknown
- Structure preservation: Unknown

**After** (Target):

- Text extraction: ≥98%
- Table detection: ≥90%
- Reading order: ≥95%
- List hierarchy: 100% (if pymupdf4llm gets it right, we should too)
- Style preservation: ≥95% (bold, italic, mono, strikeout, superscript)

### Qualitative

- [ ] All existing tests pass
- [ ] Code follows Rust idioms
- [ ] Documentation updated with algorithms
- [ ] Performance: No regression (<10% slowdown acceptable for accuracy gains)
- [ ] Maintainability: SRP followed, modules <500 lines

---

## Next Steps

In **decide.md**, will prioritize:

1. Specific changes for Iteration 01-05
2. File-level implementation plan
3. Test strategy for validation
4. Rollback plan if changes fail

---

**Analysis Complete**: ✅

- [x] First Principles foundations established
- [x] Root causes identified
- [x] Solution approaches evaluated
- [x] Risks assessed
- [x] Success metrics defined
