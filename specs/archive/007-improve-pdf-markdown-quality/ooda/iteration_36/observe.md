# IT36 — Observe: List Detection Failures

## Symptom

Bullet items in academic papers (LightRAG, etc.) rendered as raw `•` characters
in continuous paragraph text instead of proper markdown `- ` list items.

## Root Cause Analysis

Three independent failures in the pipeline:

### 1. TextGrouper merges bullet lines into one block

`can_add_line()` in `pymupdf_structs.rs` uses purely geometric criteria (Y proximity,
X overlap). Consecutive bullet lines like:

```
• General Aspect. We emphasize...
• Methodologies. To enable...
• Experimental Findings. Extensive...
```

Are vertically close (within 10pt block_gap) and horizontally overlapping, so they
get merged into one `Block` with all three items in one text blob.

### 2. `join_blocks_phase2` re-merges after splitting

Even if `can_add_line` is fixed, `join_blocks_phase2()` in `pymupdf_grouper.rs`
merges blocks with similar X boundaries and close Y gap (≤10pt). This ALSO
doesn't check for bullet markers, re-merging what was correctly split.

### 3. ListDetectionProcessor only accepts `BlockType::Text`

PdfiumBackend creates `BlockType::Paragraph` blocks, but `ListDetectionProcessor`
(structure_detection.rs line 671) only processes `BlockType::Text`. All Paragraph
blocks skip list detection entirely.

### 4. Bold formatting lost in list rendering

`render_list_item()` in `markdown.rs` uses `clean_text(after_prefix)` when
skipping the bullet prefix, which drops all span styling (bold/italic). The
spans carry the formatting info, not the raw text.

## Impact

- 27 bullet items in LightRAG paper rendered as plain paragraph text
- Lists score: effectively 0 for Pdfium-extracted documents
- Bold/italic inside list items completely lost
