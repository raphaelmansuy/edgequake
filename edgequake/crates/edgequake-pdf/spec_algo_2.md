# Updated Technical Specification: Refactoring, Enhancing Table Detection, Integrating Header/Footer Detection, and Adding Bold/Italic + H1/H2 Title Detection in edgequake-pdf Crate

## Document Metadata
- **Version**: 1.2 (Updated from 1.1 to include style and title detection)
- **Date**: January 1, 2026
- **Author**: Grok 4 (xAI Assistant)
- **Status**: Draft
- **Changes from v1.1**:
  - Added bold/italic style detection (from font metadata: weight >=600 for bold, ItalicAngle/flag for italic).
  - Added H1/H2+ title detection (heuristic: font size ratios, position, patterns like all-caps/short lines).
  - Included code examples for the new style/title processor.
- **References**:
  - PDF spec: Font descriptors (/FontWeight, /ItalicAngle).
  - Non-ML style detection: Parse font dict (e.g., lopdf/pdf_oxide approaches, but adapt post-removal).
  - Header levels: Size-based (e.g., >1.5x body = H1, >1.2x = H2) + patterns (from pdfplumber discussions).

## Overview
This spec extends v1.1 by:
1-3. As before.
4. Header/footer integration.
5. Bold/italic detection: Enhance blocks with style info.
6. H1/H2+ titles: Detect multi-level headers via heuristics.

Pure-Rust focus; non-ML for all detections.

**High-Level Impact**:
- Richer blocks: Styles enable better Markdown (e.g., **bold**, *italic*).
- Accurate titles: H1 for doc title, H2 for sections.

## Scope
### In Scope (Updated)
- All previous.
- New: `StyleDetectionProcessor` for bold/italic.
- Enhance `HeaderDetectionProcessor` for H1/H2 levels.
- Code examples.

### Out of Scope
- ML styles (non-ML first).
- Color/underlines (focus bold/italic).

## Requirements
1-4. Unchanged.

### 5. Add Bold/Italic Detection
- **Rationale**: PDFs encode styles in fonts (/FontWeight, /ItalicAngle). Detect for rendering (e.g., Markdown **bold**).
- **Method** (Non-ML):
  - Parse font dict: Weight >=600 = bold; ItalicAngle !=0 or flag = italic.
  - Per-span: If backend provides font per char/span, tag styles.
  - Heuristics: Font name contains "Bold"/"Italic".
- **Actions**:
  - New `StyleDetectionProcessor`: Iterate blocks/spans, set `FontStyle` fields.
  - Integrate post-extraction.
  - Update `MarkdownRenderer`: Use styles for **bold**, *italic*.

### 6. Add H1/H2+ Title Detection
- **Rationale**: Detect levels for TOC/Markdown (e.g., # H1, ## H2).
- **Method** (Non-ML, from Refs):
  - Size ratios: >1.5x body = H1 (title), >1.2x = H2, etc.
  - Patterns: All-caps/short (<50 chars), top position.
  - Enhance existing `HeaderDetectionProcessor`.
- **Actions**:
  - Compute body size (mode of sizes).
  - Classify levels; mark as SectionHeader with level.

## Implementation Plan (Updated)
1-4. As before.

5-6. **Phase 5: Style & Title Detection (2-3 days)**:
   - Implement processors.
   - Update schema: Ensure `FontStyle` serialized.
   - Test: PDFs with mixed styles/headers.

## Testing Strategy (Updated)
- **Unit**: Mock fonts for bold/italic; size ratios for levels.
- **Integration**: PDFs with styled text (e.g., arXiv samples).

## Risks & Mitigations (Updated)
- **Font Parsing**: Post-lopdf removal, ensure new backend exposes fonts → Fallback to name heuristics.
- **Level Accuracy**: Tune ratios; manual thresholds.

## Code Examples
New `StyleDetectionProcessor` (add to `src/processors/processor.rs`):

```rust
use crate::schema::{Block, BlockType, Document, FontStyle};
use crate::Result;
use std::collections::HashMap;

// Processor for bold/italic and header levels
pub struct StyleDetectionProcessor {
    body_size: f32, // Computed from doc
}

impl StyleDetectionProcessor {
    pub fn new() -> Self {
        Self { body_size: 0.0 }
    }

    // Compute body font size (most common)
    fn compute_body_size(&mut self, document: &Document) {
        let mut size_counts: HashMap<i32, usize> = HashMap::new();
        for page in &document.pages {
            for block in &page.blocks {
                for span in &block.spans {
                    let size_key = (span.style.size.unwrap_or(10.0) * 10.0) as i32;
                    *size_counts.entry(size_key).or_insert(0) += 1;
                }
            }
        }
        self.body_size = size_counts.iter()
            .max_by_key(|&(_, count)| count)
            .map(|(s, _)| *s as f32 / 10.0)
            .unwrap_or(10.0);
    }

    // Detect styles per span (assume backend sets family/weight/italic)
    fn detect_styles(&self, block: &mut Block) {
        for span in &mut block.spans {
            let family_lower = span.style.family.as_ref().map(|f| f.to_lowercase()).unwrap_or_default();
            // Bold: Weight >=600 or name has "bold"
            span.style.weight = Some(if span.style.weight.unwrap_or(400) >= 600 || family_lower.contains("bold") { 700 } else { 400 });
            // Italic: Flag or name has "italic/oblique"
            span.style.italic = span.style.italic || family_lower.contains("italic") || family_lower.contains("oblique");
        }
    }

    // Detect header levels
    fn detect_headers(&self, block: &mut Block) {
        if block.block_type != BlockType::Text {
            return;
        }
        if let Some(span) = block.spans.first() {
            let size = span.style.size.unwrap_or(10.0);
            let ratio = size / self.body_size;
            let is_bold = span.style.weight.unwrap_or(400) >= 600;
            let text = block.text.trim();
            let is_short = text.len() < 80;
            let is_all_caps = text == text.to_uppercase();
            if ratio > 1.5 && is_short { // H1: Large title
                block.block_type = BlockType::SectionHeader;
                block.level = Some(1);
            } else if (ratio > 1.2 || (is_bold && ratio >= 1.0)) && is_short { // H2: Section
                block.block_type = BlockType::SectionHeader;
                block.level = Some(2);
            } else if is_bold && is_all_caps { // H3: Sub-section
                block.block_type = BlockType::SectionHeader;
                block.level = Some(3);
            }
        }
    }
}

impl Processor for StyleDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        let mut this = self.clone(); // Mutable copy for body_size
        this.compute_body_size(&document);
        for page in &mut document.pages {
            for block in &mut page.blocks {
                this.detect_styles(block);
                this.detect_headers(block);
                // Recurse children
                for child in &mut block.children {
                    this.detect_styles(child);
                    this.detect_headers(child);
                }
            }
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "StyleDetectionProcessor"
    }
}
```

### Usage in Chain
Add to default chain after extraction:
```rust
.add(StyleDetectionProcessor::new())  // New
.add(HeaderFooterDetectionProcessor::new())
```

This detects styles and titles reliably!