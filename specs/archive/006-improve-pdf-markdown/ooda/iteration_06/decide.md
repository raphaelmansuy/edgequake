# Iteration 06: Decide

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Decision: Document Kerning Overlap Tolerance

Add WHY comment explaining the 0.3 \* avg_char_width overlap tolerance.

### Rationale

1. **Completes documentation** - All magic numbers now explained
2. **Prevents bugs** - Developers won't "fix" working tolerance
3. **Quick win** - Small change, high value
