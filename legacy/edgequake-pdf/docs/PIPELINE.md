# PDF Processing Pipeline

> **Deep Dive**: Document transformation pipeline from raw blocks to structured Markdown. This document explains the 13-processor chain that runs after PDF extraction.

**Module**: [src/processors/](src/processors/)  
**Entry Point**: [PdfExtractor::apply_processors()](src/extractor.rs#L260-L290)  
**Total Processors**: 13 sequential transformations

---

## Pipeline Overview

```
Document (from Backend)
    │
    ├─► MarginFilterProcessor        [1] Remove line numbers, page headers
    ├─► GarbledTextFilterProcessor    [2] Remove OCR artifacts
    ├─► LayoutProcessor               [3] Detect columns, sort reading order
    ├─► SectionNumberMergeProcessor   [4] Join "3.2" + "Methods" → "3.2 Methods"
    ├─► StyleDetectionProcessor       [5] Mark H1/H2 by font size
    ├─► HeaderDetectionProcessor      [6] Detect section headers
    ├─► SectionPatternProcessor       [7] Regex-based section detection
    ├─► CaptionDetectionProcessor     [8] "Figure 1:", "Table 2:"
    ├─► TextTableReconstructionProcessor [9] Ascii-art → structured tables
    ├─► ListDetectionProcessor        [10] Bullet/numbered lists
    ├─► CodeBlockDetectionProcessor   [11] Monospace font detection
    ├─► HyphenContinuationProcessor   [12] Fix "hyphen-\nated"
    ├─► BlockMergeProcessor           [13] Merge adjacent paragraphs
    └─► PostProcessor                 [14] Final cleanup, stats

Enhanced Document → MarkdownRenderer → Output
```

**Pipeline Invariant**: Each processor receives a valid `Document` and returns a transformed `Document`. Processors are composable and order-dependent.

---

## Processor Contract

```rust
pub trait Processor: Send + Sync {
    fn process(&self, document: Document) -> Result<Document>;
    fn name(&self) -> &str;
}
```

**Requirements**:

1. **Idempotent where possible**: Re-running should be safe
2. **Error-tolerant**: Don't fail on edge cases, log warnings
3. **Thread-safe**: Processors must be `Send + Sync`
4. **Single Responsibility**: Each processor has one job

---

## Phase 1: Cleanup (Processors 1-2)

### MarginFilterProcessor

**Purpose**: Remove margin content (line numbers, page numbers, headers/footers)

**Algorithm**:

```
1. Calculate Page Margins:
   ├── left_margin   = page_width × 0.10   (10% from left edge)
   ├── right_margin  = page_width × 0.90   (10% from right edge)
   ├── top_margin    = page_height × 0.95  (5% from top)
   └── bottom_margin = page_height × 0.05  (5% from bottom)

2. For Each Block:
   ├── If bbox entirely within margin zones → REMOVE
   ├── If text matches line number pattern (^\d+$) → REMOVE
   └── If text is single digit (page number) → REMOVE

3. Detect Running Headers:
   ├── Group blocks by Y-position across pages
   ├── If same text appears on 3+ pages at same Y → REMOVE ALL
   └── Example: "Chapter 3: Methods" on every page
```

**Edge Cases**:

- **Short pages**: Margins adapt to page dimensions
- **Wide margins**: If left/right margins too large, fall back to absolute 50pt
- **False positives**: Single-digit page numbers near content aren't removed if embedded

**WHY adaptive margins**: Documents vary in size (letter/A4/legal). Percentage-based margins handle all formats.

**Code**: [processors/layout_processing.rs#L300-L420](src/processors/layout_processing.rs#L300-L420)

```
Visual Example:
┌───────────────────────────┐
│ 1  ← Line number (REMOVE) │
│                            │
│    Introduction            │
│    Lorem ipsum dolor...    │
│                            │
│                         5  │ ← Page number (REMOVE)
└───────────────────────────┘
```

---

### GarbledTextFilterProcessor

**Purpose**: Remove OCR artifacts and garbled text from figures

**Detection Patterns**:

```rust
1. Random Character Sequences:
   ├── "xkcd1234abc" → high entropy, no vowels
   ├── "⊕⊗⊙⊚" → only symbols
   └── "ƒµ∂∑∏" → mathematical symbols without context

2. Garbled Figure Annotations:
   ├── Text over figures: "aAbBcC12#$"
   ├── OCR misreads of graph axes
   └── Scattered single chars: "a b c d e" with large gaps

3. Entropy Threshold:
   entropy = -Σ(p(char) × log(p(char)))
   if entropy > 4.5 → likely garbled
```

**Algorithm**:

```
For Each Block:
  1. Calculate character distribution
  2. Compute Shannon entropy
  3. Count vowel ratio
  4. If (entropy > 4.5 OR vowel_ratio < 0.1) AND len < 50:
       Mark as garbled → REMOVE
  5. If contains only punctuation/symbols:
       REMOVE
```

**WHY entropy**: Real text has predictable character distributions. Random OCR errors have high entropy (uniform distribution).

**Code**: [processors/text_cleanup.rs#L120-L180](src/processors/text_cleanup.rs#L120-L180)

---

## Phase 2: Layout Analysis (Processor 3)

### LayoutProcessor

**Purpose**: Detect column structure and sort blocks by reading order

**Column Detection Algorithm**:

```
1. Build X-Position Histogram:
   ├── Bin width = page_width / 50
   ├── For each block: histogram[bin] += char_count
   └── Smooth with 3-bin moving average

2. Find Gaps:
   ├── Threshold = average_bin_count / 3
   ├── Gap = consecutive bins below threshold
   └── Min gap width = 30pt (avoid noise)

3. Classify Layout:
   ├── No gaps → Single column
   ├── 1 major gap → Two columns (boundary at gap center)
   └── 2+ gaps → Three columns (or table structure)

4. Table Detection Guard:
   ├── If columns form regular grid (equal widths)
   ├── AND vertical alignment is strict (< 5pt variance)
   └── Treat as table, not multi-column text
```

**Reading Order Algorithm**:

```
Single Column:
  Sort by Y↓, then X→ (top-to-bottom, left-to-right)

Multi-Column:
  1. Assign blocks to columns by center_x
  2. Sort within each column by Y↓
  3. Merge columns: [left_col blocks, right_col blocks]
  4. Handle spanning elements (e.g., page-wide headers):
       Insert at appropriate Y-position across columns
```

**Visual Example (Two-Column Paper)**:

```
┌─────────────┬─────────────┐
│ Abstract    │             │  ← Spanning (full width)
├─────────────┼─────────────┤
│ This paper  │ networks    │  ← Two columns
│ presents a  │ to improve  │
│ novel...    │ accuracy... │
│             │             │
│ (Column 1)  │ (Column 2)  │
└─────────────┴─────────────┘

Reading Order: Abstract → Col1 → Col2
Not: Abstract → "This" → "networks" (zigzag)
```

**WHY histogram-based**: Robust to slight column misalignments. Projection histograms aggregate evidence across entire page.

**Code**:

- [processors/layout_processing.rs#L40-L85](src/processors/layout_processing.rs#L40-L85)
- [layout/column_detector.rs#L45-L120](src/layout/column_detector.rs#L45-L120)

---

## Phase 3: Structure Detection (Processors 4-8)

### SectionNumberMergeProcessor

**Purpose**: Join standalone section numbers with their titles

**Pattern Detection**:

```
Block N:   "3.2"
Block N+1: "Methods and Approach"
           ↓ MERGE
Block N:   "3.2 Methods and Approach"
```

**Algorithm**:

```
For consecutive blocks (a, b):
  1. Check if 'a' is pure section number:
       matches ^\d+(\.\d+)*\.?$
       examples: "3", "3.2", "1.2.3."

  2. Check if 'b' starts with capital letter
  3. Check vertical proximity:
       (a.bbox.y1 - b.bbox.y2) < 20pt

  4. If all pass:
       b.text = a.text + " " + b.text
       REMOVE block 'a'
```

**WHY this happens**: Some PDFs extract section numbers as separate text elements due to font changes or positioning.

**Edge Cases**:

- **List items**: "1. Item text" → Don't merge (detected by list processor)
- **Addresses**: "123 Main St" → Not a section number (contains non-digits)

**Code**: [processors/layout_processing.rs#L440-L510](src/processors/layout_processing.rs#L440-L510)

---

### StyleDetectionProcessor

**Purpose**: Detect heading levels (H1/H2) using font size analysis

**Algorithm (Geometric Mean)**:

```
1. Calculate Document Font Statistics:
   ├── For each block: char_count × font_size
   ├── weighted_sum = Σ(block.font_size × block.char_count)
   ├── total_chars = Σ(block.char_count)
   └── avg_font_size = weighted_sum / total_chars

2. For Each Block:
   ├── size_ratio = block.font_size / avg_font_size
   ├── If ratio ≥ 1.5 → H1 (Major heading, e.g., "Introduction")
   ├── If ratio ≥ 1.2 → H2 (Subheading, e.g., "2.1 Background")
   └── Else → Normal text

3. Update Block:
   ├── block.block_type = SectionHeader
   └── block.spans[0].heading_level = 1 or 2
```

**Visual Example**:

```
Document:
  Body text (10pt): 95% of chars → avg = 10pt
  Section "Intro" (15pt): ratio = 15/10 = 1.5 → H1
  Subsection "2.1" (12pt): ratio = 12/10 = 1.2 → H2
```

**WHY Ratios**:

- Papers use varied base fonts (10pt, 11pt, 12pt)
- Absolute thresholds (e.g., "≥14pt = heading") fail across documents
- Ratios adapt: 18pt in 12pt doc = 1.5x → H1
  15pt in 10pt doc = 1.5x → H1

**WHY Weighted Average**:

- Prevents skew from small text (captions, footnotes)
- Large text blocks (body) dominate average

**Code**:

- [processors/processor.rs#L320-L450](src/processors/processor.rs#L320-L450)
- [processors/font_analysis.rs#L20-L80](src/processors/font_analysis.rs#L20-L80)
- [processors/heading_classifier.rs#L15-L60](src/processors/heading_classifier.rs#L15-L60)

---

### HeaderDetectionProcessor

**Purpose**: Detect section headers using numbering patterns and font evidence

**Multi-Signal Detection**:

```
Detection Hierarchy (in order):
1. Subsection patterns: "1.1", "2.3.4" → H3+ (by dot count)
2. Single-number sections: "2.", "3." → H2 (needs font validation)
3. Font-size based: Large font + short text → Header
4. Position-aware: First page, top → Title (allow longer text)
```

**Pattern Matching**:

```rust
Subsection: r"^\d+\.\d+(?:\.\d+)*\.?\s+[A-Z]"
  Examples: "1.1 Motivation", "2.3.4. Deep Networks"
  Level: dot_count + 2
    "1.1" → 1 dot → H3
    "1.1.1" → 2 dots → H4

Single Number: r"^\d+\.?\s+[A-Z]"
  Examples: "2. Methods", "3 Results"
  Validation Required:
    - Font size > body × 1.15 OR bold
    - No commas (avoid addresses: "353 Serra Mall, Stanford")
    - Title case after number
```

**Position-Aware Length Thresholds**:

```
First page, top of page, OR large font:
  max_heading_len = 150 chars  ← Document titles
Elsewhere:
  max_heading_len = 80 chars   ← Section headers
```

**Guards Against False Positives**:

```
1. Inline Descriptions:
   "Author: John Doe" → Has colon + lowercase key → NOT header

2. List Items:
   "1. This is a list item, not a section."
   → Has ending period → NOT header

3. Addresses:
   "353 Serra Mall, Stanford, CA"
   → Contains comma → NOT header
```

**Code**: [processors/structure_detection.rs#L25-L200](src/processors/structure_detection.rs#L25-L200)

---

### CaptionDetectionProcessor

**Purpose**: Detect figure and table captions

**Pattern Detection**:

```rust
Patterns (case-insensitive):
  Figure: r"^Figure\s+\d+"   e.g., "Figure 1: Network architecture"
  Table:  r"^Table\s+\d+"    e.g., "Table 2. Results on ImageNet"
  Fig:    r"^Fig\.?\s+\d+"   e.g., "Fig. 3: Ablation study"

Caption Structure:
  "Figure 1: Main description here."
  └─┬─┘  └┬┘ └────────┬─────────────┘
   Type  Num     Description
```

**Algorithm**:

```
For Each Block:
  1. Check if text matches caption pattern
  2. Validate position:
       ├── Typically above/below figures or tables
       ├── Font size often smaller than body (9pt vs 10pt)
       └── May be centered or italicized

  3. If match:
       block.block_type = Caption
       block.caption_type = Figure | Table
       block.caption_num = extracted number
```

**WHY Important**: Captions provide context for figures. Extracted separately from figure content for proper Markdown rendering:

```markdown
![Figure 1](image.png)
**Figure 1**: Network architecture showing...
```

**Code**: [processors/structure_detection.rs#L220-L290](src/processors/structure_detection.rs#L220-L290)

---

### SectionPatternProcessor

**Purpose**: Detect section headers from special keywords and font size

**Special Section Keywords**:

```rust
const SPECIAL_SECTIONS: &[&str] = &[
    "Abstract", "Introduction", "Related Work",
    "Background", "Methodology", "Methods",
    "Approach", "Experiments", "Results",
    "Discussion", "Conclusion", "Conclusions",
    "Future Work", "Acknowledgments",
    "Acknowledgements", "References",
    "Bibliography", "Appendix"
];
```

**Detection Logic**:

```
For Each Block:
  1. Extract first word (case-insensitive)
  2. If word in SPECIAL_SECTIONS:
       ├── AND font_size > avg × 1.15
       ├── OR is_bold
       └── OR is first block on page
       → Mark as SectionHeader (H1)

  3. Running Header Detection:
       ├── If same text appears on 3+ pages at same Y-position
       └── → Mark as PageHeader (for removal)
```

**WHY Keywords + Font Evidence**: Keywords alone trigger false positives (e.g., "We present our approach..." contains "approach"). Font evidence confirms intent.

**Code**: [processors/processor.rs#L115-L240](src/processors/processor.rs#L115-L240)

---

## Phase 4: Content Restructuring (Processors 9-11)

### TextTableReconstructionProcessor

**Purpose**: Detect and reconstruct ASCII-art tables in plain text

**Detection Patterns**:

```
Ascii Table Examples:

1. Box-drawing characters:
┌─────────┬─────────┐
│ Header1 │ Header2 │
├─────────┼─────────┤
│ Cell1   │ Cell2   │
└─────────┴─────────┘

2. Pipe-delimited:
| Method | Accuracy |
|--------|----------|
| Ours   | 92.3%    |

3. Space-aligned:
Method      Accuracy    Speed
------      --------    -----
Baseline    85.2%       10ms
Ours        92.3%       15ms
```

**Detection Algorithm**:

```
For Each Block:
  1. Count special characters:
       ├── Box chars: ┌ ─ ├ │ ┐ ┴ └ ┤ ┼ ┬
       ├── Pipes: | (if 3+ per line)
       └── Equals/Dash: === or --- (horizontal separators)

  2. Calculate table_score:
       score = (box_char_count × 10 + pipe_count × 5) / text.len()

  3. If score > 0.15 OR has clear grid structure:
       → Parse as table

  4. Extract Structure:
       ├── Identify separator lines (----, ====)
       ├── Split rows by newlines
       ├── Detect column boundaries (alignment analysis)
       └── Build structured table with cells
```

**Parsing Example**:

```
Input Text:
| Method | Acc  |
|--------|------|
| Base   | 85%  |
| Ours   | 92%  |

Parsed Structure:
Table {
  headers: ["Method", "Acc"],
  rows: [
    ["Base", "85%"],
    ["Ours", "92%"]
  ]
}
```

**Output Markdown**:

```markdown
| Method | Acc |
| ------ | --- |
| Base   | 85% |
| Ours   | 92% |
```

**Code**: [processors/table_detection.rs#L400-L600](src/processors/table_detection.rs#L400-L600)

---

### ListDetectionProcessor

**Purpose**: Detect and structure bullet and numbered lists

**Bullet Patterns**:

```
Bullet Markers:
  - Dash: "- Item text"
  * Asterisk: "* Item text"
  • Bullet: "• Item text"
  → Arrow: "→ Item text"

Numbered Markers:
  1. Digit-period: "1. First item"
  a) Letter-paren: "a) Sub-item"
  i. Roman: "i. Roman numeral"
  (1) Paren-digit: "(1) Enclosed"
```

**Detection Algorithm**:

```
For Each Block:
  1. Check line starts:
       regex: r"^(\s*)([•\-\*→]|\d+\.|\w+\))\s+"

  2. Detect indentation level:
       indent_level = leading_spaces / 4
       Example: "    - Sub-item" → level 1 (4 spaces)

  3. Classify:
       ├── Bullet list (-, *, •)
       └── Numbered list (1., a), i.)

  4. Nest by indentation:
       Top-level (0 spaces) → List root
       Indented (4+ spaces) → Nested sub-list
```

**Nesting Example**:

```
Text:
1. First item
   - Sub-item A
   - Sub-item B
2. Second item

Structure:
List (numbered)
 ├── Item 1: "First item"
 │    └── List (bullet)
 │         ├── "Sub-item A"
 │         └── "Sub-item B"
 └── Item 2: "Second item"
```

**Markdown Output**:

```markdown
1. First item
   - Sub-item A
   - Sub-item B
2. Second item
```

**Code**: [processors/structure_detection.rs#L310-L410](src/processors/structure_detection.rs#L310-L410)

---

### CodeBlockDetectionProcessor

**Purpose**: Detect code blocks from monospace fonts and indentation

**Detection Signals**:

```
1. Monospace Font Names:
   ├── "Courier", "Monaco", "Consolas"
   ├── "Menlo", "DejaVu Sans Mono"
   └── "Source Code Pro", "Fira Code"

2. Consistent Indentation:
   ├── 4+ spaces at line start
   └── Maintained across multiple lines

3. Code Patterns:
   ├── function definitions: "def foo():", "function bar() {"
   ├── Variable declarations: "int x = 5;"
   ├── Keywords: "if", "for", "while", "class"
   └── Syntax: {}, [], (), ;

4. High Symbol Density:
   symbols / total_chars > 0.20
   e.g., "arr[i] = obj->value;" → 7 symbols / 22 chars = 0.32
```

**Detection Algorithm**:

```
For Each Block:
  1. Check font:
       if block.spans[0].font_name in MONOSPACE_FONTS:
         score += 50

  2. Check indentation:
       if all lines start with 4+ spaces:
         score += 30

  3. Check patterns:
       if contains keywords (def, class, function, etc.):
         score += 20

  4. Check symbol density:
       symbols = {, }, [, ], (, ), ;, :
       if density > 0.20:
         score += 20

  5. If score ≥ 60:
       block.block_type = Code
       block.language = detect_language()  // heuristics
```

**Language Detection** (heuristics):

```rust
fn detect_language(text: &str) -> Option<String> {
    if text.contains("def ") && text.contains(":") → "python"
    if text.contains("function") && text.contains("{") → "javascript"
    if text.contains("public class") → "java"
    if text.contains("fn ") && text.contains("->") → "rust"
    // ... more patterns
    else → None (generic code block)
}
```

**Markdown Output**:

````markdown
```python
def hello():
    print("Hello, world!")
```
````

**Code**: [processors/structure_detection.rs#L430-L517](src/processors/structure_detection.rs#L430-L517)

---

## Phase 5: Text Cleanup (Processor 12)

### HyphenContinuationProcessor

**Purpose**: Fix hyphenated words split across line breaks

**Problem**:

```
PDF rendering:
"This demonstrates state-of-the-
art methods for neural networks."

Extracted text:
"This demonstrates state-of-the-\nart methods..."
                               ^^^ hyphen at line break
```

**Algorithm**:

```
1. Pattern Detection:
   regex: r"(\w+)-\s*\n\s*(\w+)"
   Matches: "word-\nword" with optional whitespace

2. Validation:
   ├── Check if rejoined word exists in dictionary (optional)
   ├── Verify no punctuation after hyphen (avoid em-dashes)
   └── Confirm lowercase after break (avoid proper nouns)

3. Replacement:
   "state-of-the-\nart" → "state-of-the-art"
   "neural net-\nworks" → "neural networks"

4. Edge Cases:
   ├── "end-of-line." → Keep (period = sentence end)
   ├── "A. Smith-\nJones" → Keep (proper noun)
   └── "5-\n10" → Keep (number range)
```

**WHY Complex**: Not all line-break hyphens are continuations:

- **Em-dashes**: "The method—discussed earlier—works well."
- **Compound words**: "state-of-the-art" (keep hyphens)
- **Ranges**: "pages 5-10" (keep hyphen)

**Code**: [processors/text_cleanup.rs#L420-L510](src/processors/text_cleanup.rs#L420-L510)

---

## Phase 6: Final Processing (Processors 13-14)

### BlockMergeProcessor

**Purpose**: Merge adjacent text blocks that belong to the same paragraph

**Merge Criteria (ALL must be true)**:

```
1. Compatible Types:
   ├── Text + Text → OK
   ├── SectionHeader + SectionHeader → OK (for multi-line headers)
   └── Text + Table → NEVER

2. Vertical Proximity:
   gap = |block_a.bbox.y1 - block_b.bbox.y2|
   max_gap = DocumentStats.typical_line_spacing × 2.5
   ├── If gap < max_gap → MAY merge
   └── Else → DON'T merge

3. Horizontal Alignment:
   margin_diff = |block_a.bbox.x1 - block_b.bbox.x1|
   max_margin = DocumentStats.column_alignment_tolerance
   ├── If margin_diff < max_margin → ALIGNED
   └── If margin_diff > page_width × 0.15 → DIFFERENT COLUMNS (don't merge)

4. Style Consistency:
   ├── Font size difference < 1.5pt
   ├── Font weight: (bold + bold) OR (normal + normal)
   └── Don't merge bold header with normal text

5. List Item Guard:
   If block_b starts with bullet or number → DON'T merge
   Examples: "- Item", "1. Item", "• Item"
```

**Adaptive Thresholds** (from DocumentStats):

```rust
struct DocumentStats {
    typical_line_spacing: f32,        // Median inter-line gap
    column_alignment_tolerance: f32,  // Max X-diff within column
    page_width: f32,                  // For column separation check
}

// Computed during processing:
DocumentStats::from_document(&doc)
  typical_line_spacing = median(all vertical gaps)
  column_alignment_tolerance = stddev(left margins) × 2
```

**Merge Example**:

```
BEFORE:
Block 1: "This paper presents a novel"
Block 2: "approach to neural networks."
         (gap = 15pt, margin_diff = 2pt, same font)

AFTER:
Block 1: "This paper presents a novel approach to neural networks."
```

**Why Merge**: PDF extraction often splits paragraphs arbitrarily at line breaks. Merging reconstructs logical paragraphs for readable Markdown.

**Code**: [processors/layout_processing.rs#L100-L245](src/processors/layout_processing.rs#L100-L245)

---

### PostProcessor

**Purpose**: Final text cleanup and statistics computation

**Operations**:

```
1. Text Normalization:
   ├── Trim leading/trailing whitespace from blocks
   ├── Collapse multiple spaces: "word    word" → "word word"
   ├── Normalize quotes: "" → "" (typographic to ASCII)
   └── Fix Unicode: ligatures (ﬁ → fi, ﬂ → fl)

2. Block Cleanup:
   ├── Remove empty blocks (text.trim().is_empty())
   ├── Remove duplicate blocks (same text + same position)
   └── Filter very short blocks (< 3 chars, likely noise)

3. Statistics Update:
   ├── page.stats = PageStats::from_blocks(&page.blocks)
   ├── Count: text_blocks, tables, figures, headers, equations
   └── Compute: char_count, word_count, avg_confidence

4. Metadata Finalization:
   ├── doc.metadata.processing_time_ms = elapsed
   └── doc.metadata.extraction_method = ExtractionMethod::Native
```

**Unicode Normalization**:

```rust
Ligatures:
  ﬁ (U+FB01) → "fi"
  ﬂ (U+FB02) → "fl"
  ﬀ (U+FB00) → "ff"
  ﬃ (U+FB03) → "ffi"
  ﬄ (U+FB04) → "ffl"

Smart Quotes:
  " " (U+201C/D) → "\""
  ' ' (U+2018/9) → "'"

Em/En Dashes:
  — (U+2014) → "--"
  – (U+2013) → "-"
```

**Code**: [processors/text_cleanup.rs#L40-L120](src/processors/text_cleanup.rs#L40-L120)

---

## Pipeline Metrics

**Performance** (12-page academic paper):

```
Processor                      Time    % Total
──────────────────────────────────────────────
MarginFilterProcessor           8ms      4%
GarbledTextFilterProcessor      5ms      3%
LayoutProcessor                35ms     19%
SectionNumberMergeProcessor     3ms      2%
StyleDetectionProcessor        12ms      7%
HeaderDetectionProcessor       18ms     10%
SectionPatternProcessor         8ms      4%
CaptionDetectionProcessor       4ms      2%
TextTableReconstructionProcessor 25ms   14%
ListDetectionProcessor         15ms      8%
CodeBlockDetectionProcessor     6ms      3%
HyphenContinuationProcessor    10ms      5%
BlockMergeProcessor            20ms     11%
PostProcessor                  11ms      6%
──────────────────────────────────────────────
TOTAL                         180ms    100%
```

**Bottlenecks**:

1. **LayoutProcessor (35ms)**: XY-Cut recursion on complex layouts
2. **TextTableReconstructionProcessor (25ms)**: Regex-heavy parsing
3. **BlockMergeProcessor (20ms)**: O(n²) merge checking (optimized with early exits)

**Optimization Opportunities**:

- **Parallel Execution**: Some processors are independent (e.g., Caption + List detection)
- **Caching**: Font statistics computed once per document
- **Early Termination**: BlockMergeProcessor uses distance heuristics to skip distant blocks

---

## Processor Dependencies

```
Dependency Graph:
──────────────────

MarginFilter ────┐
                 │
GarbledFilter ───┼──► Layout ──► SectionMerge ──► StyleDetection ──┐
                 │                                                  │
                 └────────────────────────────────────────────────► Header ──► SectionPattern
                                                                         │
                                                                         ├──► Caption
                                                                         │
                                                                         ├──► TextTable
                                                                         │
                                                                         ├──► ListDetection
                                                                         │
                                                                         └──► CodeBlock
                                                                              │
                                                                              ▼
                                                                        HyphenContinuation
                                                                              │
                                                                              ▼
                                                                        BlockMerge
                                                                              │
                                                                              ▼
                                                                        PostProcessor
```

**Critical Dependencies**:

1. **Layout must run before SectionMerge**: Reading order required
2. **StyleDetection before Header**: Font stats needed for header classification
3. **HyphenContinuation before BlockMerge**: Prevents merging split words
4. **PostProcessor must run last**: Final cleanup

**Independent Processors** (could parallelize):

- Caption + List + Code detection (no cross-dependencies)
- TextTable + GarbledFilter (different block types)

---

## Testing Strategy

**Test Coverage by Processor**:

```
Processor                      Unit Tests  Integration Tests
─────────────────────────────────────────────────────────────
MarginFilterProcessor             12             3
LayoutProcessor                   18             5
BlockMergeProcessor               15             4
StyleDetectionProcessor           10             2
HeaderDetectionProcessor          22             6
TableReconstructionProcessor      14             8
ListDetectionProcessor            11             3
HyphenContinuationProcessor        8             2
PostProcessor                      6             1
─────────────────────────────────────────────────────────────
TOTAL                            116            34
```

**Test Types**:

1. **Unit Tests**: Single processor in isolation

   ```rust
   #[test]
   fn test_merge_adjacent_paragraphs() {
       let processor = BlockMergeProcessor::new();
       let doc = create_test_doc_with_split_paragraph();
       let result = processor.process(doc).unwrap();
       assert_eq!(result.pages[0].blocks.len(), 1); // Merged
   }
   ```

2. **Integration Tests**: Full pipeline

   ```rust
   #[tokio::test]
   async fn test_full_pipeline() {
       let extractor = PdfExtractor::new(mock_provider());
       let doc = extractor.extract_document(test_pdf).await.unwrap();
       assert!(doc.pages[0].blocks.iter().any(|b|
           b.block_type == BlockType::SectionHeader
       ));
   }
   ```

3. **Golden Tests**: Compare against reference output
   ```bash
   pytest tests/golden_tests/test_pipeline.py
   # Validates Markdown output against 120 gold files
   ```

**Code**: [tests/](../tests/)

---

## Common Issues & Solutions

### Issue 1: Headers Not Detected

**Symptom**: Section titles rendered as normal text

**Diagnosis**:

```
1. Check font size ratio:
   avg_font_size = 10pt
   header_size = 11pt
   ratio = 1.1 (< 1.2 threshold) → NOT detected

2. Check processor order:
   If BlockMergeProcessor runs before HeaderDetectionProcessor,
   headers merged into paragraphs before detection
```

**Solutions**:

- Adjust threshold: `StyleDetectionProcessor` ratio 1.2 → 1.15
- Fix processor order (HeaderDetectionProcessor before BlockMergeProcessor)
- Add font weight detection (bold → likely header)

---

### Issue 2: Table Reconstruction Fails

**Symptom**: Ascii tables not converted to Markdown tables

**Diagnosis**:

```
1. Check table score:
   score = (box_chars × 10 + pipes × 5) / text.len()
   If score < 0.15 → Not detected

2. Check for irregular spacing:
   |Col1  | Col2    |  ← Inconsistent padding
   |Value1|Value2   |

3. Check for nested structures:
   ┌────────────────┐
   │ Outer table    │
   │ ┌──────┐       │  ← Nested boxes confuse parser
   │ │Inner │       │
   │ └──────┘       │
   └────────────────┘
```

**Solutions**:

- Lower threshold: 0.15 → 0.10 for sparse tables
- Add tolerance for spacing irregularities
- Detect nested structures, parse outer table only

---

### Issue 3: Hyphenation Not Fixed

**Symptom**: "state-of-the-\nart" remains split

**Diagnosis**:

```
1. Check pattern:
   regex: r"(\w+)-\s*\n\s*(\w+)"
   If extra whitespace: "word- \n  word" → May not match

2. Check dictionary:
   If rejoined "state-of-the-art" not in dictionary,
   may be rejected as false positive

3. Check block boundaries:
   If hyphen at block boundary (not line boundary),
   HyphenContinuationProcessor won't see it
```

**Solutions**:

- Relax regex: `\s*` → `\s{0,5}` (allow up to 5 spaces)
- Disable dictionary check (too restrictive)
- Run HyphenContinuationProcessor after BlockMergeProcessor

---

## Extension: Custom Processors

### Template for New Processor:

```rust
use crate::processors::Processor;
use crate::schema::{Document, Block, BlockType};
use crate::Result;

pub struct MyCustomProcessor {
    // Configuration
    threshold: f32,
}

impl MyCustomProcessor {
    pub fn new() -> Self {
        Self { threshold: 0.5 }
    }

    pub fn with_threshold(threshold: f32) -> Self {
        Self { threshold }
    }

    // Helper methods
    fn process_block(&self, block: &mut Block) {
        // Transform block
    }
}

impl Processor for MyCustomProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            for block in &mut page.blocks {
                self.process_block(block);
            }
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "MyCustomProcessor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_processor() {
        let processor = MyCustomProcessor::new();
        let doc = create_test_document();
        let result = processor.process(doc).unwrap();
        // Assert transformations
    }
}
```

### Adding to Pipeline:

```rust
// In extractor.rs
fn apply_processors(&self, document: Document) -> Result<Document> {
    ProcessorChain::new()
        .add(MarginFilterProcessor::new())
        // ... existing processors
        .add(MyCustomProcessor::new())  // ← Insert here
        .add(PostProcessor::new())
        .process(document)
}
```

---

## Related Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md): System overview, module architecture
- [TABLE_DETECTION.md](TABLE_DETECTION.md): Lattice engine deep dive (next doc)
- [EXTRACTION_ENGINE.md](EXTRACTION_ENGINE.md): Backend internals (next doc)
- [TEST_PROTOCOL.md](TEST_PROTOCOL.md): Testing methodology

---

## Document Metadata

**Created**: 2026-01-03  
**Revision**: 1.0  
**Code References**: 65+ direct links  
**Test Coverage**: 150 tests covering pipeline processors
