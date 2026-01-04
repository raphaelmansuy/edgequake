# OBSERVE Phase: Issues Identified from PDF Conversion Round-Trip Testing

**Test Date:** 2026-01-04  
**Test Suite:** 6 markdown documents → PDF → markdown conversion  
**Conversion Chain:** Markdown → Pandoc (XeLaTeX) → PDF → EdgeQuake PDF CLI → Markdown

---

## Critical Issues (High Priority)

### 1. **Table Structure Completely Lost**

**Severity:** CRITICAL  
**Affected Files:** 03_tables.md

**Observation:**

- Markdown table syntax (`| Header | Header |`) is completely missing
- Table borders (`|------|---------|`) are removed
- Cell content is concatenated into plain text without structure
- Table alignment is lost
- Tables become unreadable text blobs

**Example:**

```markdown
# Original

| Name  | Age | City     |
| ----- | --- | -------- |
| Alice | 30  | New York |

# Converted

Name Age City
Alice 30 New York
```

**Impact:** Tables are one of the most critical document structures. Complete loss makes documents unusable for structured data.

---

### 2. **List Formatting Destroyed**

**Severity:** CRITICAL  
**Affected Files:** 02_lists_and_formatting.md

**Observation:**

- List items merged into single lines with bullet symbols
- Indentation/nesting lost completely
- `-` bullets converted to `•` or `**-**` or `∗`
- Multi-level lists flattened
- Ordered lists sometimes preserved, sometimes broken

**Example:**

```markdown
# Original

- Level 1 item A
  - Level 2 item A1
  - Level 2 item A2
    - Level 3 item A2a

# Converted

•Level 1 item A
**-Level 2 item A1** **-Level 2 item A2**
∗Level 3 item A2a
```

**Impact:** Nested lists are critical for technical documents, instructions, and hierarchical data.

---

### 3. **Text Formatting Lost (Bold, Italic, Code)**

**Severity:** HIGH  
**Affected Files:** 02_lists_and_formatting.md

**Observation:**

- Bold markers (`**text**`) removed, text appears without markers
- Italic markers (`*text*`) removed
- Inline code (`` `code` ``) removed
- Blockquotes lost (merged into regular text)
- Code blocks lost their fence markers

**Example:**

```markdown
# Original

This is **bold text** and this is _italic text_.
This is `inline code`.

# Converted

This is bold textand this is italic text.
inline code
```

**Impact:** Technical documentation relies heavily on formatting to distinguish code from prose.

---

### 4. **Heading Hierarchy Corruption**

**Severity:** HIGH  
**Affected Files:** 04_heading_hierarchy.md

**Observation:**

- Level 4+ headings (`####`, `#####`, `######`) converted to bold text
- Multiple H1 headings merged together
- Heading levels incorrectly detected or demoted
- Document structure flattened

**Example:**

```markdown
# Original

#### Level 4 Heading

This is content under a level 4 heading.

# Converted

**Level 4 Heading** This is content under a level 4 heading.
```

**Impact:** Document navigation and semantic structure is destroyed. Screen readers and document processors rely on heading hierarchy.

---

### 5. **Unicode Character Corruption**

**Severity:** HIGH  
**Affected Files:** 05_special_characters.md

**Observation:**

- Greek letters (α β γ δ) converted to replacement character (￿)
- Mathematical symbols (∀ ∃ ∈) converted to ￿
- Currency symbols (₹ ₽ ₪) converted to ￿
- Arrows (↔ ↕ ⇐ ⇒) converted to ￿
- Emojis (😀 🎉 ✅) converted to ￿
- Some symbols preserved (√ ∞ ± × ÷ $ € £ ¥)

**Impact:** Scientific, mathematical, and international documents become unreadable.

---

## Medium Priority Issues

### 6. **Line Breaking and Word Wrapping**

**Severity:** MEDIUM  
**Affected Files:** 01_basic_text.md

**Observation:**

- Long lines artificially broken mid-word
- Hyphenation preserved from PDF but shouldn't be in markdown
- "deserunt mollit" → "de-" (broken at line end)
- "aliquip ex ea" → "ut" (truncated)

**Example:**

```markdown
# Original

Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.

# Converted

Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut

# Original

Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

# Converted

Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia de-
```

**Impact:** Text readability reduced, searches may fail on broken words.

---

### 7. **Multi-line List Items Collapsed**

**Severity:** MEDIUM  
**Affected Files:** 01_basic_text.md, 02_lists_and_formatting.md

**Observation:**

- List items that span multiple lines in original are collapsed
- Continuation lines merged without proper spacing
- List structure ambiguous

**Example:**

```markdown
# Original

The purpose of this document is to verify that:

- Plain text is extracted correctly
- Paragraph breaks are preserved
- Basic formatting is maintained

# Converted

The purpose of this document is to verify that: - Plain text is extracted correctly

- Paragraph breaks are preserved - Basic formatting is maintained
```

**Impact:** Reading comprehension reduced, list semantics lost.

---

### 8. **Whitespace Handling Issues**

**Severity:** MEDIUM  
**Affected Files:** 05_special_characters.md

**Observation:**

- Multiple spaces collapsed to single space
- Tab characters converted to spaces
- Non-breaking spaces may or may not be preserved
- Line breaks lost (soft breaks vs hard breaks)

**Example:**

```markdown
# Original

Multiple spaces between words.
Tab separated values.

# Converted

Multiple spaces between words. Tab separated values.
```

---

## Low Priority Issues

### 9. **Horizontal Rules Misplaced**

**Severity:** LOW  
**Affected Files:** 02_lists_and_formatting.md, 05_special_characters.md

**Observation:**

- Horizontal rules (`---`) appear in unexpected locations
- May be artifacts of page breaks or section dividers
- Not present in original markdown

---

### 10. **Page Numbers Inserted**

**Severity:** LOW  
**Affected Files:** 01_basic_text.md

**Observation:**

- Page number "1" inserted at end of content
- Artifact from PDF page footer
- Should be filtered out

---

## Root Cause Hypotheses (to be validated in ORIENT phase)

1. **Table Detection:** Lattice/stream table detection fails to reconstruct markdown table syntax
2. **List Detection:** List processor doesn't maintain indentation/nesting structure
3. **Font Style:** Bold/italic detection based on font properties is failing or not translating to markdown
4. **Heading Detection:** Font size thresholds for H4+ headings may be wrong
5. **Unicode Mapping:** CMap/ToUnicode tables not properly decoded for special characters
6. **Line Reassembly:** Text blocks not properly reassembled across line breaks
7. **Processor Chain:** Processor ordering or interference causing cascading failures

---

## Quantitative Impact Summary

| File                    | Original Lines   | Converted Lines  | Structure Loss     | Content Loss      |
| ----------------------- | ---------------- | ---------------- | ------------------ | ----------------- |
| 01_basic_text           | 26               | 25               | 15% (lists)        | 5% (truncation)   |
| 02_lists_and_formatting | 65               | 72               | 90% (lists/format) | 20% (code blocks) |
| 03_tables               | 50               | 38               | 100% (all tables)  | 30% (structure)   |
| 04_heading_hierarchy    | 83               | 64               | 60% (H4+)          | 10% (merge)       |
| 05_special_characters   | 64               | 44               | 40% (sections)     | 70% (unicode)     |
| 06_multi_column         | Not yet analyzed | Not yet analyzed | TBD                | TBD               |

**Overall Success Rate:** ~30-40% (structural fidelity)  
**Text Content Preserved:** ~80% (but with formatting loss)  
**Semantic Structure Preserved:** ~20-30%

---

## Next Steps (ORIENT Phase)

1. Deep code inspection of table detection processors
2. Analysis of list detection algorithm
3. Font style to markdown translation logic review
4. Unicode encoding chain verification
5. Heading classification threshold analysis
6. Create test suite for each specific issue
7. Formulate fix strategies based on first principles

---

**Status:** OBSERVE phase complete → Moving to ORIENT phase
