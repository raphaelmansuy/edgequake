# 022_corrupted_xref_table.pdf

**Purpose**: Test extraction from PDF with a corrupted cross-reference table (XRef).
**Content**: Simple text, but XRef table is intentionally broken.
**Expected**: Extraction should fail gracefully or recover partial text; no crash.
