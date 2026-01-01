# Proposed Architecture

## Core Concept

The goal is to decouple the PDF extraction engine from the orchestration logic and the document processing pipeline. We will introduce a `PdfBackend` trait to abstract the source of the `Document` IR.

## New Component Diagram

```ascii
                                   +---------------------+
                                   |     Client Code     |
                                   +---------------------+
                                              |
                                              v
+-----------------------------------------------------------------------------------------+
|                                     PdfPipeline                                         |
|                                                                                         |
|  +----------------+      +-----------------------+      +----------------------------+  |
|  |   PdfBackend   | ---> |    ProcessorChain     | ---> |          Renderer          |  |
|  |    (Trait)     |      |                       |      |          (Trait)           |  |
|  +----------------+      +-----------------------+      +----------------------------+  |
|          ^                           ^                                 ^                |
|          |                           |                                 |                |
|          |                           |                                 |                |
+----------|---------------------------|---------------------------------|----------------+
           |                           |                                 |
           |                           |                                 |
+---------------------+    +-----------------------+      +----------------------------+
|   PdfiumBackend     |    | - LayoutProcessor     |      |      MarkdownRenderer      |
| (Impl PdfBackend)   |    | - TableProcessor      |      |    (Impl Renderer)         |
|                     |    | - CustomProcessor     |      |                            |
+---------------------+    +-----------------------+      +----------------------------+
           |
           v
    +-------------+
    | pdfium-lib  |
    +-------------+
```

## Key Changes

### 1. `PdfBackend` Trait

Define a clear interface for extracting a `Document` from bytes.

```rust
#[async_trait]
pub trait PdfBackend: Send + Sync {
    /// Extract the raw document structure from PDF bytes.
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document>;

    /// Get metadata/info about the PDF.
    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo>;
}
```

### 2. `PdfPipeline` (formerly `PdfExtractor`)

Refactor `PdfExtractor` (or rename to `PdfPipeline`) to be an orchestrator that composes these components.

```rust
pub struct PdfPipeline {
    backend: Box<dyn PdfBackend>,
    processors: ProcessorChain,
    renderer: Box<dyn Renderer>, // Optional, if we want to abstract rendering too
    llm_provider: Option<Arc<dyn LLMProvider>>,
}

impl PdfPipeline {
    pub fn new(backend: Box<dyn PdfBackend>) -> Self {
        // ...
    }

    pub fn with_processors(mut self, processors: ProcessorChain) -> Self {
        self.processors = processors;
        self
    }

    pub async fn run(&self, pdf_bytes: &[u8]) -> Result<String> {
        // 1. Extract
        let mut doc = self.backend.extract(pdf_bytes).await?;

        // 2. Process
        doc = self.processors.process(doc)?;

        // 3. LLM Enhance (if configured/provider present)
        if let Some(provider) = &self.llm_provider {
             // ...
        }

        // 4. Render
        self.renderer.render(&doc)
    }
}
```

### 3. `PdfiumBackend`

Move the logic from `PdfiumExtractor` into a struct that implements `PdfBackend`.

```rust
pub struct PdfiumBackend {
    config: PdfConfig,
    // ...
}

#[async_trait]
impl PdfBackend for PdfiumBackend {
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document> {
        // Existing logic from PdfiumExtractor::extract_document
    }
    // ...
}
```

## Benefits

1.  **Decoupling**: `PdfPipeline` doesn't know about `pdfium`. It just knows it can get a `Document`.
2.  **Testability**: We can implement a `MockBackend` that returns a pre-defined `Document` struct for testing the pipeline and processors without needing a real PDF or the `pdfium` library.
3.  **Flexibility**: We can easily add other backends (e.g., `poppler`, `mupdf`, or a cloud-based OCR service) by implementing `PdfBackend`.
4.  **Configurability**: Users can build their own processing pipeline.

## Directory Structure Changes

```
src/
  backend/
    mod.rs          <-- Trait definition
    pdfium.rs       <-- Pdfium implementation
    mock.rs         <-- Mock implementation for tests
  pipeline/
    mod.rs          <-- Pipeline orchestration
  ...
```
