# Improvement Plan

## Phase 1: Abstraction Layer

- [x] **Step 1.1**: Create `src/backend/mod.rs` and define the `PdfBackend` trait.
  - Methods: `extract`, `get_info`.
  - Use `async_trait` if needed (though `pdfium` is mostly CPU bound, wrapping it in async might be useful for the interface).
- [x] **Step 1.2**: Move `PdfiumExtractor` logic to `src/backend/pdfium.rs` and implement `PdfBackend`.
  - Ensure feature flag `pdfium` guards this module.
- [x] **Step 1.3**: Create `src/backend/mock.rs` for testing.
  - Implement a backend that returns a static `Document`.

## Phase 2: Refactoring Core

- [x] **Step 2.1**: Refactor `PdfExtractor` (or create `PdfPipeline`) to use `Box<dyn PdfBackend>`.
  - Remove direct dependency on `PdfiumExtractor`.
  - Update constructor to accept a backend.
- [x] **Step 2.2**: Make `ProcessorChain` configurable.
  - Add methods to `PdfExtractor` to append/replace processors.
  - Provide a default factory method that sets up the standard chain.

## Phase 3: Cleanup and Testing

- [x] **Step 3.1**: Update `lib.rs` to export the new modules.
- [x] **Step 3.2**: Update existing tests to use the new architecture.
  - Use `MockBackend` for pipeline tests.
- [x] **Step 3.3**: Verify `pdfium` integration still works with the new trait.
- [x] **Step 3.4**: Add new tests for the decoupled components.

## Phase 4: Documentation

- [x] **Step 4.1**: Update crate documentation in `lib.rs`.
- [x] **Step 4.2**: Add examples of how to implement a custom backend or processor.

## Detailed Tasks

### Task 1: Define `PdfBackend` Trait

```rust
// src/backend/mod.rs
use crate::Result;
use crate::schema::Document;
use crate::extractor::PdfInfo;
use async_trait::async_trait;

#[async_trait]
pub trait PdfBackend: Send + Sync {
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document>;
    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo>;
}
```

### Task 2: Implement `PdfiumBackend`

Adapt `src/pdfium_extractor.rs` to implement the trait.

### Task 3: Refactor `PdfExtractor`

Change `PdfExtractor` struct:

```rust
pub struct PdfExtractor {
    backend: Box<dyn PdfBackend>,
    processors: ProcessorChain,
    llm_provider: Arc<dyn LLMProvider>,
    config: PdfConfig,
}
```

### Task 4: Default Implementation

Provide a convenience method that tries to use `PdfiumBackend` if available.

```rust
impl PdfExtractor {
    pub fn new_with_defaults(llm_provider: Arc<dyn LLMProvider>) -> Result<Self> {
        #[cfg(feature = "pdfium")]
        let backend = Box::new(backend::pdfium::PdfiumBackend::new()?);

        #[cfg(not(feature = "pdfium"))]
        let backend = Box::new(backend::mock::MockBackend::new()); // Or error

        Ok(Self::new(backend, llm_provider))
    }
}
```
