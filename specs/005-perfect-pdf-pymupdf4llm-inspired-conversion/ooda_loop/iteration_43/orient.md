# OODA-43: Orient - Integration Strategy

## Date: 2026-02-05

## Gap Analysis

### The Real Problem

The pdfium pipeline (`PymupdfPipeline`) and the main pipeline (`PdfExtractor`) are **completely separate**:

```
PdfExtractor (used by API/tests)          PymupdfPipeline (used by eval)
         │                                          │
         ▼                                          ▼
   PdfBackend trait                          PdfiumExtractor
         │                                          │
         ▼                                          ▼
   ExtractionEngine                          RawChar extraction
   (lopdf, 1302 lines)                       (pdfium, 304 lines)
         │                                          │
         ▼                                          ▼
   TextElement structs                       TextGrouper
   (unreliable font)                         (pymupdf_grouper, 1362 lines)
         │                                          │
         ▼                                          ▼
   ProcessorChain                            MarkdownRenderer
   (15+ processors)                          (pymupdf_renderer)
         │                                          │
         ▼                                          ▼
   MarkdownRenderer                          Markdown output
   (renderers/)                              (quality = 0.786)
         │
         ▼
   Markdown output
   (quality = UNKNOWN)
```

### Option A: Make pdfium default feature

**Pros:**

- Simple change
- Backwards compatible

**Cons:**

- Won't change PdfExtractor behavior (still uses lopdf when both enabled)
- Users need libpdfium.dylib at runtime
- Doesn't integrate pdfium into main pipeline

### Option B: Integrate PymupdfPipeline into PdfExtractor

**Pros:**

- Single pipeline for everything
- API gets pdfium quality
- Tests validate pdfium

**Cons:**

- Major refactoring
- ProcessorChain needs integration with pdfium
- Risk of regressions

### Option C: Create pdfium-based PdfBackend

**Pros:**

- Uses existing abstraction
- Gradual migration
- Can keep ProcessorChain

**Cons:**

- Need to adapt output format
- May not get full pdfium benefits

---

## Decision

**Go with Option C (create pdfium-based PdfBackend)** as the safest path:

1. Create `PdfiumBackend` implementing `PdfBackend` trait
2. Make it the default when `pdfium` feature is enabled
3. Convert `RawChar` → `TextElement` → existing pipeline
4. Leverage existing ProcessorChain for post-processing

This approach:

- Minimizes code changes
- Uses proven pdfium character extraction
- Keeps existing post-processing
- Allows gradual deprecation of lopdf

---

## Implementation Plan

### Step 1: Create PdfiumBackend

New file: `backend/pdfium_backend.rs`

```rust
pub struct PdfiumBackend {
    extractor: PdfiumExtractor,
    config: PdfConfig,
}

impl PdfBackend for PdfiumBackend {
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document> {
        // 1. Extract RawChar using PdfiumExtractor
        let chars = self.extractor.extract_chars_from_bytes(pdf_bytes)?;

        // 2. Group chars → blocks using TextGrouper from pymupdf_grouper
        let grouper = TextGrouper::default();
        let blocks = grouper.group(&chars);

        // 3. Convert to schema::Document
        self.build_document(blocks)
    }
}
```

### Step 2: Update extractor.rs

```rust
pub fn with_config(llm_provider: Arc<dyn LLMProvider>, config: PdfConfig) -> Self {
    let backend: Box<dyn PdfBackend> = {
        // Prefer pdfium when available
        #[cfg(feature = "pdfium")]
        {
            match PdfiumBackend::new(config.clone()) {
                Ok(backend) => {
                    info!("Using PdfiumBackend for high-quality extraction");
                    Box::new(backend)
                }
                Err(e) => {
                    warn!("Failed to init pdfium, falling back: {}", e);
                    // Fall through to lopdf or mock
                }
            }
        }
        #[cfg(feature = "lopdf")]
        {
            info!("Using ExtractionEngine (lopdf) for PDF extraction");
            Box::new(ExtractionEngine::with_config(config.clone()))
        }
        #[cfg(not(any(feature = "pdfium", feature = "lopdf")))]
        {
            warn!("Using MockBackend (no PDF features enabled)");
            Box::new(MockBackend::new())
        }
    };
    // ...
}
```

### Step 3: Update Cargo.toml

```toml
[features]
# Default: pdfium for quality, lopdf as fallback
default = ["pdfium", "lopdf"]
```

---

## Risks and Mitigations

| Risk                           | Mitigation                            |
| ------------------------------ | ------------------------------------- |
| Runtime library missing        | Graceful fallback to lopdf            |
| Quality regression             | Compare before/after with eval script |
| ProcessorChain incompatibility | Test each processor                   |
| Build breaks                   | Keep lopdf as fallback                |
