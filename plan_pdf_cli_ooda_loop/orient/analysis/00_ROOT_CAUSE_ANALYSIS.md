# ORIENT Phase: Root Cause Analysis & Hypotheses

**Analysis Date:** 2026-01-04  
**Scope:** Deep code inspection following OBSERVE phase findings  
**Method:** First-principles thinking, code tracing, hypothesis formulation

---

## Critical Finding: TableDetectionProcessor is DISABLED

### Location

```rust
// edgequake/crates/edgequake-pdf/src/extractor.rs:324
.add(StyleDetectionProcessor::new())
// .add(TableDetectionProcessor::new()) // DISABLED - causing malformed output
.add(HeaderDetectionProcessor::new())
```

### Root Cause

**TableDetectionProcessor was intentionally disabled** due to "malformed output" issues.

### Impact Chain

```
PDF → SOTA Backend → Document IR → Processor Chain → Markdown Renderer
                                           ↓
                              TableDetectionProcessor: DISABLED
                              TextTableReconstructionProcessor: ENABLED
                                           ↓
                              No BlockType::Table created
                                           ↓
                              Fallback to plain text rendering
                                           ↓
                              Table structure completely lost
```

### Evidence

1. `render_table_from_children()` exists and works correctly (markdown.rs:413-480)
2. Table markdown syntax generation is proper: `| cell | cell |`
3. The table renderer REQUIRES `block.children` to be populated
4. `TableDetectionProcessor` creates `BlockType::Table` with `TableCell` children
5. `TextTableReconstructionProcessor` alone cannot reconstruct complex tables

### Hypothesis 1: TableDetectionProcessor had bugs, was disabled, never re-enabled

**Confidence: HIGH**

**Reasoning:**

- Comment says "causing malformed output"
- Processor code looks reasonable (table_detection.rs)
- No recent attempts to fix and re-enable
- TextTableReconstructionProcessor was added as workaround

**Test Strategy:**

1. Re-enable TableDetectionProcessor
2. Test on synthetic tables
3. Identify specific malformed output patterns
4. Fix bugs in detector logic
5. Validate fixes

---

## Issue 2: List Detection Loses Structure

### Location

```rust
// edgequake/crates/edgequake-pdf/src/processors/structure_detection.rs
// ListDetectionProcessor
```

### Observed Behavior

```markdown
# Original

- Level 1
  - Level 2
    - Level 3

# Converted

•Level 1
**-Level 2**
∗Level 3
```

### Root Cause Analysis

**Symptoms:**

1. List items merged onto single lines
2. Indentation/nesting lost
3. Multiple bullet symbols (`•`, `**-**`, `∗`)
4. No proper markdown list syntax

**Code Investigation Needed:**

1. How does ListDetectionProcessor identify lists?
2. How is indentation/level metadata stored?
3. How does markdown renderer use level metadata?
4. Why are bullets getting mixed up?

### Hypothesis 2A: List level detection works, but markdown renderer loses it

**Confidence: MEDIUM**

**Evidence Needed:**

- Check Block metadata for "level" or "indent" fields
- Trace from ListDetectionProcessor → Block → MarkdownRenderer
- Verify renderer respects indentation

### Hypothesis 2B: List detection conflates different list types

**Confidence: HIGH**

**Evidence:**

- Different bullet symbols suggest mixed detection
- `•` = unordered
- `-` = unordered (markdown)
- `∗` = nested marker?
- Bold wrapped `-` suggests style confusion

**Test Strategy:**

1. Enable debug logging for ListDetectionProcessor
2. Inspect Block metadata for detected lists
3. Check if level/indent is preserved
4. Verify markdown renderer indentation logic

---

## Issue 3: Font Styling Lost (Bold/Italic)

### Observed Behavior

```markdown
# Original

**bold** _italic_ `code`

# Converted

bold italic code
```

### Root Cause Analysis

**Hypothesis 3A: Font properties not mapped to markdown**
**Confidence: VERY HIGH**

**Reasoning:**

- PDF fonts have properties: weight, style, family
- Bold = font-weight > 600 or font-name contains "Bold"
- Italic = font-style = italic or font-name contains "Italic"
- Code = monospace font family

**Evidence Needed:**

1. Check if SOTA backend extracts font properties
2. Check if processors analyze font weights
3. Check if Block/Span stores style metadata
4. Check if markdown renderer applies \*_ and _ markers

### Code Locations to Inspect

1. `edgequake/crates/edgequake-pdf/src/backend/sota_backend.rs` - font extraction
2. `edgequake/crates/edgequake-pdf/src/processors/processor.rs` - StyleDetectionProcessor
3. `edgequake/crates/edgequake-pdf/src/renderers/markdown.rs` - render_spans_styled()

### Hypothesis 3B: Inline code requires special handling

**Confidence: HIGH**

**Reasoning:**

- Inline code in markdown: \`code\`
- Requires identifying code spans vs bold/italic
- Monospace font detection needed

---

## Issue 4: Heading Hierarchy Collapsed (H4+)

### Observed Behavior

```markdown
# Original

#### Level 4 Heading

# Converted

**Level 4 Heading**
```

### Root Cause Analysis

**Hypothesis 4A: Font size thresholds too strict**
**Confidence: VERY HIGH**

**Evidence:**

- H4+ converted to bold, not headings
- Suggests font-size-based classification
- Threshold may only detect H1-H3

**Code Location:**

```rust
// edgequake/crates/edgequake-pdf/src/processors/processor.rs
impl StyleDetectionProcessor {
    fn detect_headers(&self, block: &mut Block) {
        // Font size thresholds for heading detection
    }
}
```

**Investigation:**

1. What are current font size thresholds?
2. What is typical H4/H5/H6 font size in PDFs?
3. Is there adaptive threshold logic?

### Hypothesis 4B: Heading detection uses wrong algorithm

**Confidence: MEDIUM**

**Alternative approach:**

- Relative font size (larger than body text)
- Position in document (start of section)
- Text length (short phrases)
- Capitalization patterns

---

## Issue 5: Unicode Corruption

### Observed Behavior

```
α β γ → ￿ ￿ ￿
∀ ∃ ∈ → ￿ ￿ ￿
😀 🎉 → ￿ ￿
```

### Root Cause Analysis

**Hypothesis 5A: CMap/ToUnicode table decoding failure**
**Confidence: VERY HIGH**

**Reasoning:**

- `￿` (U+FFFD) = replacement character
- Indicates character mapping failure
- PDF embeds fonts with custom encodings
- ToUnicode CMap converts glyph IDs → Unicode

**Code Location:**

```rust
// edgequake/crates/edgequake-pdf/src/backend/encodings.rs
// CMap decoding logic
```

**Investigation:**

1. Is ToUnicode CMap being read?
2. Is fallback encoding being used?
3. Are multi-byte characters handled?
4. Is UTF-8 encoding enforced at output?

### Hypothesis 5B: Font doesn't have glyphs

**Confidence: LOW**

**Counter-evidence:**

- Some symbols work (√ ∞ ± × ÷)
- Selective failure suggests encoding issue, not missing glyphs

---

## Issue 6: Line Breaking and Hyphenation

### Observed Behavior

```markdown
# Original

...deserunt mollit anim id est laborum.

# Converted

...officia de-
```

### Root Cause Analysis

**Hypothesis 6A: PDF hyphenation preserved in output**
**Confidence: VERY HIGH**

**Reasoning:**

- PDFs break words with hyphens for justification
- Text extraction preserves line breaks
- Hyphen removal logic either missing or broken

**Code Location:**

```rust
// edgequake/crates/edgequake-pdf/src/processors/text_cleanup.rs
impl HyphenContinuationProcessor {
    fn process(&self, document: Document) -> Result<Document> {
        // Should merge "de-\nserunt" → "deserunt"
    }
}
```

**Investigation:**

1. Is HyphenContinuationProcessor running?
2. What are matching rules?
3. Are there edge cases (legitimate hyphens)?

### Hypothesis 6B: Line length limits in extraction

**Confidence: MEDIUM**

**Evidence:**

- Lines truncated at consistent length
- Might be buffer limit in SOTA backend

---

## Issue 7: Whitespace Handling

### Observed Behavior

```markdown
# Original

Multiple spaces between words.

# Converted

Multiple spaces between words.
```

### Root Cause Analysis

**Hypothesis 7A: Whitespace normalization too aggressive**
**Confidence: HIGH**

**Reasoning:**

- Multiple spaces collapsed to single
- Tabs converted to spaces
- Might be in `clean_text()` method

**Code Location:**

```rust
// edgequake/crates/edgequake-pdf/src/renderers/markdown.rs
fn clean_text(&self, text: &str) -> String {
    // Likely normalizes whitespace here
}
```

---

## Priority Matrix

| Issue                        | Severity | Complexity | Impact | Priority           |
| ---------------------------- | -------- | ---------- | ------ | ------------------ |
| **Table Detection Disabled** | CRITICAL | MEDIUM     | 100%   | **P0 - Fix First** |
| **List Structure Lost**      | CRITICAL | HIGH       | 90%    | **P0**             |
| **Unicode Corruption**       | HIGH     | HIGH       | 70%    | **P1**             |
| **Heading Hierarchy**        | HIGH     | LOW        | 60%    | **P1**             |
| **Bold/Italic Lost**         | HIGH     | MEDIUM     | 50%    | **P1**             |
| **Hyphenation**              | MEDIUM   | LOW        | 20%    | **P2**             |
| **Whitespace**               | LOW      | LOW        | 10%    | **P3**             |

---

## Recommended Action Plan (DECIDE Phase Input)

### Phase 1: Quick Wins

1. **Re-enable TableDetectionProcessor** (1-2 hours)

   - Test on synthetic data
   - Fix specific bugs if found
   - Validate table markdown output

2. **Fix Heading Thresholds** (30 min)
   - Adjust font size ranges for H4-H6
   - Test heading hierarchy preservation

### Phase 2: Structural Fixes

3. **Fix List Indentation** (2-4 hours)

   - Debug list level detection
   - Ensure metadata preserved
   - Fix markdown indentation logic

4. **Implement Font Style Detection** (2-3 hours)
   - Extract font-weight/style
   - Map to bold/italic markdown
   - Handle inline code

### Phase 3: Character Encoding

5. **Fix Unicode Mapping** (4-6 hours)
   - Debug CMap decoding
   - Add fallback encodings
   - Test with full Unicode range

### Phase 4: Polish

6. **Fix Hyphenation** (1 hour)
7. **Adjust Whitespace** (30 min)

---

## Next Steps

1. Create DECIDE phase document with specific implementation plan
2. Prioritize fixes based on impact/effort
3. Create test cases for each issue
4. Implement fixes iteratively
5. Validate with round-trip testing

---

**Status:** ORIENT phase complete → Moving to DECIDE phase
