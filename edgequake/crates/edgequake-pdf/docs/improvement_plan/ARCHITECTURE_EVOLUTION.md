# Architecture Evolution: Refactoring for Scale

> **OODA Loop - Decide**: Proposed architectural improvements for better maintainability, extensibility, and performance.

**Goal**: Transform current ad-hoc design into principled architecture  
**Focus**: Separation of concerns, pluggability, testability  
**Timeline**: 4-6 weeks incremental refactoring

---

## Executive Summary

**Current Architecture**: Monolithic with tight coupling  
**Target Architecture**: Modular with clear boundaries  
**Key Improvements**:

1. **Plugin System** for processors (extensibility)
2. **Streaming API** for large documents (scalability)
3. **Backend Abstraction** completed (portability)
4. **Error Handling** redesign (debuggability)
5. **Configuration System** (usability)

---

## 1. Plugin System for Processors

### 1.1 Current Problem

**Code**: [extractor.rs#L260-L290](../../src/extractor.rs#L260-L290)

```rust
// ❌ Hardcoded processor chain - cannot be extended by users
fn apply_processors(&self, doc: Document) -> Result<Document> {
    let chain = ProcessorChain::new()
        .add(MarginFilterProcessor::new())
        .add(GarbledTextFilterProcessor::new())
        .add(LayoutProcessor::new())
        // ... 10 more hardcoded processors
        .add(BlockMergeProcessor::new());

    chain.process(doc)
}
```

**Limitations**:

1. Users cannot add custom processors
2. Cannot disable default processors
3. No processor configuration
4. Fixed processing order

---

### 1.2 Proposed Plugin Architecture

```rust
// New trait for pluggable processors
pub trait ProcessorPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32;  // Lower = earlier in chain
    fn process(&self, doc: Document, config: &Config) -> Result<Document>;
    fn can_process(&self, doc: &Document) -> bool { true }
}

// Registry for managing plugins
pub struct ProcessorRegistry {
    plugins: Vec<Box<dyn ProcessorPlugin>>,
}

impl ProcessorRegistry {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn register(&mut self, plugin: Box<dyn ProcessorPlugin>) {
        self.plugins.push(plugin);
        self.plugins.sort_by_key(|p| p.priority());
    }

    pub fn process(&self, mut doc: Document, config: &Config) -> Result<Document> {
        for plugin in &self.plugins {
            if plugin.can_process(&doc) {
                doc = plugin.process(doc, config)?;
            }
        }
        Ok(doc)
    }
}

// Usage: User can register custom processors
let mut registry = ProcessorRegistry::new();

// Built-in processors
registry.register(Box::new(MarginFilterProcessor::with_priority(100)));
registry.register(Box::new(LayoutProcessor::with_priority(200)));

// User's custom processor
registry.register(Box::new(MyCustomProcessor::with_priority(150)));

let doc = extractor.extract_document(pdf_bytes)?;
let processed = registry.process(doc, &config)?;
```

**Benefits**:

1. **Extensibility**: Users can add domain-specific processors
2. **Flexibility**: Can disable/reorder processors via priority
3. **Configurability**: Each processor gets config object
4. **Testability**: Easy to test individual processors in isolation

---

### 1.3 Example Custom Processor

```rust
// User-defined processor for redacting sensitive information
pub struct RedactionProcessor {
    patterns: Vec<Regex>,
}

impl ProcessorPlugin for RedactionProcessor {
    fn name(&self) -> &str { "redaction" }
    fn priority(&self) -> i32 { 50 }  // Early in pipeline

    fn process(&self, mut doc: Document, config: &Config) -> Result<Document> {
        for page in &mut doc.pages {
            for block in &mut page.blocks {
                for pattern in &self.patterns {
                    block.text = pattern.replace_all(&block.text, "[REDACTED]").to_string();
                }
            }
        }
        Ok(doc)
    }

    fn can_process(&self, doc: &Document) -> bool {
        // Only process if document has text blocks
        doc.pages.iter().any(|p| !p.blocks.is_empty())
    }
}
```

---

## 2. Streaming API for Large Documents

### 2.1 Current Limitation

**Problem**: Load entire PDF into memory

```rust
// ❌ OOM on large documents (>200 pages)
pub async fn extract_document(&self, pdf_bytes: &[u8]) -> Result<Document> {
    let lopdf_doc = LopdfDocument::load_mem(pdf_bytes)?;  // 500MB for 500-page doc

    let mut pages = Vec::new();
    for (page_num, page_id) in page_ids.iter().enumerate() {
        let page = self.backend.extract_page(&lopdf_doc, *page_id, page_num)?;
        pages.push(page);  // Accumulate all pages in RAM
    }

    Ok(Document { pages, .. })  // Document holds 500+ pages
}
```

**Memory Usage**: 2.5 MB/page × 500 pages = 1.25 GB (plus PDF itself)

---

### 2.2 Proposed Streaming API

```rust
use futures::stream::{Stream, StreamExt};

pub struct PdfExtractor {
    // ... existing fields
}

impl PdfExtractor {
    // ✅ Stream pages as they're extracted
    pub fn extract_pages_stream<'a>(
        &'a self,
        pdf_bytes: &'a [u8]
    ) -> impl Stream<Item = Result<Page>> + 'a {
        async_stream::stream! {
            let lopdf_doc = match LopdfDocument::load_mem(pdf_bytes) {
                Ok(doc) => doc,
                Err(e) => {
                    yield Err(PdfError::from(e));
                    return;
                }
            };

            let page_ids = match self.backend.get_page_ids(&lopdf_doc) {
                Ok(ids) => ids,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            for (page_num, page_id) in page_ids.into_iter().enumerate() {
                match self.backend.extract_page(&lopdf_doc, page_id, page_num) {
                    Ok(page) => yield Ok(page),
                    Err(e) => yield Err(e),
                }
            }
        }
    }

    // New: Write directly to file without accumulating pages
    pub async fn extract_to_file_streaming(
        &self,
        pdf_bytes: &[u8],
        output_path: &Path,
    ) -> Result<()> {
        let mut file = tokio::fs::File::create(output_path).await?;
        let renderer = MarkdownRenderer::new();

        let mut stream = self.extract_pages_stream(pdf_bytes);

        while let Some(page_result) = stream.next().await {
            let page = page_result?;
            let markdown = renderer.render_page(&page)?;
            file.write_all(markdown.as_bytes()).await?;
        }

        Ok(())
    }
}
```

**Memory Usage**: 2.5 MB max (single page + PDF overhead) regardless of doc size

---

### 2.3 Usage Examples

```rust
// Example 1: Process pages incrementally
let mut stream = extractor.extract_pages_stream(&pdf_bytes);

while let Some(page_result) = stream.next().await {
    let page = page_result?;

    // Process page immediately (constant memory)
    process_page(page)?;

    // Page is dropped here, memory freed
}

// Example 2: Parallel streaming with bounded concurrency
use futures::stream::StreamExt;

extractor.extract_pages_stream(&pdf_bytes)
    .map(|page_result| async move {
        let page = page_result?;
        enhance_with_llm(page).await
    })
    .buffer_unordered(4)  // Process 4 pages concurrently
    .collect::<Vec<_>>()
    .await;

// Example 3: Write directly to file (no accumulation)
extractor.extract_to_file_streaming(&pdf_bytes, Path::new("output.md")).await?;
```

---

## 3. Enhanced Backend Abstraction

### 3.1 Current Backend Trait

**Code**: [backend/mod.rs](../../src/backend/mod.rs)

```rust
pub trait PdfBackend: Send + Sync {
    fn extract_page(&self, doc: &LopdfDocument, page_id: ObjectId, page_num: usize)
        -> Result<Page>;

    fn get_page_ids(&self, doc: &LopdfDocument) -> Result<Vec<ObjectId>>;
}
```

**Limitations**:

1. Tied to `lopdf` types (not truly abstract)
2. No incremental extraction
3. No backend-specific configuration
4. No progress reporting

---

### 3.2 Improved Backend Abstraction

```rust
pub trait PdfBackend: Send + Sync {
    type Document: Send + Sync;
    type PageId: Send + Sync + Clone;

    // Core extraction
    fn load_document(&self, bytes: &[u8]) -> Result<Self::Document>;
    fn get_page_count(&self, doc: &Self::Document) -> usize;
    fn extract_page(&self, doc: &Self::Document, page_id: &Self::PageId, page_num: usize)
        -> Result<Page>;

    // Metadata
    fn get_metadata(&self, doc: &Self::Document) -> Result<DocumentMetadata>;
    fn get_page_ids(&self, doc: &Self::Document) -> Result<Vec<Self::PageId>>;

    // Capabilities
    fn supports_ocr(&self) -> bool { false }
    fn supports_incremental(&self) -> bool { false }

    // Optional: Progress reporting
    fn set_progress_callback(&mut self, callback: Box<dyn Fn(usize, usize) + Send + Sync>) {}
}
```

**Benefits**:

1. **Portability**: Can implement for PyMuPDF, PDFium, etc.
2. **Flexibility**: Backend-specific document types
3. **Observability**: Progress callbacks
4. **Feature Detection**: Can query backend capabilities

---

### 3.3 Example Alternative Backend

```rust
// PyMuPDF backend (faster native extraction)
pub struct PyMuPdfBackend {
    progress_callback: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
}

impl PdfBackend for PyMuPdfBackend {
    type Document = fitz::Document;
    type PageId = usize;

    fn load_document(&self, bytes: &[u8]) -> Result<Self::Document> {
        fitz::Document::from_bytes(bytes)
            .map_err(|e| PdfError::LoadFailed(e.to_string()))
    }

    fn extract_page(&self, doc: &Self::Document, page_id: &usize, page_num: usize)
        -> Result<Page> {
        let page = doc.load_page(*page_id)?;

        // Use PyMuPDF's native text extraction (3x faster than lopdf)
        let text = page.get_text("text")?;
        let blocks = self.parse_text_blocks(&text)?;

        if let Some(callback) = &self.progress_callback {
            callback(page_num, doc.page_count());
        }

        Ok(Page { blocks, .. })
    }

    fn supports_ocr(&self) -> bool { true }
    fn supports_incremental(&self) -> bool { true }
}
```

---

## 4. Error Handling Redesign

### 4.1 Current Error Enum

**Code**: [src/error.rs](../../src/error.rs)

```rust
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("PDF parsing failed: {0}")]
    PdfParse(String),  // ❌ Too vague

    #[error("Processing error: {0}")]
    Processor(String),  // ❌ No context

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

**Problems**:

1. **No Context**: Which page failed? Which processor?
2. **No Recovery Hints**: How can user fix the issue?
3. **No Error Codes**: Hard to programmatically handle errors

---

### 4.2 Improved Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("Failed to load PDF: {reason}")]
    LoadFailed {
        reason: String,
        file_size: usize,
        hint: String,
    },

    #[error("Page {page} extraction failed: {reason}")]
    PageExtraction {
        page: usize,
        reason: String,
        recoverable: bool,
    },

    #[error("Processor '{processor}' failed at page {page}: {reason}")]
    ProcessorFailed {
        processor: String,
        page: usize,
        reason: String,
        backtrace: Backtrace,
    },

    #[error("Font '{font}' encoding error: {reason}")]
    FontEncoding {
        font: String,
        reason: String,
        hint: String,
    },

    #[error("Table detection failed: {reason}")]
    TableDetection {
        reason: String,
        lines_found: usize,
        tables_found: usize,
    },
}

impl PdfError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            PdfError::PageExtraction { recoverable, .. } => *recoverable,
            PdfError::ProcessorFailed { .. } => true,
            _ => false,
        }
    }

    pub fn recovery_hint(&self) -> Option<&str> {
        match self {
            PdfError::LoadFailed { hint, .. } => Some(hint),
            PdfError::FontEncoding { hint, .. } => Some(hint),
            _ => None,
        }
    }
}
```

**Error Usage**:

```rust
// Example: Better error reporting
fn extract_page(&self, doc: &LopdfDocument, page_id: ObjectId, page_num: usize)
    -> Result<Page> {

    let page_dict = doc.get_object(page_id)?
        .as_dict()
        .map_err(|_| PdfError::PageExtraction {
            page: page_num,
            reason: "Page dictionary is not a dictionary object".to_string(),
            recoverable: false,
        })?;

    let fonts = self.load_fonts(page_dict)
        .map_err(|e| PdfError::PageExtraction {
            page: page_num,
            reason: format!("Font loading failed: {}", e),
            recoverable: true,  // Can continue with default font
        })?;

    // ...
}
```

---

## 5. Configuration System

### 5.1 Current Configuration

**Problem**: Hardcoded constants scattered across codebase

```rust
// margin_filter.rs
const LEFT_MARGIN: f32 = 50.0;  // ❌ Magic number
const RIGHT_MARGIN: f32 = 50.0;

// table_detector.rs
const MIN_TABLE_ROWS: usize = 2;  // ❌ Not configurable

// header_detector.rs
const HEADING_RATIO_THRESHOLD: f32 = 1.3;  // ❌ Cannot tune
```

---

### 5.2 Proposed Configuration System

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    pub general: GeneralConfig,
    pub layout: LayoutConfig,
    pub processors: ProcessorConfig,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub backend: BackendType,
    pub output_format: OutputFormat,
    pub error_handling: ErrorHandling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub margin_threshold: f32,  // Configurable margin size
    pub column_gap_min: f32,
    pub line_spacing_factor: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    pub enabled_processors: Vec<String>,
    pub processor_priorities: HashMap<String, i32>,

    pub heading: HeadingConfig,
    pub table: TableConfig,
    pub list: ListConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadingConfig {
    pub ratio_threshold: f32,  // 1.3
    pub min_ratio_for_h1: f32,  // 2.0
    pub detect_subsections: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableConfig {
    pub min_rows: usize,
    pub min_cols: usize,
    pub enable_lattice: bool,
    pub enable_text_table: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub parallel_pages: bool,
    pub thread_count: Option<usize>,  // None = auto-detect
    pub use_spatial_index: bool,
    pub enable_object_pooling: bool,
}

impl ExtractionConfig {
    // Load from file
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: ExtractionConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    // Load from environment
    pub fn from_env() -> Result<Self> {
        envy::from_env().map_err(|e| PdfError::ConfigError(e.to_string()))
    }

    // Default configuration
    pub fn default() -> Self {
        // ...
    }
}
```

---

### 5.3 Configuration File Format (TOML)

```toml
# edgequake.toml

[general]
backend = "lopdf"
output_format = "markdown"
error_handling = "continue_on_recoverable"

[layout]
margin_threshold = 50.0
column_gap_min = 30.0
line_spacing_factor = 1.5

[processors]
enabled_processors = [
    "margin_filter",
    "layout",
    "header_detection",
    "table_detection",
    "list_detection"
]

[processors.heading]
ratio_threshold = 1.3
min_ratio_for_h1 = 2.0
detect_subsections = true

[processors.table]
min_rows = 2
min_cols = 2
enable_lattice = true
enable_text_table = true

[performance]
parallel_pages = true
thread_count = 4
use_spatial_index = true
enable_object_pooling = true
```

**Usage**:

```rust
// Load from file
let config = ExtractionConfig::from_file("edgequake.toml")?;

// Or use builder pattern
let config = ExtractionConfig::default()
    .with_parallel_pages(true)
    .with_thread_count(8)
    .with_heading_ratio(1.5);

let extractor = PdfExtractor::with_config(provider, config);
let doc = extractor.extract_document(&pdf_bytes)?;
```

---

## 6. Observability & Instrumentation

### 6.1 Progress Reporting

```rust
pub trait ProgressReporter: Send + Sync {
    fn report_progress(&self, current: usize, total: usize, message: &str);
}

pub struct PdfExtractor {
    backend: Arc<dyn PdfBackend>,
    provider: Arc<dyn LLMProvider>,
    progress: Option<Arc<dyn ProgressReporter>>,
}

impl PdfExtractor {
    pub fn with_progress<R>(mut self, reporter: R) -> Self
    where R: ProgressReporter + 'static {
        self.progress = Some(Arc::new(reporter));
        self
    }

    pub async fn extract_document(&self, pdf_bytes: &[u8]) -> Result<Document> {
        // ... extract pages

        for (page_num, page_id) in page_ids.iter().enumerate() {
            if let Some(reporter) = &self.progress {
                reporter.report_progress(
                    page_num + 1,
                    page_ids.len(),
                    &format!("Extracting page {}", page_num + 1)
                );
            }

            let page = self.backend.extract_page(&lopdf_doc, *page_id, page_num)?;
            pages.push(page);
        }

        // ...
    }
}
```

**Usage**:

```rust
// CLI progress bar
struct CliProgressReporter;

impl ProgressReporter for CliProgressReporter {
    fn report_progress(&self, current: usize, total: usize, message: &str) {
        println!("[{}/{}] {}", current, total, message);
    }
}

let extractor = PdfExtractor::new(provider)
    .with_progress(CliProgressReporter);
```

---

### 6.2 Performance Metrics

```rust
pub struct ExtractionMetrics {
    pub total_time: Duration,
    pub page_times: Vec<Duration>,
    pub processor_times: HashMap<String, Duration>,
    pub table_detection_time: Duration,
    pub memory_peak: usize,
}

impl PdfExtractor {
    pub fn extract_with_metrics(&self, pdf_bytes: &[u8])
        -> Result<(Document, ExtractionMetrics)> {

        let start = Instant::now();
        let mut metrics = ExtractionMetrics::default();

        // ... extraction with timing

        metrics.total_time = start.elapsed();

        Ok((doc, metrics))
    }
}
```

---

## 7. Migration Path

### Phase 1: Non-Breaking Additions (Week 1-2)

1. Add `ProcessorPlugin` trait alongside existing `Processor`
2. Add `extract_pages_stream()` alongside `extract_document()`
3. Add `ExtractionConfig` with `Default` impl
4. Add new error types without removing old ones

**No breaking changes** - existing code continues to work.

---

### Phase 2: Deprecation Warnings (Week 3-4)

1. Mark old APIs as `#[deprecated]` with migration hints
2. Add examples using new APIs
3. Update documentation
4. Add migration guide

---

### Phase 3: Breaking Changes (Week 5-6)

1. Remove deprecated APIs (major version bump)
2. Make `ExtractionConfig` required in constructor
3. Replace `PdfError` with new error types
4. Update all examples and tests

---

## 8. Implementation Checklist

### Plugin System

- [ ] Define `ProcessorPlugin` trait
- [ ] Create `ProcessorRegistry` struct
- [ ] Add priority-based sorting
- [ ] Implement `can_process()` predicate
- [ ] Add configuration passing
- [ ] Write plugin example
- [ ] Update documentation

### Streaming API

- [ ] Add `async-stream` dependency
- [ ] Implement `extract_pages_stream()`
- [ ] Add `extract_to_file_streaming()`
- [ ] Test with large documents (500+ pages)
- [ ] Benchmark memory usage
- [ ] Update examples

### Backend Abstraction

- [ ] Refactor `PdfBackend` trait with associated types
- [ ] Add progress callback support
- [ ] Add capability queries
- [ ] Implement for lopdf backend
- [ ] (Optional) Implement PyMuPDF backend
- [ ] Update documentation

### Error Handling

- [ ] Define new `PdfError` variants
- [ ] Add context fields (page, processor, etc.)
- [ ] Implement recovery hints
- [ ] Add `is_recoverable()` method
- [ ] Update all error sites
- [ ] Add error handling examples

### Configuration System

- [ ] Define `ExtractionConfig` structs
- [ ] Add TOML serialization
- [ ] Add environment variable loading
- [ ] Add builder pattern API
- [ ] Update all processors to use config
- [ ] Create example config file
- [ ] Document all options

---

## Next Document

[QUALITY_GAPS.md](QUALITY_GAPS.md) - Missing features and quality improvements for production readiness.
