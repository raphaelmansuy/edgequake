# 023_incomplete_unicode_mapping.pdf

**Purpose**: Test extraction from PDF with missing Unicode mapping for some glyphs.
**Content**: Text with some glyphs mapped to (cid:x) values.
**Expected**: Extracted text should show (cid:x) for unmapped glyphs; rest should be normal.
