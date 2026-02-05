# OODA Iteration 14 - Observe

## Observation Date

2025-02-05

## Quality Issue Detected

### Issue: TOC Leader Dots Not Cleaned Up

**Symptom:**
In PDF documents with Table of Contents, leader dots (`.....`) are preserved in output:

```markdown
5.1 - Actions ................................
5.2 - Operations ................................
**.............. 3**
**............. 5**
```

**Test Document:** Apple-Sandbox-Guide-v1.0.pdf

**Expected Output:**
Either clean removal of dots, or formatted as proper TOC:

```markdown
5.1 Actions
5.2 Operations
```

Or simply omit TOC leader patterns entirely since the page numbers are meaningless in markdown.

### Root Cause Analysis

1. **Leader dots extraction:** PDFs use leader dots (also called "dot leaders") as visual connectors between TOC entries and page numbers
2. **No cleanup:** The `cleanup_markdown_artifacts()` function doesn't handle dots patterns
3. **Text extraction:** Dots are extracted as regular text characters

### Evidence

From `/tmp/it14_apple.md` lines 28-68:

```markdown
5.1 - Actions ................................
5.2 - Operations ................................
5.3 - Filters ................................
5.4 - Modifiers................................
5.5 Other keywords
**6 - Special hardcoded cases**
**.............. 3**
**............. 5**
31
35
............... 36
**................. 37**
**..... 43**
**.......... 43**
```

### Impact Assessment

| Category      | Impact                    |
| ------------- | ------------------------- |
| Readability   | Medium - cluttered output |
| Semantic      | Low - no data loss        |
| Quality Score | -5 points for formatting  |

## Metrics

- Lines with leader dots: 16
- False positive code blocks: 0 (previous fix working)
- Tables detected: 0 in this document (no tables)

## Next Phase: Orient
