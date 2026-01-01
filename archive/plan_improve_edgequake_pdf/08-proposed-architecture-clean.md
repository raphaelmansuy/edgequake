# Proposed Architecture: Separation of Concerns

## Core Principle

**Backends extract, Processors analyze, Renderers format.**

Each layer has a single, well-defined responsibility with no overlap.

## Revised Component Responsibilities

### 1. Backend Layer: Raw Extraction Only

**Single Responsibility:** Extract raw text/blocks from PDF bytes

**What it SHOULD do:**

- Load PDF from bytes
- Extract characters with positions
- Group characters into words (whitespace detection)
- Group words into lines (y-coordinate clustering)
- Create Block objects with bounding boxes
- Extract metadata (title, author, page count)
- Extract images (if configured)

**What it should NOT do:**

- ❌ Detect columns
- ❌ Analyze reading order
- ❌ Sort blocks
- ❌ Detect document structure

**Revised PdfBackend contract:**

```rust
#[async_trait]
pub trait PdfBackend: Send + Sync {
    /// Extract raw blocks from PDF (unsorted, no layout analysis)
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document>;

    /// Get PDF metadata without full extraction
    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo>;
}
```

**Post-Refactor Behavior:**

```rust
// PdfiumBackend::extract() should return:
Document {
    pages: vec![
        Page {
            blocks: vec![/* UNSORTED blocks */],
            columns: vec![],  // ← Empty!
            // ...
        }
    ]
}
```

### 2. Layout Analysis: Structure Detection

**Single Responsibility:** Analyze block structure and reading order

**Handled by:** `LayoutProcessor` (ONLY)

**What it does:**

- Detect page columns using horizontal projection
- Determine reading order (Z-order + multi-column)
- Apply XY-Cut recursive segmentation
- Populate `page.columns`
- Sort `page.blocks` by reading order

**When it runs:** First processor in the chain

### 3. Processors: Semantic Enhancement

**Single Responsibility:** Add semantic meaning to blocks

**Processors:**

1. **LayoutProcessor** - Structure analysis (columns, order)
2. **TableDetectionProcessor** - Identify tables from grid layout
3. **HeaderDetectionProcessor** - Detect headings from font size/weight
4. **CaptionDetectionProcessor** - Associate captions with figures
5. **ListDetectionProcessor** - Identify bullet/numbered lists
6. **CodeBlockDetectionProcessor** - Detect code from monospace fonts
7. **BlockMergeProcessor** - Merge adjacent paragraphs
8. **NormalizeProcessor** - Clean whitespace

**Pipeline guarantees:**

- Each processor receives a valid Document
- Processors are independent (can be reordered)
- Processing is idempotent

### 4. Renderers: Output Generation

**Single Responsibility:** Convert Document to target format

**No change needed** - already clean separation

## Revised Data Flow

```
┌───────────┐
│ PDF Bytes │
└─────┬─────┘
      │
      ▼
┌──────────────────────────────────────┐
│ PdfBackend::extract()                │
│ ┌──────────────────────────────────┐ │
│ │ 1. Load PDF                      │ │
│ │ 2. Extract characters + position │ │
│ │ 3. Group into words              │ │
│ │ 4. Group into lines              │ │
│ │ 5. Create UNSORTED blocks        │ │
│ │ 6. Extract images                │ │
│ └──────────────────────────────────┘ │
└─────┬────────────────────────────────┘
      │
      ▼
┌─────────────────────┐
│ Document (raw)      │ ← Blocks unsorted, no columns
│ • Unsorted blocks   │
│ • No columns        │
│ • Raw structure     │
└─────┬───────────────┘
      │
      ▼
┌──────────────────────────────────────┐
│ LayoutProcessor::process()           │
│ ┌──────────────────────────────────┐ │
│ │ 1. Analyze layout (XY-Cut)       │ │
│ │ 2. Detect columns                │ │
│ │ 3. Determine reading order       │ │
│ │ 4. Sort blocks                   │ │
│ │ 5. Set page.columns              │ │
│ └──────────────────────────────────┘ │
└─────┬────────────────────────────────┘
      │
      ▼
┌─────────────────────┐
│ Document (analyzed) │ ← Now sorted, columns detected
└─────┬───────────────┘
      │
      ▼
┌─────────────────────┐
│ Other Processors    │
│ • TableDetection    │
│ • HeaderDetection   │
│ • ListDetection     │
│ • BlockMerge        │
│ • Normalize         │
└─────┬───────────────┘
      │
      ▼
┌─────────────────────┐
│ Renderer            │
└─────┬───────────────┘
      │
      ▼
┌─────────────────────┐
│ Output String       │
└─────────────────────┘
```

## Benefits of This Architecture

### 1. Single Responsibility

- Each component does ONE thing
- Easy to understand and maintain
- Clear testing boundaries

### 2. Consistent Abstraction

- All backends return raw, unsorted blocks
- No special cases
- Predictable behavior

### 3. Pluggable Components

- Can swap layout algorithm without touching backend
- Can skip layout if not needed
- Can add alternative backends easily

### 4. Better Testability

- Backend can be tested without layout logic
- Layout can be tested with synthetic blocks
- Processors test in isolation

### 5. Performance

- No duplicate work
- Clear optimization points
- Can profile each stage independently

## Backend Comparison (Before vs After)

### Before (Current)

```rust
// PdfiumBackend::extract()
let blocks = self.extract_page_blocks(page, page_num)?;

// ❌ Backend does layout analysis
let analyzer = LayoutAnalyzer::new();
let layout = analyzer.analyze(&blocks, width, height);
page.columns = layout.columns;
analyzer.sort_by_reading_order(&mut page.blocks, &page.columns);

return Document { pages, ... }; // Already sorted!
```

### After (Proposed)

```rust
// PdfiumBackend::extract()
let blocks = self.extract_page_blocks(page, page_num)?;

// ✅ Backend just returns raw blocks
let page = Page {
    blocks,  // UNSORTED!
    columns: vec![],  // EMPTY!
    ...
};

return Document { pages, ... }; // Raw data only
```

### MockBackend (No Change Needed!)

```rust
// MockBackend already does it right:
async fn extract(&self, _pdf_bytes: &[u8]) -> Result<Document> {
    Ok(self.document.clone())  // Returns whatever you give it
}
```

## Processor Pipeline (No Change)

The pipeline already has LayoutProcessor first, so it will work correctly once backend is fixed:

```rust
ProcessorChain::new()
    .add(LayoutProcessor::new())  // ← Will now be the ONLY place doing layout
    .add(TableDetectionProcessor::new())
    .add(HeaderDetectionProcessor::new())
    // ... rest unchanged
```

## Migration Path

### Phase 1: Refactor PdfiumBackend (CRITICAL)

1. Remove `LayoutAnalyzer` instantiation from `extract()`
2. Remove `analyzer.analyze()` call
3. Remove `sort_by_reading_order()` call
4. Leave `page.columns` empty
5. Return unsorted blocks

### Phase 2: Verify Tests

1. Run existing tests (should still pass)
2. Add test to verify blocks are unsorted after backend
3. Add test to verify blocks are sorted after LayoutProcessor

### Phase 3: Update Documentation

1. Update backend trait documentation
2. Add architecture diagram to README
3. Document the separation of concerns

## Testing Strategy

### Backend Tests

```rust
#[test]
fn test_backend_returns_unsorted_blocks() {
    let backend = PdfiumBackend::new().unwrap();
    let doc = backend.extract(pdf_bytes).await.unwrap();

    // Blocks should be unsorted
    assert!(doc.pages[0].columns.is_empty());

    // Blocks should have valid bounding boxes
    for block in &doc.pages[0].blocks {
        assert!(block.bbox.width() > 0.0);
    }
}
```

### Layout Processor Tests

```rust
#[test]
fn test_layout_processor_sorts_blocks() {
    let mut doc = create_unsorted_document();
    assert!(doc.pages[0].columns.is_empty());

    let processor = LayoutProcessor::new();
    let result = processor.process(doc).unwrap();

    // Should now have columns
    assert!(!result.pages[0].columns.is_empty());

    // Blocks should be in reading order
    // (check y-coordinate is monotonic within columns)
}
```

### Integration Tests

```rust
#[test]
fn test_full_pipeline() {
    let backend = PdfiumBackend::new().unwrap();
    let extractor = PdfExtractor::with_backend(backend, ...);

    let markdown = extractor.extract_to_markdown(pdf_bytes).await.unwrap();

    // Should have proper document structure
    assert!(markdown.contains("# Heading"));
    assert!(markdown.contains("| Table | Cell |"));
}
```

## Rollout Plan

1. ✅ Create this proposal document
2. ⏳ Review with team/stakeholders
3. ⏳ Make changes to PdfiumBackend (1 file, ~10 lines removed)
4. ⏳ Run full test suite
5. ⏳ Add new tests for separation
6. ⏳ Update documentation
7. ✅ Merge to main

## Risk Mitigation

### Risk: Breaking existing code

**Mitigation:** Full test suite must pass before merge

### Risk: Performance regression

**Mitigation:** Benchmark before/after (should be faster!)

### Risk: Quality degradation

**Mitigation:** Integration tests verify output quality

## Success Metrics

- ✅ All tests pass
- ✅ Backend is < 400 lines (from 494)
- ✅ No duplicate layout analysis
- ✅ MockBackend and PdfiumBackend behave consistently
- ✅ Can swap backends without changing pipeline

## Next Steps

See [08-implementation-roadmap.md](./08-implementation-roadmap.md) for detailed implementation plan.
