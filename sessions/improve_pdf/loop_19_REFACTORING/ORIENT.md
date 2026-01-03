# OODA Loop 19: Code Refactoring for Modularity

**Phase:** ORIENT  
**Date:** 2025-05-01  
**Author:** Refactoring Initiative

## Architecture Design

### New Module Structure

```
edgequake-pdf/src/processors/
├── mod.rs                      # Module exports
├── processor.rs                # Core processor implementations
├── font_analysis.rs           # NEW: FontAnalyzer module (130 lines)
└── heading_classifier.rs      # NEW: HeadingClassifier module (180 lines)
```

### FontAnalyzer Module

**Single Responsibility:** Statistical analysis of font sizes in PDF documents

**Interface:**

```rust
pub struct FontAnalyzer;

impl FontAnalyzer {
    pub fn new() -> Self;
    pub fn detect_body_font_size(&self, document: &Document) -> f32;
    fn calculate_median(sizes: &mut [f32]) -> f32;
    fn is_valid_size(size: f32) -> bool;
}
```

**First Principles Design:**

- **Median over Mean**: Robust to outliers (large headings don't skew baseline)
- **Filtering Strategy**: Only consider Text/Paragraph blocks (exclude headers)
- **Sanity Bounds**: Reject sizes < 0 or > 100 pt (prevents corrupted data)
- **Default Fallback**: 12pt if no valid sizes found (typical body font)

**Why These Choices:**

- Median is statistically robust for skewed distributions
- Headers/footers contaminate mean calculation
- Real-world documents typically use 9-14pt body fonts
- Fail-safe behavior prevents downstream errors

### HeadingClassifier Module

**Single Responsibility:** Geometric heading detection based on font size ratios

**Interface:**

```rust
pub struct HeadingClassifier;

impl HeadingClassifier {
    pub fn new() -> Self;
    pub fn classify(&self, block: &Block, body_size: f32) -> (bool, u8);
    fn calculate_level(&self, size_ratio: f32) -> u8;
    fn is_valid_heading_text(&self, text: &str) -> bool;
}
```

**Geometric Classification:**

- **H2 (level 2)**: Font size ≥ 1.8x body (very large)
- **H3 (level 3)**: Font size ≥ 1.5x body (large)
- **H4 (level 4)**: Font size ≥ 1.3x body (medium)
- **H5 (level 5)**: Font size ≥ 1.2x body (slightly large)

**Validation Heuristics:**

- **Consistency Check**: 80% of spans must have large font
- **Length Filter**: Must be < 100 characters (headings are short)
- **Punctuation Rule**: No trailing period (body text often ends with .)
- **Case Check**: Has lowercase letters (prevents ALL CAPS running headers)

**Why These Ratios:**

- Based on empirical analysis of academic papers
- 1.8x = typical H2 in LaTeX documents (14pt body → 25pt heading)
- 1.5x = common H3 (14pt → 21pt)
- 1.3x = H4 subsections (14pt → 18pt)
- 1.2x = minimum perceptible size difference

### Refactored SectionPatternProcessor

**Updated Responsibilities:**

1. **Pattern Matching**: Numbered sections (1., 3.2., etc.)
2. **Semantic Detection**: Special section names (Abstract, References)
3. **Running Header Detection**: Cross-page text repetition
4. **Orchestration**: Delegates to FontAnalyzer and HeadingClassifier

**Changed Structure:**

```rust
pub struct SectionPatternProcessor {
    section_regex: Regex,
    special_sections: Vec<&'static str>,
    font_analyzer: FontAnalyzer,        // NEW: Delegation
    heading_classifier: HeadingClassifier, // NEW: Delegation
}
```

**Processing Strategy (Hierarchical):**

1. **Running Headers** (highest priority) → Prevents false positives
2. **Numbered Sections** → Explicit structure
3. **Special Section Names** → Semantic knowledge
4. **Font-Size Detection** → Geometric fallback (via HeadingClassifier)

**Why This Order:**

- Running headers must be filtered first (appear on every page)
- Numbered sections are most reliable (explicit structure)
- Special names are domain-specific (known section types)
- Font size is last resort (works when patterns/names fail)

## Design Rationale

### Why Separate Modules?

**FontAnalyzer Separation:**

- Can be reused by other processors (not just SectionPatternProcessor)
- Independently testable (unit tests without PDF processing overhead)
- Single concern: statistical analysis, no document semantics
- Future-proof: easy to add percentile-based detection, multi-modal analysis

**HeadingClassifier Separation:**

- Geometric classification is orthogonal to pattern matching
- Can be used for TOC extraction, outline generation
- Testable with mock blocks (no full document needed)
- Easy to tune thresholds without touching processor logic

### High-Signal Comments

**Before (Low Signal):**

```rust
// Calculate median
sizes.sort();
sizes[sizes.len() / 2]
```

**After (High Signal):**

```rust
// Why median instead of mean?
// - Robust to outliers (large headings don't skew baseline)
// - Percentile-based approach matches human perception
// - Mean would be pulled up by every H2/H3 in document
sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
sizes[sizes.len() / 2]
```

**Comment Philosophy:**

- Explain WHY decisions were made (design rationale)
- NOT what the code does (code should be self-documenting)
- Include alternatives considered and why rejected
- Reference first principles when relevant

## Risk Mitigation

### Potential Issues

1. **Interface Changes**: New modules may not perfectly match inline methods
2. **Performance Overhead**: Function call delegation vs inline code
3. **Test Coverage**: New modules need comprehensive unit tests
4. **Integration Bugs**: Subtle behavior differences in refactored code

### Mitigation Strategies

1. **Exact Functional Equivalence**: Copy logic verbatim first, optimize later
2. **Compiler Optimization**: Rust inlines small functions, no runtime cost
3. **Comprehensive Testing**: Unit tests for new modules + existing integration tests
4. **Incremental Migration**: One module at a time, verify at each step

## Next Steps

**DECIDE Phase**: Choose implementation approach

- Sequential extraction: FontAnalyzer first, then HeadingClassifier
- Update SectionPatternProcessor to use new modules
- Add unit tests for new modules
- Verify all integration tests still pass
