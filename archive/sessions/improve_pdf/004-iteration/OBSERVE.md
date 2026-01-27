# OBSERVE.md - Iteration 004: First Principles Refactoring

**Directory:** `edgequake/crates/edgequake-pdf/src`

## Mission Statement

**Critical High-Stakes Mission:** Eliminate ALL heuristics and "cheating" patterns from PDF processing code. Replace with first-principles approaches based on PDF structure, not pattern matching or hardcoded rules.

## Code Smells & Heuristics Identified

### 1. SECTION_KEYWORDS Constant (processor.rs:12-72)

**SMELL:** 60+ hardcoded keywords for section detection
**WHY BRITTLE:**

- Only works for English academic papers
- Misses variations, translations, domain-specific terms
- Keyword list grows indefinitely with edge cases

**FIRST PRINCIPLE:** Section headers are identified by:

- Font size larger than body text
- Font weight (bold)
- Positional context (after whitespace, before content)
- Structural markers in PDF (Outline/Bookmarks)

### 2. Magic Number Thresholds

**processor.rs:**

- Line 171: `25.0` - Y-band matching for section numbers
- Line 259: `left_margin: 50.0` - margin filtering
- Line 259: `right_margin: 30.0`
- Line 260: `top_margin: 40.0`
- Line 261: `bottom_margin: 40.0`
- Line 338: `max_short_word_ratio: 0.35` - garbled text detection
- Line 378: String length checks `<= 3`, `<= 6`, `>= 4`, `>= 8`
- Line 697: `150.0` - table column gap threshold
- Line 885: `25.0` - vertical gap for block merge
- Line 891: `100.0` - horizontal zone threshold
- Line 896: `1.5` - font size tolerance

**WHY BRITTLE:** Values chosen empirically for specific documents, fail on different layouts/scales

**FIRST PRINCIPLE:** Use relative measurements:

- Measure actual font sizes from document, use median/percentiles
- Margins: % of page dimensions
- Gaps: relative to character width or line height
- Thresholds: learned from document statistics, not hardcoded

### 3. Hardcoded Word Lists (processor.rs:358-362)

```rust
let valid_short_words = [
    "a", "an", "as", "at", "be", "by", "do", "go", ...
];
```

**WHY BRITTLE:** English-only, misses acronyms, technical terms, other languages

**FIRST PRINCIPLE:** Use statistical measures:

- Token frequency distribution
- Character n-gram models
- Language-agnostic text quality metrics (entropy, compression ratio)

### 4. Specialized Parsers (processor.rs:628-683)

**parse_agent_pipeline_leaderboard():** Hardcoded to parse ONE specific table format

**WHY BRITTLE:** Only works for that exact table, fails on all other layouts

**FIRST PRINCIPLE:** Table detection should:

- Use PDF's line/rectangle drawing commands (borders)
- Detect aligned text regions (geometric clustering)
- Identify cell boundaries from whitespace analysis
- Not assume any specific content or column names

### 5. table_like_score() Heuristic (processor.rs:570-589)

```rust
let mut score = 0;
if multi_space_runs { score += 2; }
if digits >= 3 { score += 2; }
if pipes >= 2 { score += 3; }
```

**WHY BRITTLE:** Arbitrary weights, fails on text with similar patterns

**FIRST PRINCIPLE:** Tables are identified by:

- Consistent vertical alignment of text across rows
- Rectangular whitespace structure
- PDF graphics primitives (lines, borders)
- Not by content pattern matching

### 6. Column Detection (layout/column_detector.rs)

**HEURISTICS:**

- Line 66: `min_gap_width: 30.0`
- Line 67: `min_column_width: 100.0`
- Line 69: `bin_size: 5.0`
- Line 92: `page_width * 0.8` filter threshold
- Line 192: `max_count * 0.35` gap threshold
- Line 193: `avg_count * 0.2` alternative threshold

**WHY BRITTLE:**

- Histogram binning loses precision
- Fixed thresholds fail on different page sizes/scales
- Assumes specific column layouts

**FIRST PRINCIPLE:** Column detection should:

- Use x-coordinate clustering of text blocks (k-means, DBSCAN)
- Measure actual gaps between text (not histogram bins)
- Scale all thresholds relative to page dimensions
- Use reading order heuristics (left-to-right, top-to-bottom)

### 7. Control Character Handling (processor.rs:2088-2120)

**Hardcoded byte values:** `\x02`, `\x1F`, `\xAD` for soft hyphens

**WHY BRITTLE:** Only handles known control codes, misses Unicode variants

**FIRST PRINCIPLE:**

- Use Unicode normalization (NFKC)
- Handle all zero-width and formatting characters systematically
- Use PDF's actual text positioning (TJ operator) to detect word breaks

### 8. Regex Patterns (processor.rs:1853-1856)

```rust
section_regex: Regex::new(
    r"^([0-9A-Z]+\.(?:[0-9]+\.)*)\s+([A-Z][A-Za-z0-9\s,:\-\(\)]+)$"
)
```

**WHY BRITTLE:** Assumes specific formatting, fails on variations

**FIRST PRINCIPLE:** Use PDF structure:

- Outline tree for sections
- Font/style changes for headers
- Geometric layout for section boundaries

## Current Metrics (Baseline)

```
Table Accuracy: 3.5%
Style Accuracy: 16.9%
Robustness: 100.0%
Performance: 90.0%
Composite: 27.2/100
```

## Root Cause Analysis

The code is built on **pattern recognition** rather than **PDF fundamentals**:

1. **Text content matching** instead of structural analysis
2. **Hardcoded thresholds** instead of adaptive algorithms
3. **Special-case handlers** instead of general solutions
4. **Heuristic scoring** instead of geometric/semantic models

## First Principles Approach

PDF files encode:

- **Exact text positioning** (x, y coordinates for each character)
- **Font information** (family, size, weight, style)
- **Graphics primitives** (lines, rectangles for table borders)
- **Logical structure** (Outline, Tagged PDF)

**Strategy:**

1. **Use actual coordinates** - cluster text by position, not histogram bins
2. **Use font metrics** - identify headers by size/weight, not keywords
3. **Use graphics** - detect tables from borders, not content patterns
4. **Make adaptive** - learn thresholds from document statistics

## Next Steps (ORIENT Phase)

1. Design modular architecture:

   - `TextNormalizer`: Unicode normalization, whitespace
   - `GeometricAnalyzer`: Clustering, alignment detection
   - `FontAnalyzer`: Style classification from metrics
   - `TableExtractor`: Border-based detection
   - `LayoutAnalyzer`: Column/region detection

2. Each module:

   - Single responsibility
   - No hardcoded constants
   - Testable in isolation
   - Composable with others

3. Test-driven refactoring:
   - Write tests first
   - Refactor one module at a time
   - Validate metrics don't regress
   - Measure improvements iteratively
