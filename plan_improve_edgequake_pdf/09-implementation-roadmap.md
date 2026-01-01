# Implementation Roadmap: Eliminating Layout Duplication

## Overview

This roadmap details the precise steps to refactor `PdfiumBackend` to remove duplicate layout analysis and achieve clean separation of concerns.

## Goal

**Remove layout analysis from PdfiumBackend so that LayoutProcessor is the only component performing structural analysis.**

## Changes Required

### File: `edgequake/crates/edgequake-pdf/src/backend/pdfium.rs`

**Current Code (lines 420-440):**
```rust
// Process each page
for page_index in 0..pages_to_process {
    let pdfium_page = pdfium_doc.pages().get(page_index as u16)...;
    
    // Extract blocks with word boundary detection
    let blocks = self.extract_page_blocks(&pdfium_page, page_index + 1)?;

    // ❌ Apply layout analysis to each page
    let analyzer = LayoutAnalyzer::new();
    let layout = analyzer.analyze(
        &blocks,
        pdfium_page.width().value,
        pdfium_page.height().value,
    );

    let mut page = Page::new(...);
    page.blocks = blocks;
    page.columns = layout.columns;  // ❌ Backend sets columns

    // ❌ Sort blocks by reading order
    analyzer.sort_by_reading_order(&mut page.blocks, &page.columns);

    pages.push(page);
}
```

**Proposed Code:**
```rust
// Process each page
for page_index in 0..pages_to_process {
    let pdfium_page = pdfium_doc.pages().get(page_index as u16)...;
    
    // Extract blocks with word boundary detection
    let blocks = self.extract_page_blocks(&pdfium_page, page_index + 1)?;

    // ✅ Just create the page with raw blocks
    let page = Page {
        number: page_index + 1,
        width: pdfium_page.width().value,
        height: pdfium_page.height().value,
        blocks,
        columns: vec![],  // ✅ Empty - will be filled by LayoutProcessor
        margins: None,
    };

    pages.push(page);
}
```

**Lines to Remove:**
- Line 426-430: `let analyzer = LayoutAnalyzer::new();` and `analyzer.analyze(...)`
- Line 432-437: `page.columns = layout.columns;`
- Line 439-440: `analyzer.sort_by_reading_order(...)`

**Lines to Keep:**
- Line 422-424: Extract blocks (this is the backend's job!)
- Line 432-437: Create Page (but with empty columns)

**Import to Remove:**
- Line 18: `use crate::layout::LayoutAnalyzer;` (no longer needed)

## Step-by-Step Implementation

### Step 1: Backup Current State
```bash
cd edgequake/crates/edgequake-pdf
cp src/backend/pdfium.rs src/backend/pdfium.rs.before_refactor
```

### Step 2: Remove Layout Analysis Code

**Edit `src/backend/pdfium.rs`:**

1. Remove import (line 18):
```rust
- use crate::layout::LayoutAnalyzer;
```

2. Replace lines 420-445 with:
```rust
// Process each page
for page_index in 0..pages_to_process {
    let pdfium_page = pdfium_doc.pages().get(page_index as u16).map_err(|e| {
        PdfError::PdfParse(format!("Failed to get page {}: {:?}", page_index, e))
    })?;

    debug!(
        "Processing page {}/{} with Pdfium",
        page_index + 1,
        pages_to_process
    );

    // Extract blocks with word boundary detection
    let blocks = self.extract_page_blocks(&pdfium_page, page_index + 1)?;

    // Create page with unsorted blocks (layout analysis happens in LayoutProcessor)
    let page = Page {
        number: page_index + 1,
        width: pdfium_page.width().value,
        height: pdfium_page.height().value,
        blocks,
        columns: vec![],  // Will be populated by LayoutProcessor
        margins: None,
    };

    pages.push(page);
}
```

### Step 3: Verify Compilation
```bash
cargo check -p edgequake-pdf
```

**Expected:** No compilation errors (Page::new() might need to be replaced with struct literal)

### Step 4: Run Tests
```bash
cargo test -p edgequake-pdf
```

**Expected:** All tests should still pass because LayoutProcessor will now do the work

### Step 5: Verify Behavior
```bash
# Run the CLI to verify end-to-end
cargo run -p edgequake-pdf -- convert path/to/test.pdf
```

**Expected:** Output should be identical (layout still happens, just in the processor)

### Step 6: Add Regression Test

**Create new test in `tests/backend_test.rs`:**
```rust
#[tokio::test]
async fn test_backend_returns_unsorted_blocks() {
    // This test verifies that backends return raw, unsorted blocks
    // and that layout analysis is deferred to processors
    
    use edgequake_pdf::backend::mock::MockBackend;
    use edgequake_pdf::schema::{Document, Page, Block, BlockType, BoundingBox};
    
    // Create a document with blocks in wrong reading order
    let mut doc = Document::new();
    let mut page = Page::new(1, 600.0, 800.0);
    
    // Block B (should be second but is first)
    page.blocks.push(Block::new(
        BlockType::Text,
        BoundingBox::new(100.0, 200.0, 200.0, 220.0),
        "Block B".to_string(),
    ));
    
    // Block A (should be first but is second)
    page.blocks.push(Block::new(
        BlockType::Text,
        BoundingBox::new(100.0, 100.0, 200.0, 120.0),
        "Block A".to_string(),
    ));
    
    doc.pages.push(page);
    
    // Backend should return blocks AS-IS (unsorted)
    let backend = MockBackend::with_document(doc.clone());
    let result = backend.extract(&[]).await.unwrap();
    
    // Verify blocks are still in original (wrong) order
    assert_eq!(result.pages[0].blocks[0].text, "Block B");
    assert_eq!(result.pages[0].blocks[1].text, "Block A");
    
    // Verify columns are empty (not analyzed yet)
    assert!(result.pages[0].columns.is_empty());
}

#[tokio::test]
async fn test_layout_processor_sorts_blocks() {
    // This test verifies that LayoutProcessor correctly sorts blocks
    
    use edgequake_pdf::processors::{LayoutProcessor, Processor};
    use edgequake_pdf::schema::{Document, Page, Block, BlockType, BoundingBox};
    
    // Create document with unsorted blocks
    let mut doc = Document::new();
    let mut page = Page {
        number: 1,
        width: 600.0,
        height: 800.0,
        blocks: vec![
            Block::new(
                BlockType::Text,
                BoundingBox::new(100.0, 200.0, 200.0, 220.0),
                "Block B".to_string(),
            ),
            Block::new(
                BlockType::Text,
                BoundingBox::new(100.0, 100.0, 200.0, 120.0),
                "Block A".to_string(),
            ),
        ],
        columns: vec![],
        margins: None,
    };
    doc.pages.push(page);
    
    // Process with LayoutProcessor
    let processor = LayoutProcessor::new();
    let result = processor.process(doc).unwrap();
    
    // Verify blocks are now sorted (A before B)
    assert_eq!(result.pages[0].blocks[0].text, "Block A");
    assert_eq!(result.pages[0].blocks[1].text, "Block B");
}
```

### Step 7: Update Documentation

**Edit `src/backend/mod.rs` trait documentation:**
```rust
/// PDF extraction backend.
///
/// Implementors are responsible for extracting raw content from PDF bytes
/// and returning an unprocessed Document. Layout analysis (column detection,
/// reading order) is performed by LayoutProcessor, not by backends.
///
/// # Contract
///
/// - `extract()` must return blocks with valid bounding boxes
/// - Blocks should NOT be sorted (reading order determined by LayoutProcessor)
/// - `page.columns` should be empty (populated by LayoutProcessor)
/// - Font styles and text spans should be preserved
#[async_trait]
pub trait PdfBackend: Send + Sync {
    /// Extract raw, unprocessed document from PDF bytes.
    ///
    /// Returns a Document with unsorted blocks and no layout analysis.
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document>;
    
    /// Get PDF metadata without full extraction.
    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo>;
}
```

### Step 8: Performance Verification

**Benchmark before/after:**
```bash
# Before refactor
cargo build --release -p edgequake-pdf
time target/release/edgequake-pdf convert test.pdf

# After refactor
cargo build --release -p edgequake-pdf
time target/release/edgequake-pdf convert test.pdf
```

**Expected:** Faster or same speed (layout analysis now runs once, not twice)

## Verification Checklist

- [ ] Code compiles without warnings
- [ ] All existing tests pass
- [ ] New regression tests added
- [ ] Backend returns unsorted blocks
- [ ] LayoutProcessor sorts blocks correctly
- [ ] Integration tests produce same output
- [ ] CLI works end-to-end
- [ ] Performance is same or better
- [ ] Documentation updated
- [ ] Code review completed

## Rollback Plan

If issues arise:
```bash
cd edgequake/crates/edgequake-pdf
git checkout src/backend/pdfium.rs
# or
cp src/backend/pdfium.rs.before_refactor src/backend/pdfium.rs
```

## Expected Outcomes

### Code Quality
- **Before:** 494 lines in pdfium.rs
- **After:** ~450 lines (removed ~40 lines)
- **Complexity:** Reduced (single responsibility)

### Performance
- **Before:** Layout analysis runs twice (backend + processor)
- **After:** Layout analysis runs once (processor only)
- **Expected:** 10-20% faster on large documents

### Maintainability
- **Before:** Layout logic in two places
- **After:** Layout logic in one place (LayoutProcessor)
- **Testing:** Easier to test backend and layout independently

### Consistency
- **Before:** PdfiumBackend ≠ MockBackend behavior
- **After:** All backends return raw, unsorted blocks
- **Abstraction:** Clean, predictable contract

## Timeline

- **Step 1-2 (Code Changes):** 30 minutes
- **Step 3-5 (Testing):** 1 hour
- **Step 6 (New Tests):** 1 hour
- **Step 7-8 (Documentation):** 30 minutes
- **Total:** ~3 hours

## Success Criteria

✅ **All tests pass**
✅ **No performance regression**
✅ **Cleaner separation of concerns**
✅ **Consistent backend behavior**
✅ **Easier to maintain**

## Next Steps After Implementation

1. Consider adding more backends (PyMuPDF, Poppler)
2. Make LayoutProcessor configurable (different algorithms)
3. Add streaming support (process pages incrementally)
4. Optimize layout analysis performance
5. Add vision/OCR integration

## Related Documents

- [07-current-architecture-deep.md](./07-current-architecture-deep.md) - Current state analysis
- [08-proposed-architecture-clean.md](./08-proposed-architecture-clean.md) - Proposed architecture
- [05-verification.md](./05-verification.md) - Testing strategy
