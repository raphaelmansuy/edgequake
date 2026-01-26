# OBSERVE Phase: Issue Catalog

## Summary

**Tests Run:** 6 markdown documents
**Issues Found:** 9 distinct issues
**Critical Issues:** 4
**Medium Issues:** 3
**Minor Issues:** 2

---

## Critical Issues

### ISSUE-001: Heading Level Miscalculation

**Severity:** CRITICAL  
**Affected Tests:** All 6 documents

**Expected:**

```markdown
# Title (H1)

## Section (H2)

### Subsection (H3)
```

**Actual:**

```markdown
#### Title (H4)

##### Section (H5)

**Subsection** (Bold, not H3)
```

**Root Cause Hypothesis:**

- Heading detection is calculating levels incorrectly
- Font size thresholds are miscalibrated for pandoc-generated PDFs
- H1 and H2 are being classified as H4/H5

---

### ISSUE-002: Tables Not Detected

**Severity:** CRITICAL  
**Affected Tests:** 04_tables.md

**Expected:**

```markdown
| Name  | Age | City     |
| ----- | --- | -------- |
| Alice | 30  | New York |
```

**Actual:**

```markdown
Name Age City Alice 30 New York Bob 25 London Carol 35 Paris
```

**Root Cause Hypothesis:**

- Table grid detection failing (logs show "5 rows, 0 cols")
- LatticeEngine not finding column separators
- Text-based table reconstruction not triggering

---

### ISSUE-003: Spurious Page Number "1"

**Severity:** CRITICAL  
**Affected Tests:** All 6 documents

**Pattern:**
All converted files end with:

```markdown
...content...

1
```

**Root Cause Hypothesis:**

- Page number from PDF footer not being filtered
- Margin filter processor not removing page numbers
- Or page number is within content area bounds

---

### ISSUE-004: Paragraph Boundary Loss

**Severity:** CRITICAL  
**Affected Tests:** 06_multi_paragraph.md

**Expected:**

```markdown
Paragraph 1...

Paragraph 2...

Paragraph 3...
```

**Actual:**

```markdown
Paragraph 1... Paragraph 2...

Paragraph 3...
```

Multiple paragraphs merged into single blocks.

**Root Cause Hypothesis:**

- Y-gap threshold too large for paragraph detection
- BlockMergeProcessor over-aggressive merging
- Line spacing in pandoc PDFs different from expected

---

## Medium Issues

### ISSUE-005: List Formatting Incorrect

**Severity:** MEDIUM  
**Affected Tests:** 03_lists.md

**Expected:**

```markdown
- First item
- Second item
```

**Actual:**

```markdown
• First item
• Second item
```

Uses bullet character instead of markdown dash syntax.

---

### ISSUE-006: Code Block Language Lost

**Severity:** MEDIUM  
**Affected Tests:** 05_code_blocks.md

**Expected:**

```python
def hello():
    pass
```

**Actual:**

```
def hello():
pass
```

Language specifier lost and indentation stripped.

---

### ISSUE-007: Nested Lists Broken

**Severity:** MEDIUM  
**Affected Tests:** 03_lists.md

Nested list items rendered as bold dashes instead of indented bullets.

---

## Minor Issues

### ISSUE-008: Inline Code Extra Space

**Severity:** MINOR  
**Affected Tests:** 02_formatted_text.md, 05_code_blocks.md

**Expected:** `print()`  
**Actual:** ` print()` (space before text)

---

### ISSUE-009: Numbered Lists as H2 Headings

**Severity:** MINOR  
**Affected Tests:** 03_lists.md

**Expected:**

```markdown
1. First item
2. Second item
```

**Actual:**

```markdown
## 1. First numbered item

## 2. Second numbered item
```

---

## Priority Order for Fixes

1. **ISSUE-001** - Heading levels (affects all documents)
2. **ISSUE-003** - Page number filtering (affects all documents)
3. **ISSUE-002** - Table detection (critical feature)
4. **ISSUE-004** - Paragraph boundaries (readability)
5. **ISSUE-005** - List formatting
6. **ISSUE-009** - Numbered lists as headings
7. **ISSUE-006** - Code block language
8. **ISSUE-007** - Nested lists
9. **ISSUE-008** - Inline code spacing
