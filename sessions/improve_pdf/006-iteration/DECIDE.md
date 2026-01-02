# DECIDE.md - Iteration 006

**Directory:** `edgequake/crates/edgequake-pdf/src/processors`

**Timestamp:** 2026-01-02

## Decision: Remove SECTION_KEYWORDS, Use Multi-Signal Detection

### Patch Summary

Remove all keyword-based section detection and replace with principled multi-signal approach combining:

1. Structural patterns (numbering)
2. Font properties (size, weight)
3. Content properties (length, capitalization)

### Changes

#### Change 1: Remove SECTION_KEYWORDS Constant

**File:** `processor.rs`
**Lines:** 13-63

**Before:**

```rust
const SECTION_KEYWORDS: &[&str] = &[
    "abstract",
    "introduction",
    // ... 60+ keywords
];
```

**After:**

```rust
// REMOVED - First principles: detect sections by font size and structure, not keywords
```

#### Change 2: Remove Keyword Check Method

**File:** `processor.rs`
**Lines:** 123-128 (in SectionNumberMergeProcessor)

**Before:**

```rust
fn starts_with_section_keyword(text: &str) -> bool {
    let lower = text.to_lowercase();
    let trimmed = lower.trim();
    SECTION_KEYWORDS.iter().any(|kw| trimmed.starts_with(kw))
}
```

**After:**

```rust
// REMOVED - replaced with structural pattern detection
```

#### Change 3: Refactor Section Number Merge Logic

**File:** `processor.rs`
**Lines:** ~169 (in SectionNumberMergeProcessor)

**Before:**

```rust
if Self::starts_with_section_keyword(title_text) {
    // Merge only if keyword matches
}
```

**After:**

```rust
// Merge if title block looks like a section title (capitalized, short, on same line)
if Self::looks_like_section_title(title_text) {
    // Merge
}

// New helper method
fn looks_like_section_title(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() > 100 {
        return false;
    }
    // Check if first character is uppercase (title case)
    trimmed.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
}
```

#### Change 4: Strengthen HeaderDetectionProcessor

**File:** `processor.rs`
**Lines:** 2270-2295 (in HeaderDetectionProcessor)

**Before:**

```rust
// For single number patterns, require section keyword
let is_section_keyword = SECTION_KEYWORDS
    .iter()
    .any(|kw| after_lower.starts_with(kw));

if is_section_keyword {
    block.block_type = BlockType::SectionHeader;
    block.level = Some(2);
}
```

**After:**

```rust
// For single number patterns, use multi-signal detection
fn is_likely_section(text: &str, span: &TextSpan, body_size: f32) -> bool {
    // Check multiple independent signals
    let is_short = text.len() < 100;
    let is_capitalized = text.trim()
        .split_whitespace()
        .skip(1) // Skip the number
        .next()
        .map(|word| word.chars().next().unwrap().is_uppercase())
        .unwrap_or(false);

    let size = span.style.size.unwrap_or(body_size);
    let weight = span.style.weight.unwrap_or(400);
    let is_bold = weight >= 600;
    let is_larger = size > body_size * 1.15;

    // Decision logic: need strong confirmation
    // EITHER font evidence (size OR bold) AND structural evidence (short + capitalized)
    // OR very strong font evidence (size AND bold)
    (is_larger || is_bold) && is_short && is_capitalized
        || (is_larger && is_bold)
}

// Single number sections like "1 Introduction" are H2
if is_likely_section(&text, span, body_size) {
    block.block_type = BlockType::SectionHeader;
    block.level = Some(2);
}
```

### Acceptance Criteria

1. ✅ No SECTION_KEYWORDS references remain
2. ✅ `cargo test -p edgequake-pdf` passes
3. ✅ `cargo clippy` shows no warnings about unused constants
4. ✅ Validator shows no regression in Style Accuracy
5. ✅ Works on non-English test documents (if available)

### Expected Metrics

- **Style Accuracy:** 30-35% (maintain or slight improvement)
- **Robustness:** 100% (no change)
- **Maintainability:** +++ (removes 60+ line keyword list)
- **Correctness:** +++ (works on any language/domain)

### Testing Strategy

1. Run existing tests (should all pass)
2. Check test*section_merge*\* tests still work
3. Verify header detection on AlphaEvolve.pdf (numbered sections)
4. Check that list items aren't mistaken for headers

### Why This is Better

**Before (Keyword):**

- "1. Introduction" → Header ✅
- "1. Executive Summary" → Text ❌ (keyword missing)
- "1. Introducción" (Spanish) → Text ❌ (English only)
- "1. First, explore..." → Text ✅ (correct rejection)

**After (Multi-Signal):**

- "1. Introduction" → Header ✅ (bold, short, capitalized)
- "1. Executive Summary" → Header ✅ (bold, short, capitalized)
- "1. Introducción" → Header ✅ (bold, short, capitalized)
- "1. First, explore..." → Text ✅ (not capitalized, not bold)

### Implementation Notes

- Keep subsection patterns (1.1, 1.1.1) - these are ALWAYS sections
- For single numbers, use multi-signal confidence scoring
- Font size is primary signal, other properties are confirmatory
- This approach is composable: can add more signals later (spacing, consistency)
