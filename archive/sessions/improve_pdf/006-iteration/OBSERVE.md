# OBSERVE.md - Iteration 006

**Directory:** `edgequake/crates/edgequake-pdf/src/processors`

**Timestamp:** 2026-01-02

## Target: Eliminate SECTION_KEYWORDS Heuristic

### Current State (After Loop 005)

- ✅ 111 tests passing
- Lattice engine enabled for table detection
- Composite Score: ~32.5/100 (estimated, need validator run)

### Code Smell: SECTION_KEYWORDS Constant

**Location:** `processor.rs:13-63`

```rust
const SECTION_KEYWORDS: &[&str] = &[
    "abstract",
    "introduction",
    "background",
    "related work",
    // ... 60+ more keywords
];
```

**Usage Sites:**

1. `SectionNumberMergeProcessor::starts_with_section_keyword()` (line 127)
2. `HeaderDetectionProcessor` single number pattern validation (line 2279-2290)

### Why This is Wrong (Violations of First Principles)

1. **Language Dependency:** Only works for English papers
2. **Domain Specificity:** Academic keywords don't work for technical docs, manuals, reports
3. **Brittleness:** New domains require code changes
4. **Completeness:** Impossible to enumerate all possible section names
5. **False Positives:** Common words like "model", "system" are too generic
6. **Maintenance Burden:** 60+ keyword list needs constant updates

### What First Principles Says

**Section headers are characterized by:**

1. **Font Properties:** Larger size, bold weight (already detected by HeaderDetectionProcessor)
2. **Hierarchical Structure:** Numbering patterns (1., 1.1., 1.1.1.)
3. **Position:** Start of logical blocks, spatial separation
4. **Consistency:** Same styling within same hierarchy level

**NOT by:**

- Text content matching keyword lists
- Language-specific patterns
- Domain-specific vocabulary

### Current Font-Based Detection

`HeaderDetectionProcessor` (lines 2630-2740) already implements proper detection:

```rust
// H1: Very large (> 1.6x body_size)
// H2: Large (> 1.4x body_size)
// H3: Moderately larger (> 1.25x body_size)
```

It also handles numbered patterns:

```rust
// Subsection: "1.1 Motivation" -> H3
// Single number: "1. Introduction" -> H2
```

But then **adds keyword validation** for single numbers (line 2279):

```rust
let is_section_keyword = SECTION_KEYWORDS
    .iter()
    .any(|kw| after_lower.starts_with(kw));
```

### The Problem

This hybrid approach means:

- "1. Introduction" → H2 (keyword match) ✅
- "1. Executive Overview" → Text (no keyword match) ❌

But both should be H2 based on:

1. Numbered pattern "1."
2. Font size (if larger than body)
3. Position/structure

### Solution Plan

1. **Remove SECTION_KEYWORDS constant** entirely
2. **Strengthen numerical pattern detection:**
   - "1." followed by capitalized text → likely H2
   - "1.1" followed by text → likely H3
   - Pattern + font size confirmation
3. **Keep font-based detection** as primary signal
4. **Remove keyword checks** from HeaderDetectionProcessor

### Files to Modify

1. `processor.rs` - Remove SECTION_KEYWORDS constant and all uses
2. Remove `starts_with_section_keyword()` method from SectionNumberMergeProcessor
3. Update HeaderDetectionProcessor to trust font + numbering patterns

### Expected Impact

- **Style Accuracy:** May initially dip slightly (fewer false positives from keywords)
- **Robustness:** +10-20 points (works on non-English, non-academic docs)
- **Maintainability:** Cleaner code, no keyword list to maintain
- **Correctness:** More principled detection based on PDF properties
