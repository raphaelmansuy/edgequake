# Current Architecture Analysis

## Overview

The `edgequake-pdf` crate is designed to extract content from PDF files and convert it into Markdown. It currently relies heavily on `pdfium-render` for the low-level extraction.

## Component Diagram

```ascii
+---------------------------------------------------------+
|                     PdfExtractor                        |
|                                                         |
|  +-------------------+    +--------------------------+  |
|  |   Configuration   |    |      LLM Provider        |  |
|  +-------------------+    +--------------------------+  |
|                                                         |
|  +---------------------------------------------------+  |
|  |                 Extraction Logic                  |  |
|  |                                                   |  |
|  |  [ Conditional Compilation: feature = "pdfium" ]  |  |
|  |                                                   |  |
|  |  +-------------------+                            |  |
|  |  |  PdfiumExtractor  | <--- Hardcoded Dependency  |  |
|  |  +-------------------+                            |  |
|  |           |                                       |  |
|  +-----------|---------------------------------------+  |
|              |                                          |
|              v                                          |
|      +---------------+                                  |
|      |   Document    |  (Intermediate Representation)   |
|      +---------------+                                  |
|              |                                          |
|              v                                          |
|  +-----------------------+                              |
|  |    ProcessorChain     |                              |
|  |                       |                              |
|  |  - LayoutProcessor    |                              |
|  |  - TableDetection     |                              |
|  |  - HeaderDetection    |                              |
|  |  - ...                |                              |
|  +-----------------------+                              |
|              |                                          |
|              v                                          |
|      +---------------+                                  |
|      |   Document    |  (Enhanced)                      |
|      +---------------+                                  |
|              |                                          |
|              v                                          |
|  +-----------------------+                              |
|  |   MarkdownRenderer    |                              |
|  +-----------------------+                              |
|              |                                          |
|              v                                          |
|      +---------------+                                  |
|      |    String     |  (Markdown Output)               |
|      +---------------+                                  |
+---------------------------------------------------------+
```

## Critical Analysis

1.  **Tight Coupling**: `PdfExtractor` is tightly coupled to `PdfiumExtractor`. The dependency is managed via `#[cfg(feature = "pdfium")]` blocks directly inside the `extract_document` method. This makes it impossible to swap the backend at runtime or easily mock it for testing without feature flags.

2.  **Violation of Dependency Inversion**: High-level policy (`PdfExtractor`) depends on low-level detail (`PdfiumExtractor`). Both should depend on abstractions.

3.  **Rigid Pipeline**: The `ProcessorChain` is constructed inside `apply_processors` with a fixed set of processors. Users cannot easily customize the pipeline (e.g., disable table detection or add a custom processor) without modifying the code.

4.  **Mixed Responsibilities**: `PdfExtractor` is responsible for:

    - Initializing the backend.
    - Running the extraction.
    - Constructing and running the processing pipeline.
    - Invoking the renderer.
    - Handling LLM enhancement.

5.  **Testability**: Testing `PdfExtractor` requires the `pdfium` library to be present. There is no easy way to unit test the orchestration logic with a mock extractor.

## Code Evidence

From `src/extractor.rs`:

```rust
pub async fn extract_document(&self, _pdf_bytes: &[u8]) -> Result<Document> {
    // ...
    #[cfg(feature = "pdfium")]
    {
        // Direct instantiation of concrete type
        let pdfium_extractor = PdfiumExtractor::with_config(self.config.clone())
            .map_err(|e| PdfError::PdfParse(format!("Failed to initialize Pdfium: {}", e)))?;

        // ...
    }
    // ...
}
```

From `src/extractor.rs`:

```rust
async fn apply_processors(&self, document: Document) -> Result<Document> {
    // Hardcoded chain
    let chain = ProcessorChain::new()
        .add(LayoutProcessor::new())
        .add(TableDetectionProcessor::new())
        // ...
        .add(PostProcessor::new());

    chain.process(document)
        // ...
}
```
