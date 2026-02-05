# OODA-43: Observe - Current Default Backend Configuration

## Date: 2026-02-05

## Objective

Understand the current feature flag configuration and backend selection logic before making pdfium the default.

---

## Current Cargo.toml Configuration

```toml
[features]
# Default features include lopdf backend for native PDF parsing
default = ["lopdf"]
lopdf = ["dep:lopdf"]
# pdfium backend for accurate text extraction (requires libpdfium at runtime)
pdfium = ["dep:pdfium-render"]
```

**Key Points:**

- `lopdf` is the current default
- `pdfium` requires explicit `--features pdfium` flag
- Both can be enabled simultaneously

---

## Current Backend Selection Logic

From `extractor.rs`:

```rust
pub fn with_config(llm_provider: Arc<dyn LLMProvider>, config: PdfConfig) -> Self {
    // Select backend based on features
    let backend: Box<dyn PdfBackend> = {
        #[cfg(feature = "lopdf")]
        {
            info!("Using ExtractionEngine (lopdf) for PDF extraction");
            Box::new(crate::backend::ExtractionEngine::with_config(config.clone()))
        }
        #[cfg(not(feature = "lopdf"))]
        {
            tracing::warn!("Using MockBackend for PDF extraction (lopdf feature disabled)");
            Box::new(MockBackend::new())
        }
    };
    // ...
}
```

**Problem:** The logic doesn't check for pdfium feature at all. Even if pdfium is enabled, it falls back to lopdf when both are present.

---

## Evaluation Pipeline Check

From `scripts/eval_comprehensive.py`:

```python
result = subprocess.run(
    ["cargo", "run", "--features", "pdfium", "-p", "edgequake-pdf", ...],
    ...
)
```

The evaluation explicitly requests `--features pdfium`, overriding the default.

---

## convert_pdf_full Example

From checking the examples, `convert_pdf_full.rs` uses the pdfium pipeline:

```rust
// Uses PymupdfPipeline which requires pdfium feature
let pipeline = PymupdfPipeline::new()?;
let markdown = pipeline.convert_file(&args.pdf_path)?;
```

This example is what the evaluation calls, so it uses pdfium directly.

---

## Observations

1. **Dual path**: There are TWO separate pipelines:
   - `PdfExtractor` using `PdfBackend` trait (supports lopdf/mock)
   - `PymupdfPipeline` using `PdfiumExtractor` directly (pdfium only)

2. **The evaluation bypasses PdfExtractor entirely**:
   - It calls `convert_pdf_full` example
   - Which uses `PymupdfPipeline`
   - Which uses `PdfiumExtractor`
   - No `PdfBackend` trait involved

3. **Implication**: Changing the default feature won't affect the evaluation pipeline!

4. **The change needed**:
   - Make pdfium the default for NEW code
   - Update `PdfExtractor` to use pdfium when available
   - Deprecate the lopdf path

---

## File Inventory

| Component          | Uses                         | Feature |
| ------------------ | ---------------------------- | ------- |
| `PdfExtractor`     | lopdf (via ExtractionEngine) | default |
| `PymupdfPipeline`  | pdfium (via PdfiumExtractor) | pdfium  |
| `convert_pdf_full` | PymupdfPipeline              | pdfium  |
| API server         | PdfExtractor                 | default |
| Tests              | PdfExtractor                 | default |

**Critical Insight:** The API server and tests use `PdfExtractor`, which uses lopdf! Only the evaluation script gets the pdfium quality.
