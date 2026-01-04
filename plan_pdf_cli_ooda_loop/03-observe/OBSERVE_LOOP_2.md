# OBSERVE Phase - Loop 2 (Post-Fix Validation)

## Status After Fixes FIX-001 to FIX-004

### Previous Issues vs Current

| Issue     | Description               | Status  | Notes                                |
| --------- | ------------------------- | ------- | ------------------------------------ |
| ISSUE-001 | H1→H4, H2→H5, H3→bold     | PARTIAL | Now H1→H2 (better), H3 still→H2      |
| ISSUE-002 | Tables not detected       | OPEN    | Still flattened to single line       |
| ISSUE-003 | Page number "1" at end    | FIXED ✓ | No longer appears in output          |
| ISSUE-004 | Paragraph boundaries lost | OPEN    | Still merging paragraphs             |
| ISSUE-005 | Bullet • not converted    | FIXED ✓ | Now using `-` correctly              |
| ISSUE-006 | Code block language lost  | OPEN    | Still missing lang hints             |
| ISSUE-007 | Nested lists broken       | PARTIAL | Child items now bold `-`             |
| ISSUE-008 | Inline code extra space   | OPEN    | Still `print()` instead of `print()` |
| ISSUE-009 | Numbered lists as H2      | OPEN    | Still `## 1. First...`               |

## New Issues Identified

### ISSUE-010: Code block indentation lost

- **Severity**: MEDIUM
- **Symptom**: Python code loses indentation
- **Example**: `print("Hello")` at column 0 instead of indented

### ISSUE-011: Inline code opening backtick placement

- **Severity**: MEDIUM
- **Symptom**: Space between backtick and content
- **Example**: `` `print()`` `instead of` `print()` ``

## Remaining Critical Issues

### CRITICAL-1: Heading Level Offset

- H1 (title) rendered as H2
- H2 (sections) rendered as H2 (correct)
- H3 (subsections) rendered as H2 (wrong - should be H3)

### CRITICAL-2: Tables Still Broken

- No markdown table syntax generated
- All table content on single line
- No column/row detection from borderless PDF tables

### CRITICAL-3: Paragraph Merging

- Multi-paragraph sections collapsed into single paragraph
- Blank line separation not preserved

### CRITICAL-4: Numbered Lists

- Pattern `1.` detected as section heading instead of list
- Outputs `## 1. First numbered item`

## Files Compared

```
01_simple_text.md:    3 issues remaining
02_formatted_text.md: 4 issues remaining
03_lists.md:          5 issues remaining (nested + numbered)
04_tables.md:         1 issue remaining (tables)
05_code_blocks.md:    4 issues remaining (code blocks)
06_multi_paragraph.md: 2 issues remaining (paragraphs)
```

## Action Required

Need Loop 2 fixes for:

1. **FIX-005**: Heading level calibration (H1 should stay H1)
2. **FIX-006**: Numbered list detection vs heading detection
3. **FIX-007**: Code block language preservation
4. **FIX-008**: Paragraph boundary detection
5. **FIX-009**: Text-based table detection
