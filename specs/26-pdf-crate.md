# EdgeQuake PDF to Markdown Extraction Crate Specification

**Goal:** Create a generic and pluggable Rust crate for transforming PDFs into structured Markdown using EdgeQuake's existing LLM provider system  
**Date:** 2025-12-31  
**Status:** 📋 SPECIFICATION COMPLETE - Ready for Implementation

---

## Executive Summary

**Problem:** EdgeQuake needs a robust PDF processing capability to extract structured Markdown from PDF documents, leveraging existing LLM infrastructure for AI-enhanced processing.

**Solution:** `edgequake-pdf` crate with CLI tool that integrates seamlessly with EdgeQuake's LLM provider system, supporting text, tables, images, and scanned documents with >95% accuracy.

**Key Features:**
- ✅ Generic & pluggable LLM provider integration
- ✅ AI-enhanced table and image processing
- ✅ Comprehensive CLI tool for standalone usage
- ✅ Fallback mechanisms for reliability
- ✅ Performance optimized (<15s for 100-page PDFs)

---

## Business Rules

**R001:** All PDF processing must use EdgeQuake's existing LLM provider abstraction, never direct API calls.  
**See:** `edgequake-llm/src/traits.rs`

**R002:** The crate must be generic and pluggable - any LLM provider implementing the trait should work seamlessly.  
**See:** `edgequake-llm/src/providers/`

**R003:** Processing must include graceful degradation - if AI features fail, rule-based extraction should still work.  
**See:** `edgequake-pdf/src/extractor.rs`

**R004:** CLI tool must follow EdgeQuake's command-line conventions and support all major use cases.  
**See:** `edgequake/crates/` structure

**R005:** All extracted content must preserve document structure, page numbers, and reading order.  
**See:** `edgequake-pdf/src/config.rs`

---

## Current State & Gap Analysis

### ✅ What EdgeQuake Already Has
- Robust LLM provider abstraction (`edgequake-llm`)
- OpenAI and mock provider implementations
- Async Rust infrastructure with tokio
- Comprehensive testing framework
- Workspace-integrated crate structure

### 🔴 Critical Gaps (Must Fix)
- **No PDF processing capability**
- **No vision support in LLM providers**
- **No CLI tools for document processing**
- **No structured Markdown output from PDFs**

### 🟡 Nice-to-Have Gaps
- Advanced layout handling
- OCR optimization
- Performance benchmarking
- Cross-platform testing

---

## Implementation Plan - 7 Phases

### Phase 1: Foundation & Critical Mitigations (Week 1) 🔴 CRITICAL
**Goal:** Establish working PDF extraction with fallback strategies

**Tasks:**
1. **Research & Validate `pdf_oxide` API** 🔄 IN PROGRESS
   - Test current API compatibility
   - Implement fallback to `lopdf` + `pdf-extract` if needed
   - **Files:** `edgequake/crates/edgequake-pdf/Cargo.toml`

2. **Extend `edgequake-llm` with Vision Support** 📋 PLANNED
   - Add `VisionProvider` trait
   - Implement vision in OpenAI provider
   - Update mock provider for testing
   - **Files:** `edgequake-llm/src/traits.rs`, `edgequake-llm/src/providers/`

3. **Create Basic PDF Crate Structure** 📋 PLANNED
   - Set up `edgequake-pdf` crate
   - Implement core extraction logic
   - Add to workspace Cargo.toml
   - **Files:** `edgequake/crates/edgequake-pdf/src/lib.rs`

4. **Working Prototype** 📋 PLANNED
   - Text extraction from native PDFs
   - Basic Markdown output
   - Integration testing
   - **Files:** `edgequake/crates/edgequake-pdf/tests/`

### Phase 2: Core AI Enhancement (Week 2-3) 🟡 HIGH PRIORITY
**Goal:** Add AI-powered processing for complex content

**Tasks:**
1. **AI Enhancement Pipeline** 📋 PLANNED
   - Integrate vision capabilities
   - Add table refinement with AI
   - Image description generation
   - **Files:** `edgequake-pdf/src/extractor.rs`

2. **Performance Optimization** 📋 PLANNED
   - Concurrent AI processing
   - Memory usage optimization
   - Cost monitoring
   - **Files:** `edgequake-pdf/src/config.rs`

3. **Error Handling & Fallbacks** 📋 PLANNED
   - Graceful degradation strategies
   - Comprehensive error types
   - Recovery mechanisms
   - **Files:** `edgequake-pdf/src/error.rs`

### Phase 3: CLI Tool Development (Week 4) 🟡 HIGH PRIORITY
**Goal:** Complete standalone CLI tool for end users

**Tasks:**
1. **CLI Crate Setup** 📋 PLANNED
   - Create `edgequake-pdf-cli` binary crate
   - Implement clap-based command structure
   - Add to workspace
   - **Files:** `edgequake/crates/edgequake-pdf-cli/src/main.rs`

2. **Core Commands** 📋 PLANNED
   - `convert` subcommand with full options
   - `info` for PDF analysis
   - `batch` for multi-file processing
   - **Files:** `edgequake/crates/edgequake-pdf-cli/src/commands/`

3. **User Experience** 📋 PLANNED
   - Progress bars and colored output
   - Configuration file support
   - Comprehensive help system
   - **Files:** `edgequake/crates/edgequake-pdf-cli/src/config.rs`

### Phase 4: Advanced Features (Week 5) 🟠 MEDIUM PRIORITY
**Goal:** Handle complex document layouts and edge cases

**Tasks:**
1. **Layout Processing** 📋 PLANNED
   - Multi-column detection
   - Complex table structures
   - Figure and caption handling
   - **Files:** `edgequake-pdf/src/layout.rs`

2. **OCR Integration** 📋 PLANNED
   - Scanned document support
   - Cloud OCR fallbacks
   - Quality validation
   - **Files:** `edgequake-pdf/src/ocr.rs`

3. **Cross-Platform Testing** 📋 PLANNED
   - macOS, Linux, Windows validation
   - Container deployment
   - Performance benchmarking
   - **Files:** `edgequake/crates/edgequake-pdf/tests/`

### Phase 5: Production Readiness (Week 6) 🟢 LOW PRIORITY
**Goal:** Enterprise-grade reliability and monitoring

**Tasks:**
1. **Comprehensive Testing** 📋 PLANNED
   - 90%+ test coverage
   - E2E test suite
   - Performance regression tests
   - **Files:** `edgequake/crates/edgequake-pdf/tests/`

2. **Monitoring & Observability** 📋 PLANNED
   - Cost tracking and limits
   - Performance metrics
   - Error reporting
   - **Files:** `edgequake-pdf/src/metrics.rs`

3. **Documentation** 📋 PLANNED
   - API documentation
   - CLI usage examples
   - Production deployment guide
   - **Files:** `edgequake/crates/edgequake-pdf/README.md`

### Phase 6: Distribution & Packaging (Week 7) 🟢 LOW PRIORITY
**Goal:** Make the tool accessible to end users

**Tasks:**
1. **Binary Distribution** 📋 PLANNED
   - GitHub releases with pre-built binaries
   - Cross-platform builds (x86_64, ARM)
   - Installation scripts
   - **Files:** `.github/workflows/release.yml`

2. **Package Manager Support** 📋 PLANNED
   - Cargo install support
   - Homebrew formula
   - Docker container
   - **Files:** `Dockerfile`, `edgequake-pdf-cli/install.sh`

3. **Final Validation** 📋 PLANNED
   - Production deployment testing
   - User acceptance testing
   - Performance validation
   - **Files:** `edgequake/crates/edgequake-pdf/tests/e2e/`

---

## Technical Architecture

### Core Components

```rust
// Main library interface
pub struct PdfExtractor<P: LLMProvider> {
    config: PdfExtractorConfig,
    provider: Arc<P>,
}

impl<P: LLMProvider> PdfExtractor<P> {
    pub async fn extract_to_markdown(&self, pdf_bytes: &[u8]) -> Result<String> {
        // 1. Parse PDF with pdf_oxide
        // 2. Extract text, tables, images
        // 3. Enhance with AI processing
        // 4. Generate structured Markdown
    }
}
```

### LLM Provider Integration

**Current State:** Basic text-only providers  
**Required Extension:** Vision capabilities for image processing

```rust
// New VisionProvider trait
#[async_trait]
pub trait VisionProvider: LLMProvider {
    async fn process_with_images(
        &self,
        messages: Vec<ChatMessage>,
        images: Vec<ImageData>,
    ) -> Result<ChatResponse>;
}
```

### CLI Architecture

```
edgequake/crates/
├── edgequake-pdf/          # Library crate
│   ├── src/
│   │   ├── lib.rs          # Main API
│   │   ├── extractor.rs    # Core extraction logic
│   │   ├── config.rs       # Configuration structs
│   │   └── error.rs        # Error types
│   └── Cargo.toml
└── edgequake-pdf-cli/      # CLI binary crate
    ├── src/
    │   ├── main.rs         # CLI entry point
    │   └── commands/       # Subcommand implementations
    │       ├── convert.rs
    │       ├── info.rs
    │       ├── batch.rs
    │       └── config.rs
    └── Cargo.toml
```

---

## Dependencies & Configuration

### Cargo.toml (Library)

```toml
[package]
name = "edgequake-pdf"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "PDF to Markdown extraction with AI enhancement for EdgeQuake"

[dependencies]
# Workspace dependencies
edgequake-llm.workspace = true
async-trait.workspace = true
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
anyhow.workspace = true
futures.workspace = true

# PDF processing
pdf_oxide = "0.2.2"

# Image processing
image = "0.25"
base64 = "0.22"

# Optional: Local OCR fallback
tesseract-rs = "0.4"

# CLI (optional, for standalone usage)
clap = { version = "4.5", features = ["derive"] }
```

### CLI Cargo.toml

```toml
[package]
name = "edgequake-pdf-cli"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Command-line interface for PDF to Markdown extraction"

[[bin]]
name = "edgequake-pdf-cli"
path = "src/main.rs"

[dependencies]
edgequake-pdf.workspace = true
clap = { version = "4.5", features = ["derive", "color", "suggestions"] }
anyhow.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tokio-util = { version = "0.7", features = ["io"] }
indicatif = "0.17"
dialoguer = "0.11"
```

---

## CLI Tool Specification

### Command Structure

```bash
edgequake-pdf-cli [GLOBAL_OPTIONS] <SUBCOMMAND> [SUBCOMMAND_OPTIONS]
```

### Global Options
- `-v, --verbose`: Enable verbose logging
- `-q, --quiet`: Suppress output
- `--config <FILE>`: Configuration file path
- `--provider <TYPE>`: LLM provider (openai, mock)

### Subcommands

#### Convert (Primary)
```bash
edgequake-pdf-cli convert [OPTIONS] <INPUT> [OUTPUT]
```
**Key Options:**
- `--pages <RANGE>`: Page range (e.g., "1-5,8,10-12")
- `--mode <MODE>`: Processing mode (fast, balanced, accurate)
- `--vision <BOOL>`: Enable vision processing
- `--max-concurrency <N>`: Concurrent AI requests
- `--progress`: Show progress bar

#### Info (Analysis)
```bash
edgequake-pdf-cli info [OPTIONS] <INPUT>
```
**Options:**
- `--metadata`: Document metadata
- `--pages`: Page information
- `--structure`: Document structure analysis
- `--images`: Image detection
- `--tables`: Table detection

#### Batch (Multi-file)
```bash
edgequake-pdf-cli batch [OPTIONS] <INPUT_DIR> <OUTPUT_DIR>
```
**Options:**
- `--pattern <PATTERN>`: File pattern matching
- `--recursive`: Process subdirectories
- `--parallel <N>`: Parallel processing count
- `--continue-on-error`: Continue after failures

#### Config (Management)
```bash
edgequake-pdf-cli config [SUBCOMMAND]
```
**Subcommands:**
- `init`: Create default config
- `show`: Display current config
- `set <KEY> <VALUE>`: Set config value
- `get <KEY>`: Get config value

---

## Usage Examples

### Library Usage
```rust
use edgequake_pdf::{PdfExtractor, PdfExtractorConfig};
use edgequake_llm::providers::openai::OpenAIProvider;

#[tokio::main]
async fn main() -> Result<()> {
    let llm_provider = Arc::new(OpenAIProvider::new(env!("OPENAI_API_KEY")));
    let config = PdfExtractorConfig::new(llm_provider);
    let extractor = PdfExtractor::with_config(config);

    let pdf_bytes = std::fs::read("document.pdf")?;
    let markdown = extractor.extract_to_markdown(&pdf_bytes).await?;

    println!("{}", markdown);
    Ok(())
}
```

### CLI Usage
```bash
# Basic conversion
edgequake-pdf-cli convert document.pdf output.md

# Advanced processing
edgequake-pdf-cli convert \
  --mode accurate \
  --vision true \
  --pages 1-10 \
  --progress \
  document.pdf

# Batch processing
edgequake-pdf-cli batch ./pdfs ./output --recursive --parallel 4

# PDF analysis
edgequake-pdf-cli info --metadata --structure document.pdf
```

---

## Risk Assessment & Mitigations

### Critical Risks 🔴

| Risk | Probability | Impact | Mitigation | Status |
|------|-------------|--------|------------|--------|
| pdf_oxide API Incompatible | High | Critical | Fallback to lopdf + pdf-extract | 🔄 Research Required |
| Vision Support Missing | High | Critical | Extend LLM provider traits | 📋 Implementation Planned |
| Performance Target Miss | Medium | High | Benchmarking + optimization | 📋 Planned |
| AI Cost Overrun | Medium | Medium | Cost limits + monitoring | 📋 Planned |

### Go/No-Go Decision Points

- **Day 2**: pdf_oxide API validation complete
- **Day 7**: Vision trait implementation working
- **Day 14**: Basic PDF extraction + AI enhancement functional
- **Day 21**: Performance benchmarks meet targets
- **Day 35**: Full feature set working, cross-platform tested

---

## Success Metrics

### Technical Metrics
- ✅ PDF text extraction accuracy: >95% for native PDFs
- ✅ Vision-enhanced processing: >80% for scanned/complex PDFs
- ✅ Performance: <15s for 100-page PDFs
- ✅ Memory usage: <500MB for typical documents
- ✅ API reliability: >99.5% with fallbacks

### Quality Metrics
- ✅ Test coverage: >90% unit, >80% integration, >70% E2E
- ✅ Error handling: Graceful degradation for all failure modes
- ✅ Documentation: Complete API docs and usage examples
- ✅ Cross-platform: Consistent behavior on Linux/macOS/Windows

### Operational Metrics
- ✅ Cost efficiency: <$0.10 per 100-page document
- ✅ Scalability: Handle concurrent requests without degradation
- ✅ Maintainability: Clear code structure and documentation

---

## Implementation Roadmap

### Immediate Actions (Next 24-48 hours)
1. **API Research**: Validate `pdf_oxide` compatibility and test fallbacks
2. **Crate Setup**: Create `edgequake-pdf` directory structure
3. **Vision Extension**: Begin `VisionProvider` trait implementation

### Week 1: Foundation
- Complete pdf_oxide validation and fallback implementation
- Extend LLM providers with vision support
- Create working text extraction prototype

### Week 2-3: Core Features
- AI enhancement pipeline with vision integration
- Performance optimization and benchmarking
- Comprehensive error handling

### Week 4: CLI Tool
- Complete CLI crate with all subcommands
- User experience polish (progress bars, colors, config)
- Cross-platform testing

### Week 5-7: Production Ready
- Advanced features (layout, OCR)
- Comprehensive testing and documentation
- Distribution and packaging

---

## Files to Create/Modify

### New Files
- `edgequake/crates/edgequake-pdf/Cargo.toml`
- `edgequake/crates/edgequake-pdf/src/lib.rs`
- `edgequake/crates/edgequake-pdf/src/extractor.rs`
- `edgequake/crates/edgequake-pdf/src/config.rs`
- `edgequake/crates/edgequake-pdf/src/error.rs`
- `edgequake/crates/edgequake-pdf-cli/Cargo.toml`
- `edgequake/crates/edgequake-pdf-cli/src/main.rs`
- `edgequake/crates/edgequake-pdf-cli/src/commands/`

### Modified Files
- `edgequake/Cargo.toml` (add new crates to workspace)
- `edgequake-llm/src/traits.rs` (add VisionProvider trait)
- `edgequake-llm/src/providers/openai.rs` (implement vision)
- `edgequake-llm/src/providers/mock.rs` (mock vision)

---

## Testing Strategy

### Unit Tests
- Component isolation testing
- Mock LLM provider usage
- Error condition validation
- **Location:** `edgequake/crates/edgequake-pdf/tests/unit/`

### Integration Tests
- Full pipeline testing with real providers
- Performance benchmarking
- Cross-platform validation
- **Location:** `edgequake/crates/edgequake-pdf/tests/integration/`

### E2E Tests
- CLI tool testing
- Real PDF processing
- Batch operation validation
- **Location:** `edgequake/crates/edgequake-pdf/tests/e2e/`

### Test Data
- Curated PDF test suite
- Various document types (native, scanned, complex)
- Performance benchmarking documents
- **Location:** `edgequake/crates/edgequake-pdf/test-data/`

---

## Verification Plan

### Manual Testing Steps
1. **Basic Functionality**: Convert simple PDF to Markdown
2. **AI Enhancement**: Process document with tables and images
3. **CLI Tool**: Test all subcommands with various options
4. **Batch Processing**: Process multiple PDFs with different settings
5. **Error Handling**: Test fallback mechanisms and error recovery
6. **Performance**: Benchmark against target metrics
7. **Cross-Platform**: Validate on all supported platforms

### Automated Verification
- **CI Pipeline**: Full test suite on all platforms
- **Performance Regression**: Automated benchmarking
- **Integration Tests**: End-to-end pipeline validation
- **Code Quality**: Clippy and rustfmt checks

---

## Conclusion

This specification provides a comprehensive, actionable plan for implementing a production-ready PDF-to-Markdown extraction crate within the EdgeQuake ecosystem. The phased approach ensures quality implementation with proper testing and documentation.

**Key Success Factors:**
1. **Early API validation** of PDF parsing libraries
2. **Incremental development** with working prototypes
3. **Comprehensive testing** at all levels
4. **User-focused design** with both library and CLI interfaces
5. **Performance-first architecture** with cost awareness

The implementation will deliver a valuable addition to EdgeQuake's document processing capabilities, enabling users to transform PDF documents into structured Markdown with AI-enhanced accuracy and layout preservation.

```toml
[package]
name = "edgequake-pdf"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "PDF to Markdown extraction with AI enhancement for EdgeQuake"

[dependencies]
# Workspace dependencies
edgequake-llm.workspace = true
async-trait.workspace = true
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
anyhow.workspace = true
futures.workspace = true

# PDF processing
pdf_oxide = "0.2.2"  # Latest stable as of Dec 2025; update to v0.4+ for full table support

# Image processing
image = "0.25"  # For image extraction and basic processing (e.g., resizing for AI input)
base64 = "0.22"  # For encoding images for vision API

# Optional: Local OCR fallback
tesseract-rs = "0.4"  # Optional: Local OCR fallback for scanned PDFs if AI OCR is insufficient

# CLI (optional, for standalone usage)
clap = { version = "4.5", features = ["derive"] }  # CLI interface

# CLI (optional, for standalone usage)
clap = { version = "4.5", features = ["derive"] }  # CLI interface
```

- `edgequake-llm`: Core LLM provider abstraction for AI interactions. Supports OpenAI and compatible providers.
- `pdf_oxide`: Core for PDF parsing, text extraction, layout analysis (XY-Cut/DBSCAN), and partial Markdown conversion. As of v0.2.2 (2025), it handles text, images, and basic tables; v0.4.0 adds full table read/write.
- `image` and `base64`: For processing images extracted from PDFs for vision API calls.

Minimum Rust version: 1.78 (matches workspace).

### Architecture

The `edgequake-pdf` crate follows a modular, pluggable pipeline architecture that integrates with EdgeQuake's LLM provider system:

1. **Input Handling**: Load PDF file into `pdf_oxide::Document`.
2. **Extraction Phase**: Use `pdf_oxide` to parse pages, extracting text blocks, tables, images, and metadata.
3. **AI Enhancement Phase**: Batch elements (e.g., images, ambiguous tables) and send to the configured LLM provider for processing.
4. **Assembly Phase**: Combine extracts into Markdown, inserting AI-generated descriptions, refined tables, and structural markers.
5. **Output**: Serialize to Markdown string/file.

High-level flow (in async Rust):

```rust
use edgequake_llm::traits::{LLMProvider, ChatMessage, ChatRole};
use edgequake_pdf::{PdfExtractor, PdfExtractorConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Get LLM provider from EdgeQuake's provider factory
    let llm_provider = Arc::new(/* EdgeQuake LLM provider instance */);
    
    let config = PdfExtractorConfig::default()
        .with_llm_provider(llm_provider)
        .with_ocr_threshold(0.8);
    
    let extractor = PdfExtractor::new(config);
    let pdf_bytes = std::fs::read("document.pdf")?;
    let markdown = extractor.extract_to_markdown(&pdf_bytes).await?;
    
    // Write to file or return
    std::fs::write("output.md", &markdown)?;
    Ok(())
}
```

- **Pluggability**: The crate accepts any `Arc<dyn LLMProvider>` implementation, allowing use of different providers (OpenAI, Mock, etc.).
- **Configurability**: Via `PdfExtractorConfig`:
  - LLM provider instance
  - OCR confidence threshold
  - Maximum pages to process
  - AI model selection
  - Output format options

#### Error Handling
- Use `anyhow` for propagation.
- Fallbacks: If AI fails, use `pdf_oxide`'s rule-based output.
- Logging: Integrate `tracing` for debug info.

### Handling Specific Elements

The tool processes PDFs page-by-page for efficiency, maintaining global context (e.g., running headers/footers).

#### 1. Text Extraction
- **pdf_oxide Role**: Extracts text with layout-aware reading order (XY-Cut algorithm), handles ligatures, hyphenation, and fonts. Outputs as structured blocks (e.g., paragraphs, headings via font size heuristics).
- **AI Enhancement**: For scanned/low-confidence text, send page images to AI vision endpoint for OCR and rephrasing. Prompt: "Extract and clean all text from this image, preserving order and structure."
- **Output in Markdown**: Convert to # Headers, **bold**, *italic*, lists, etc. Preserve paragraphs.

#### 2. Tables
- **pdf_oxide Role**: In v0.4.0+, uses grid detection for extraction. For earlier versions, fallback to text-block clustering.
- **AI Enhancement**: If table is complex/misparsed (e.g., merged cells), convert to image and send to AI. Prompt: "Interpret this table image as Markdown. Include headers, rows, and handle spans accurately."
- **Output**: Render as Markdown tables (`| Col1 | Col2 |`). Add captions if detected nearby.

#### 3. Images and Figures
- **pdf_oxide Role**: Extracts embedded images (JPEG/PNG/TIFF) with positions. Detects figures via spatial clustering (DBSCAN).
- **AI Enhancement**: Base64-encode image and send to vision model. Prompt: "Describe this image/figure in detail, including key elements, colors, and context. Generate a concise alt-text and caption."
- **Output**: Insert as Markdown images: `![Figure Description](data:image/png;base64,<base64_data>)` or link to extracted file. Include AI-generated caption and number (e.g., "Figure 1: [AI Description]"). Auto-number figures sequentially.

#### 4. Figure Descriptions and Captions
- **pdf_oxide Role**: Identifies potential captions via proximity to images (e.g., text blocks below/above).
- **AI Enhancement**: Refine with context. Prompt: "Given this figure image and nearby text '[excerpt]', generate an accurate description and caption."
- **Output**: Append to image Markdown: `![Alt text](...)  
  *Figure X: AI-generated description.*`

#### 5. Page Numbers and Metadata
- **pdf_oxide Role**: Parses page metadata, detects headers/footers via repetition analysis.
- **AI Enhancement**: Minimal; use only if ambiguous (e.g., send footer image for confirmation).
- **Output**: Insert as footnotes or section breaks: `--- Page N ---`. Preserve in TOC if applicable.

#### 6. Overall Structure and Reading Order
- **pdf_oxide Role**: Global layout analysis ensures logical flow.
- **AI Enhancement**: For highly unstructured PDFs, send extracted text to AI for reorganization. Prompt: "Restructure this raw text into coherent Markdown, adding headings and sections where logical."
- **Edge Cases**:
  - Scanned PDFs: Auto-detect via `pdf_oxide`'s OCR flag; trigger full-page AI vision if needed.
  - Multi-column: Use `pdf_oxide`'s column detection; AI fallback for errors.
  - Math/Equations: Extract as images, describe via AI (prompt: "Convert this equation image to LaTeX Markdown").

### AI Integration Details

- **Provider Integration**: Uses EdgeQuake's `LLMProvider` trait for AI interactions. Supports all configured providers (OpenAI, Mock, etc.).
- **Vision Support**: Requires LLM providers to support vision capabilities. For providers without native vision support, the crate will need to extend the `LLMProvider` trait to include vision methods.
- **Endpoints Used**:
  - Chat Completions: For text refinements and image descriptions.
  - Vision capabilities: For images/tables (if supported by the provider).
- **Batching**: Collect elements (e.g., all images) and send in parallel via `tokio::join!` to minimize latency.
- **Prompt Engineering**:
  - System Prompt (global): "You are a PDF extraction assistant. Output only clean Markdown or JSON as requested. Be precise and preserve original meaning."
  - Per-Task Prompts: As above, tailored for determinism.
- **Rate Limiting/Retries**: Leverages EdgeQuake's existing rate limiting and retry mechanisms.
- **Cost Optimization**: Only invoke AI for non-confident extracts (e.g., if `pdf_oxide` confidence < threshold).
- **Response Parsing**: Expect JSON from AI (e.g., `{ "markdown": "...", "description": "..." }`); use `serde` to deserialize.

**Note on Vision Support**: The current `edgequake-llm` crate may need extension to support vision capabilities. The `LLMProvider` trait should be extended with vision methods:

```rust
#[async_trait]
pub trait VisionProvider: LLMProvider {
    async fn chat_with_images(
        &self,
        messages: &[ChatMessageWithImages],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse>;
}

#[derive(Debug, Clone)]
pub struct ChatMessageWithImages {
    pub role: ChatRole,
    pub content: Vec<ChatContent>,
}

#[derive(Debug, Clone)]
pub enum ChatContent {
    Text(String),
    Image { mime_type: String, data: Vec<u8> },
}
```

Example AI Call (using EdgeQuake LLM provider):

```rust
use edgequake_llm::traits::{LLMProvider, ChatMessage, ChatRole, CompletionOptions};

async fn describe_image(
    provider: &dyn LLMProvider,
    image_base64: &str
) -> Result<String> {
    let messages = vec![
        ChatMessage::system("You are a PDF extraction assistant. Describe this image for Markdown alt-text."),
        ChatMessage::user(format!("Analyze this image and provide a concise description:\n![Image](data:image/png;base64,{})", image_base64)),
    ];
    
    let options = CompletionOptions::default()
        .with_temperature(0.1); // Low temperature for consistency
    
    let response = provider.chat(&messages, Some(&options)).await?;
    Ok(response.content)
}
```

### Implementation Notes

- **Crate Structure**: Located at `edgequake/crates/edgequake-pdf/` with modules:
  - `lib.rs`: Main crate interface and configuration
  - `extractor.rs`: Core PDF processing logic using `pdf_oxide`
  - `ai_enhancer.rs`: AI interaction layer using `edgequake-llm`
  - `assembler.rs`: Markdown assembly and formatting
  - `error.rs`: Error types specific to PDF processing
- **Integration with EdgeQuake**: 
  - Added to workspace members in root `Cargo.toml`
  - Can be used by other EdgeQuake crates (e.g., `edgequake-pipeline` for document ingestion)
  - Follows EdgeQuake coding conventions: `tracing` for logging, `thiserror` for errors, async/await patterns
- **Testing**: Comprehensive test suite covering unit, integration, and e2e scenarios. Benchmark against pyzerox and other PDF extraction tools.
- **Performance**: `pdf_oxide` is ~50x faster than Python equivs; AI adds latency, so parallelize.
- **Extensions**: Add web support (Wasm) or server mode via Axum/Rocket.
- **Build/Run**: `cargo build --package edgequake-pdf`; Example usage in pipeline:

```rust
use edgequake_pdf::{PdfExtractor, PdfExtractorConfig};
use edgequake_llm::providers::openai::OpenAIProvider;

// In a pipeline context
let llm_provider = Arc::new(OpenAIProvider::new(env!("OPENAI_API_KEY")));
let config = PdfExtractorConfig::new(llm_provider);
let extractor = PdfExtractor::with_config(config);
let markdown = extractor.extract_to_markdown(pdf_bytes).await?;
```

## CLI Tool

The `edgequake-pdf` crate includes a command-line interface (CLI) tool for standalone PDF processing, enabling users to convert PDFs to Markdown without writing code. The CLI tool (`edgequake-pdf-cli`) provides a user-friendly interface to the library functionality with comprehensive options for configuration, output formatting, and processing modes.

### CLI Architecture

The CLI tool is implemented as a separate binary crate that depends on the `edgequake-pdf` library crate. This separation allows the library to be used programmatically while providing a standalone executable for command-line usage.

**Crate Structure**:
```
edgequake/crates/
├── edgequake-pdf/          # Library crate
│   ├── src/
│   │   ├── lib.rs
│   │   ├── extractor.rs
│   │   ├── config.rs
│   │   └── error.rs
│   └── Cargo.toml
└── edgequake-pdf-cli/      # CLI binary crate
    ├── src/
    │   └── main.rs
    └── Cargo.toml
```

### CLI Dependencies

The CLI crate adds the following dependencies to the base library dependencies:

```toml
[package]
name = "edgequake-pdf-cli"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Command-line interface for PDF to Markdown extraction"

[dependencies]
# Library dependency
edgequake-pdf.workspace = true

# CLI framework
clap = { version = "4.5", features = ["derive", "color", "suggestions"] }

# Additional utilities
anyhow.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# File I/O
tokio-util = { version = "0.7", features = ["io"] }

# Optional: Progress bars
indicatif = "0.17"

# Optional: Interactive mode
dialoguer = "0.11"
```

### Command Structure

The CLI follows a hierarchical command structure with global options and subcommands:

```bash
edgequake-pdf-cli [GLOBAL_OPTIONS] <SUBCOMMAND> [SUBCOMMAND_OPTIONS]
```

**Global Options**:
- `-v, --verbose`: Enable verbose logging (can be repeated for more verbosity)
- `-q, --quiet`: Suppress all output except errors
- `--log-level <LEVEL>`: Set logging level (error, warn, info, debug, trace)
- `--config <FILE>`: Path to configuration file (JSON/TOML)
- `--provider <PROVIDER>`: LLM provider to use (openai, mock) [default: auto-detect]

**Subcommands**:
- `convert`: Convert PDF to Markdown (default subcommand)
- `info`: Display PDF information and metadata
- `batch`: Process multiple PDFs in batch mode
- `config`: Manage configuration settings

### Convert Subcommand

The primary subcommand for PDF conversion with comprehensive options:

```bash
edgequake-pdf-cli convert [OPTIONS] <INPUT> [OUTPUT]
```

**Arguments**:
- `<INPUT>`: Path to input PDF file (required)
- `[OUTPUT]`: Path to output Markdown file (optional, defaults to stdout or auto-generated)

**Options**:
- `-f, --format <FORMAT>`: Output format (markdown, html, json) [default: markdown]
- `--pages <RANGE>`: Page range to process (e.g., "1-5,8,10-12") [default: all]
- `--mode <MODE>`: Processing mode (fast, balanced, accurate) [default: balanced]
- `--vision <BOOL>`: Enable/disable vision processing [default: auto]
- `--ocr <BOOL>`: Enable/disable OCR fallback [default: auto]
- `--table-mode <MODE>`: Table processing mode (rule-based, ai-enhanced, hybrid) [default: hybrid]
- `--image-mode <MODE>`: Image processing mode (ignore, describe, inline, reference) [default: describe]
- `--max-concurrency <N>`: Maximum concurrent AI requests [default: 5]
- `--timeout <SECONDS>`: Processing timeout per page [default: 30]
- `--cost-limit <DOLLARS>`: Maximum cost limit for AI processing [default: unlimited]
- `--progress`: Show progress bar during processing
- `--dry-run`: Show what would be processed without actually doing it

### Info Subcommand

Display detailed information about a PDF file:

```bash
edgequake-pdf-cli info [OPTIONS] <INPUT>
```

**Options**:
- `--metadata`: Show document metadata (title, author, creation date, etc.)
- `--pages`: Show page count and dimensions
- `--structure`: Analyze and display document structure
- `--images`: List all images with details
- `--tables`: Detect and preview tables
- `--text-stats`: Show text statistics (word count, language detection)

### Batch Subcommand

Process multiple PDFs with batch processing capabilities:

```bash
edgequake-pdf-cli batch [OPTIONS] <INPUT_DIR> <OUTPUT_DIR>
```

**Options**:
- `--pattern <PATTERN>`: File pattern to match (e.g., "*.pdf") [default: *.pdf]
- `--recursive`: Process subdirectories recursively
- `--parallel <N>`: Number of PDFs to process in parallel [default: 2]
- `--continue-on-error`: Continue processing other files if one fails
- `--output-template <TEMPLATE>`: Output filename template (e.g., "{name}.md")
- `--summary`: Generate batch processing summary report

### Configuration Management

The CLI supports configuration files for persistent settings:

```bash
edgequake-pdf-cli config [SUBCOMMAND]
```

**Subcommands**:
- `init`: Create default configuration file
- `show`: Display current configuration
- `set <KEY> <VALUE>`: Set configuration value
- `get <KEY>`: Get configuration value

**Configuration File Format (TOML)**:
```toml
[processing]
mode = "balanced"
vision_enabled = true
ocr_fallback = true
max_concurrency = 5
timeout_seconds = 30

[output]
format = "markdown"
include_page_numbers = true
preserve_layout = true

[ai]
provider = "openai"
model = "gpt-4o-mini"
temperature = 0.1
cost_limit = 1.0

[logging]
level = "info"
file = "edgequake-pdf.log"
```

### Usage Examples

**Basic PDF conversion**:
```bash
# Convert PDF to Markdown (output to stdout)
edgequake-pdf-cli convert document.pdf

# Convert with output file
edgequake-pdf-cli convert document.pdf output.md

# Convert specific pages
edgequake-pdf-cli convert --pages 1-5,10 document.pdf
```

**Advanced processing with custom settings**:
```bash
# High-accuracy mode with vision and progress bar
edgequake-pdf-cli convert \
  --mode accurate \
  --vision true \
  --progress \
  --max-concurrency 3 \
  document.pdf output.md

# Fast processing for simple documents
edgequake-pdf-cli convert \
  --mode fast \
  --vision false \
  --table-mode rule-based \
  simple.pdf
```

**Batch processing**:
```bash
# Process all PDFs in directory
edgequake-pdf-cli batch ./pdfs ./markdown --recursive --parallel 4

# Process with custom naming
edgequake-pdf-cli batch \
  ./input \
  ./output \
  --output-template "{name}_{date}.md" \
  --summary
```

**Information and analysis**:
```bash
# Get PDF metadata
edgequake-pdf-cli info --metadata document.pdf

# Analyze document structure
edgequake-pdf-cli info --structure --tables document.pdf
```

**Configuration management**:
```bash
# Initialize configuration
edgequake-pdf-cli config init

# Set API key
edgequake-pdf-cli config set ai.api_key "sk-your-key-here"

# Show current config
edgequake-pdf-cli config show
```

### CLI Implementation Details

**Main Entry Point** (`src/main.rs`):
```rust
use clap::{Parser, Subcommand};
use edgequake_pdf_cli::*;

#[derive(Parser)]
#[command(name = "edgequake-pdf-cli")]
#[command(about = "Convert PDFs to Markdown with AI enhancement")]
#[command(version, long_about = None)]
struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[arg(short, long)]
    quiet: bool,

    #[arg(long, value_name = "LEVEL")]
    log_level: Option<String>,

    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(long, value_name = "PROVIDER")]
    provider: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Convert(ConvertArgs),
    Info(InfoArgs),
    Batch(BatchArgs),
    Config(ConfigArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose, cli.quiet, cli.log_level)?;

    // Load configuration
    let config = load_config(cli.config)?;

    // Execute command
    match cli.command {
        Commands::Convert(args) => convert_command(args, config).await,
        Commands::Info(args) => info_command(args, config).await,
        Commands::Batch(args) => batch_command(args, config).await,
        Commands::Config(args) => config_command(args, config).await,
    }
}
```

**Error Handling & User Experience**:
- **Colored output**: Success (green), warnings (yellow), errors (red)
- **Progress indicators**: Progress bars for long operations
- **Interactive prompts**: Optional confirmation for destructive operations
- **Help system**: Comprehensive `--help` for all commands and options
- **Auto-completion**: Shell completion scripts generation

**Performance Considerations**:
- **Streaming I/O**: Large PDFs processed without loading entirely into memory
- **Concurrent processing**: Multiple pages/AI requests processed simultaneously
- **Resource monitoring**: Memory and API usage tracking
- **Graceful degradation**: Automatic fallback to simpler modes on resource constraints

**Distribution & Packaging**:
- **Cargo installation**: `cargo install edgequake-pdf-cli`
- **Pre-built binaries**: GitHub releases with cross-platform builds
- **Container images**: Docker images for isolated execution
- **Package managers**: Future support for `brew`, `apt`, `winget`

The CLI tool provides a complete user interface to the `edgequake-pdf` library, making PDF-to-Markdown conversion accessible to users who prefer command-line workflows over programmatic integration.

## Testing Strategy

The `edgequake-pdf` crate implements a comprehensive testing strategy with three levels of testing: unit tests, integration tests, and end-to-end tests. All tests follow EdgeQuake's testing conventions and use the mock LLM provider by default for CI/CD.

### Unit Tests

Unit tests focus on testing individual functions and methods in isolation, using mocks and stubs for dependencies.

#### Configuration Tests (`config.rs`)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PdfConfig::default();
        assert_eq!(config.ocr_threshold, 0.8);
        assert!(config.include_page_numbers);
        assert!(config.extract_images);
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = PdfConfig::new()
            .with_ocr_threshold(0.9)
            .with_max_pages(10)
            .with_page_numbers(false);

        assert_eq!(config.ocr_threshold, 0.9);
        assert_eq!(config.max_pages, Some(10));
        assert!(!config.include_page_numbers);
    }

    #[test]
    fn test_config_validation() {
        let config = PdfConfig::new().with_ocr_threshold(1.5);
        // Should clamp or reject invalid values
        assert!(config.ocr_threshold <= 1.0);
    }
}
```

#### Error Handling Tests (`error.rs`)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = PdfError::PdfParse("Invalid PDF format".to_string());
        assert!(error.to_string().contains("PDF parsing error"));
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let pdf_error: PdfError = io_error.into();
        assert!(matches!(pdf_error, PdfError::Io(_)));
    }
}
```

#### Extractor Logic Tests (`extractor.rs`)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::providers::mock::MockProvider;

    #[tokio::test]
    async fn test_extractor_creation() {
        let mock_provider = Arc::new(MockProvider::new());
        let extractor = PdfExtractor::new(mock_provider);
        assert_eq!(extractor.config.ocr_threshold, 0.8);
    }

    #[tokio::test]
    async fn test_should_enhance_with_ai() {
        let mock_provider = Arc::new(MockProvider::new());
        let extractor = PdfExtractor::new(mock_provider);

        // Short text should trigger AI enhancement
        assert!(extractor.should_enhance_with_ai("Short text"));

        // Long, clean text should not
        let long_text = "This is a long paragraph with normal text content that should not require AI enhancement.";
        assert!(!extractor.should_enhance_with_ai(long_text));
    }

    #[tokio::test]
    async fn test_empty_pdf_handling() {
        let mock_provider = Arc::new(MockProvider::new());
        let extractor = PdfExtractor::new(mock_provider);

        let result = extractor.extract_to_markdown(&[]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("PDF Extraction"));
    }
}
```

### Integration Tests

Integration tests verify that different components work together correctly, using real dependencies where possible but still relying on mocks for external services.

#### Test Directory Structure
```
edgequake-pdf/
├── tests/
│   ├── integration_tests.rs
│   └── fixtures/
│       ├── sample_text.pdf
│       ├── sample_scanned.pdf
│       ├── sample_tables.pdf
│       ├── sample_images.pdf
│       └── expected_outputs/
│           ├── sample_text.md
│           ├── sample_scanned.md
│           └── ...
```

#### PDF Processing Integration Tests
```rust
#[cfg(test)]
mod integration_tests {
    use edgequake_pdf::{PdfExtractor, PdfConfig};
    use edgequake_llm::providers::mock::MockProvider;
    use std::fs;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_text_pdf_processing() {
        let mock_provider = Arc::new(MockProvider::new());
        let config = PdfConfig::new()
            .with_ocr_threshold(0.5) // Lower threshold to trigger AI
            .with_page_numbers(true);

        let extractor = PdfExtractor::with_config(mock_provider, config);

        // Load test PDF
        let pdf_bytes = fs::read("tests/fixtures/sample_text.pdf").unwrap();
        let result = extractor.extract_to_markdown(&pdf_bytes).await;

        assert!(result.is_ok());
        let markdown = result.unwrap();

        // Verify basic structure
        assert!(markdown.contains("# PDF Extraction"));
        assert!(markdown.contains("OCR Threshold: 0.5"));
    }

    #[tokio::test]
    async fn test_configuration_persistence() {
        let mock_provider = Arc::new(MockProvider::new());
        let config = PdfConfig::new()
            .with_max_pages(5)
            .with_image_extraction(false);

        let extractor = PdfExtractor::with_config(mock_provider, config);

        // Verify config is applied
        assert_eq!(extractor.config.max_pages, Some(5));
        assert!(!extractor.config.extract_images);
    }

    #[tokio::test]
    async fn test_error_handling_with_invalid_pdf() {
        let mock_provider = Arc::new(MockProvider::new());
        let extractor = PdfExtractor::new(mock_provider);

        // Test with invalid PDF data
        let invalid_pdf = b"This is not a PDF file";
        let result = extractor.extract_to_markdown(invalid_pdf).await;

        // Should handle gracefully (placeholder implementation)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ai_enhancement_integration() {
        let mut mock_provider = MockProvider::new();

        // Configure mock to return enhanced text
        mock_provider.set_response("Enhanced text with better formatting");

        let mock_provider = Arc::new(mock_provider);
        let config = PdfConfig::new().with_ocr_threshold(0.0); // Force AI enhancement
        let extractor = PdfExtractor::with_config(mock_provider, config);

        let pdf_bytes = fs::read("tests/fixtures/needs_enhancement.pdf").unwrap_or_default();
        let result = extractor.extract_to_markdown(&pdf_bytes).await;

        assert!(result.is_ok());
        // Verify AI enhancement was applied
    }
}
```

#### LLM Provider Integration Tests
```rust
#[cfg(test)]
mod llm_integration_tests {
    use edgequake_llm::providers::{mock::MockProvider, openai::OpenAIProvider};
    use edgequake_pdf::{PdfExtractor, PdfConfig};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_with_mock_provider() {
        let mock_provider = Arc::new(MockProvider::new());
        let extractor = PdfExtractor::new(mock_provider);

        let pdf_bytes = b"dummy pdf content";
        let result = extractor.extract_to_markdown(pdf_bytes).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires OPENAI_API_KEY
    async fn test_with_openai_provider() {
        let api_key = std::env::var("OPENAI_API_KEY").unwrap();
        let openai_provider = Arc::new(OpenAIProvider::new(api_key));
        let extractor = PdfExtractor::new(openai_provider);

        let pdf_bytes = fs::read("tests/fixtures/real_test.pdf").unwrap();
        let result = extractor.extract_to_markdown(&pdf_bytes).await;

        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(!markdown.is_empty());
    }
}
```

### End-to-End Tests

E2E tests validate the complete PDF processing pipeline from input to output, testing real-world scenarios and performance characteristics.

#### E2E Test Structure
```rust
#[cfg(test)]
mod e2e_tests {
    use edgequake_pdf::{PdfExtractor, PdfConfig};
    use edgequake_llm::providers::mock::MockProvider;
    use std::{fs, path::Path, sync::Arc, time::Instant};

    struct TestCase {
        input_pdf: String,
        expected_markdown: String,
        config: PdfConfig,
        description: String,
    }

    impl TestCase {
        fn new(name: &str, config: PdfConfig) -> Self {
            Self {
                input_pdf: format!("tests/e2e/fixtures/{}.pdf", name),
                expected_markdown: format!("tests/e2e/expected/{}.md", name),
                config,
                description: name.to_string(),
            }
        }
    }

    #[tokio::test]
    async fn test_text_heavy_document() {
        let test_case = TestCase::new("academic_paper", PdfConfig::new());

        run_e2e_test(test_case).await;
    }

    #[tokio::test]
    async fn test_scanned_document() {
        let test_case = TestCase::new(
            "scanned_document",
            PdfConfig::new()
                .with_ocr_threshold(0.3) // Lower threshold for scanned docs
                .with_image_extraction(true)
        );

        run_e2e_test(test_case).await;
    }

    #[tokio::test]
    async fn test_table_heavy_document() {
        let test_case = TestCase::new(
            "financial_report",
            PdfConfig::new()
                .with_table_enhancement(true)
                .with_max_pages(20)
        );

        run_e2e_test(test_case).await;
    }

    #[tokio::test]
    async fn test_image_rich_document() {
        let test_case = TestCase::new(
            "presentation_slides",
            PdfConfig::new()
                .with_image_extraction(true)
                .with_extract_images(true)
        );

        run_e2e_test(test_case).await;
    }

    async fn run_e2e_test(test_case: TestCase) {
        println!("Running E2E test: {}", test_case.description);

        let start_time = Instant::now();

        // Setup
        let mock_provider = Arc::new(MockProvider::new());
        let extractor = PdfExtractor::with_config(mock_provider, test_case.config);

        // Load input
        let pdf_bytes = fs::read(&test_case.input_pdf)
            .expect(&format!("Failed to read PDF: {}", test_case.input_pdf));

        // Process
        let result = extractor.extract_to_markdown(&pdf_bytes).await
            .expect("PDF extraction failed");

        let duration = start_time.elapsed();
        println!("Processing time: {:?}", duration);

        // Verify output exists and has content
        assert!(!result.is_empty(), "Output should not be empty");
        assert!(result.contains("# PDF Extraction"), "Should contain header");

        // Performance check (adjust thresholds based on benchmarks)
        assert!(duration.as_secs() < 30, "Processing should complete within 30 seconds");

        // Load expected output for comparison (if available)
        if Path::new(&test_case.expected_markdown).exists() {
            let expected = fs::read_to_string(&test_case.expected_markdown)
                .expect("Failed to read expected output");

            // Basic structure validation
            assert!(result.lines().count() > 5, "Output should have substantial content");

            // TODO: Add more sophisticated output validation
            // e.g., check for specific markdown elements, structure preservation
        }
    }

    #[tokio::test]
    async fn test_performance_regression() {
        let mock_provider = Arc::new(MockProvider::new());
        let extractor = PdfExtractor::new(mock_provider);

        // Load a standard test PDF
        let pdf_bytes = fs::read("tests/e2e/fixtures/benchmark.pdf")
            .unwrap_or_else(|_| b"dummy content".to_vec());

        let mut times = Vec::new();

        // Run multiple times for statistical significance
        for _ in 0..5 {
            let start = Instant::now();
            let _ = extractor.extract_to_markdown(&pdf_bytes).await.unwrap();
            times.push(start.elapsed());
        }

        let avg_time = times.iter().sum::<std::time::Duration>() / times.len() as u32;

        // Assert performance is within acceptable bounds
        assert!(avg_time.as_millis() < 5000, "Average processing time should be < 5 seconds");

        println!("Average processing time: {:?}", avg_time);
    }

    #[tokio::test]
    async fn test_error_recovery() {
        let mock_provider = Arc::new(MockProvider::new());
        let extractor = PdfExtractor::new(mock_provider);

        // Test with various edge cases
        let edge_cases = vec![
            ("empty_pdf", b"" as &[u8]),
            ("corrupted_pdf", b"This is not a PDF"),
            ("very_large_pdf", &[0u8; 100 * 1024 * 1024]), // 100MB
        ];

        for (case_name, pdf_data) in edge_cases {
            println!("Testing error recovery: {}", case_name);

            let result = extractor.extract_to_markdown(pdf_data).await;

            // Should not panic, should return some result
            assert!(result.is_ok(), "Should handle {} gracefully", case_name);
        }
    }
}
```

#### Benchmark Tests
```rust
#[cfg(test)]
mod benchmark_tests {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    use edgequake_pdf::{PdfExtractor, PdfConfig};
    use edgequake_llm::providers::mock::MockProvider;
    use std::sync::Arc;

    fn bench_pdf_extraction(c: &mut Criterion) {
        let mock_provider = Arc::new(MockProvider::new());
        let extractor = PdfExtractor::new(mock_provider);

        // Small PDF benchmark
        let small_pdf = b"small pdf content";
        c.bench_function("extract_small_pdf", |b| {
            b.iter(|| {
                let _ = black_box(extractor.extract_to_markdown(small_pdf));
            })
        });

        // Large PDF benchmark (simulate)
        let large_pdf = vec![0u8; 10 * 1024 * 1024]; // 10MB
        c.bench_function("extract_large_pdf", |b| {
            b.iter(|| {
                let _ = black_box(extractor.extract_to_markdown(&large_pdf));
            })
        });
    }

    criterion_group!(benches, bench_pdf_extraction);
    criterion_main!(benches);
}
```

### Test Data and Fixtures

#### Test PDF Generation
```bash
# Generate test PDFs using various tools
# Text-based PDF
echo "This is a test document with some text content." | ps2pdf - test_text.pdf

# Scanned document simulation (image-based PDF)
convert -size 800x600 xc:white -pointsize 12 -annotate +10+30 'Scanned Document Content' test_scanned.pdf

# Table-heavy PDF
# Use pandoc or other tools to generate PDFs with tables
```

#### CI/CD Integration
```yaml
# .github/workflows/test.yml
name: Test
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install dependencies
        run: sudo apt-get install -y poppler-utils tesseract-ocr
      - name: Run tests
        run: cargo test --package edgequake-pdf
      - name: Run integration tests
        run: cargo test --package edgequake-pdf --test integration_tests
      - name: Run benchmarks
        run: cargo bench --package edgequake-pdf
```

### Test Coverage Goals

- **Unit Tests**: >90% coverage of individual functions
- **Integration Tests**: Cover all major component interactions
- **E2E Tests**: Cover real-world usage scenarios
- **Performance Tests**: Ensure <10s processing for 100-page PDFs
- **Error Handling**: Test all error paths and recovery mechanisms

### Running Tests

```bash
# Run all tests
cargo test --package edgequake-pdf

# Run specific test types
cargo test --package edgequake-pdf unit
cargo test --package edgequake-pdf integration_tests
cargo test --package edgequake-pdf e2e_tests

# Run with real LLM provider (requires API keys)
OPENAI_API_KEY=sk-... cargo test --package edgequake-pdf -- --ignored

# Run benchmarks
cargo bench --package edgequake-pdf

# Generate coverage report
cargo tarpaulin --package edgequake-pdf --out Html
```

## Roadblocks and Mitigations

### 🚧 **Critical Roadblocks**

#### 1. **pdf_oxide API Compatibility** 🔴
**Issue**: The specification assumes `pdf_oxide` v0.2.2 API, but the actual API may differ significantly. The code examples use `Document::load_from_bytes()` and `page.extract_text()` which may not exist or work as expected.

**Impact**: Core PDF parsing functionality may fail to compile or work incorrectly.

**Detailed Mitigation Strategy**:

**Phase 1: API Research & Validation (Day 1)**
```rust
// Create a minimal test to verify pdf_oxide API
#[cfg(test)]
mod pdf_oxide_validation {
    use std::fs;

    #[test]
    fn test_pdf_oxide_basic_functionality() {
        // Load a simple test PDF
        let pdf_bytes = fs::read("tests/fixtures/simple.pdf")
            .expect("Test PDF not found");

        // Test actual pdf_oxide API - UPDATE THIS BASED ON REAL API
        match pdf_oxide::Document::load(&pdf_bytes) {
            Ok(doc) => {
                println!("PDF loaded successfully. Pages: {}", doc.pages().len());
                // Test text extraction
                if let Some(page) = doc.pages().first() {
                    match page.extract_text() {
                        Ok(text) => println!("Text extracted: {}", text.len()),
                        Err(e) => println!("Text extraction failed: {}", e),
                    }
                }
            }
            Err(e) => panic!("PDF loading failed: {}", e),
        }
    }
}
```

**Phase 2: Fallback Implementation (Day 2-3)**
If `pdf_oxide` proves incompatible, implement fallback using `lopdf`:

```rust
// Alternative implementation using lopdf
use lopdf::Document;

pub struct PdfParser {
    document: Document,
}

impl PdfParser {
    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self> {
        let document = Document::load_from(bytes)?;
        Ok(Self { document })
    }

    pub fn extract_text(&self, page_num: u32) -> Result<String> {
        let page = self.document.get_page(page_num)?;
        let text = self.document.extract_text(&[page_num])?;
        Ok(text)
    }

    pub fn page_count(&self) -> usize {
        self.document.get_pages().len()
    }
}
```

**Phase 3: Wrapper Abstraction (Day 4)**
Create a trait-based abstraction to allow switching PDF libraries:

```rust
#[async_trait]
pub trait PdfParser: Send + Sync {
    async fn load_from_bytes(bytes: &[u8]) -> Result<Box<dyn PdfParser>>;
    fn extract_text(&self, page_num: usize) -> Result<String>;
    fn page_count(&self) -> usize;
    fn extract_images(&self, page_num: usize) -> Result<Vec<PdfImage>>;
}

pub struct PdfImage {
    pub data: Vec<u8>,
    pub format: ImageFormat,
    pub bounds: Rect,
}
```

**Contingency Plan**: If no suitable Rust PDF library exists, consider:
- Using Python libraries via `pyo3` (embed Python runtime)
- External PDF-to-text conversion tools
- Web service integration for PDF processing

#### 2. **Vision Support in edgequake-llm** 🔴
**Issue**: Current `edgequake-llm` traits do not include vision/image processing capabilities. The specification assumes vision support exists or can be easily added.

**Impact**: Cannot process scanned PDFs, images, tables, or complex layouts that require visual analysis.

**Detailed Mitigation Strategy**:

**Phase 1: Extend LLMProvider Trait (Day 1-2)**
Add vision capabilities to the existing trait system:

```rust
// In edgequake-llm/src/traits.rs

/// Extended message content for vision support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatContent {
    Text(String),
    Image { mime_type: String, data: Vec<u8> },
}

/// Vision-enabled chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionChatMessage {
    pub role: ChatRole,
    pub content: Vec<ChatContent>,
}

/// Vision provider trait
#[async_trait]
pub trait VisionProvider: Send + Sync {
    /// Get the name of this provider
    fn name(&self) -> &str;

    /// Check if vision is supported
    fn supports_vision(&self) -> bool { true }

    /// Generate a completion with vision content
    async fn chat_with_vision(
        &self,
        messages: &[VisionChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse>;

    /// Generate embedding for image
    async fn embed_image(&self, image_data: &[u8], mime_type: &str) -> Result<Vec<f32>> {
        Err(LlmError::NotSupported("Image embedding not supported".to_string()))
    }
}

// Extend existing LLMProvider to optionally support vision
#[async_trait]
pub trait LLMProvider: Send + Sync {
    // ... existing methods ...

    /// Check if this provider supports vision
    fn supports_vision(&self) -> bool { false }

    /// Cast to vision provider if supported
    fn as_vision_provider(&self) -> Option<&dyn VisionProvider> { None }
}
```

**Phase 2: Update OpenAI Provider (Day 3-4)**
Implement vision support in the OpenAI provider:

```rust
// In edgequake-llm/src/providers/openai.rs

use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPart,
    ChatCompletionRequestMessageContentPartImage, ChatCompletionRequestMessageContentPartText,
};

impl VisionProvider for OpenAIProvider {
    fn supports_vision(&self) -> bool {
        // Vision supported for GPT-4o, GPT-4o-mini, etc.
        matches!(self.model.as_str(), "gpt-4o" | "gpt-4o-mini" | "gpt-4-vision-preview")
    }

    async fn chat_with_vision(
        &self,
        messages: &[VisionChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        let mut openai_messages = Vec::new();

        for msg in messages {
            let content_parts: Vec<ChatCompletionRequestMessageContentPart> = msg.content
                .iter()
                .map(|content| match content {
                    ChatContent::Text(text) => {
                        ChatCompletionRequestMessageContentPart::Text(
                            ChatCompletionRequestMessageContentPartText { text: text.clone() }
                        )
                    }
                    ChatContent::Image { mime_type, data } => {
                        let base64_data = base64::encode(data);
                        let image_url = format!("data:{};base64,{}", mime_type, base64_data);

                        ChatCompletionRequestMessageContentPart::Image(
                            ChatCompletionRequestMessageContentPartImage {
                                r#type: "image_url".to_string(),
                                image_url: async_openai::types::chat::ImageUrl {
                                    url: image_url,
                                    detail: Some("high".to_string()), // or "low" for cost optimization
                                },
                            }
                        )
                    }
                })
                .collect();

            let openai_msg = match msg.role {
                ChatRole::User => ChatCompletionRequestUserMessageArgs::default()
                    .content(content_parts)
                    .build()?,
                ChatRole::System => ChatCompletionRequestSystemMessageArgs::default()
                    .content(content_parts.iter()
                        .filter_map(|part| match part {
                            ChatCompletionRequestMessageContentPart::Text(text_part) => Some(text_part.text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" "))
                    .build()?,
                ChatRole::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(content_parts.iter()
                        .filter_map(|part| match part {
                            ChatCompletionRequestMessageContentPart::Text(text_part) => Some(text_part.text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" "))
                    .build()?,
                _ => return Err(LlmError::InvalidRequest("Unsupported role for vision".to_string())),
            };

            openai_messages.push(ChatCompletionRequestMessage::from(openai_msg));
        }

        // Create chat completion request
        let mut request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(openai_messages);

        if let Some(opts) = options {
            if let Some(temp) = opts.temperature {
                request = request.temperature(temp as f32);
            }
            if let Some(max_tokens) = opts.max_tokens {
                request = request.max_tokens(max_tokens as u32);
            }
        }

        let response = self.client.chat().create(request.build()?).await?;
        let content = response.choices[0].message.content.clone()
            .unwrap_or_default();

        Ok(LLMResponse::new(content, self.model.clone()))
    }
}

impl LLMProvider for OpenAIProvider {
    // ... existing implementation ...

    fn supports_vision(&self) -> bool {
        VisionProvider::supports_vision(self)
    }

    fn as_vision_provider(&self) -> Option<&dyn VisionProvider> {
        Some(self)
    }
}
```

**Phase 3: Update Mock Provider (Day 5)**
Add vision support to the mock provider for testing:

```rust
// In edgequake-llm/src/providers/mock.rs

impl VisionProvider for MockProvider {
    async fn chat_with_vision(
        &self,
        messages: &[VisionChatMessage],
        _options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        // Return mock responses based on input
        let has_images = messages.iter()
            .any(|msg| msg.content.iter()
                .any(|content| matches!(content, ChatContent::Image { .. })));

        let response_text = if has_images {
            "This appears to be an image showing [mock description]. The content includes text and visual elements.".to_string()
        } else {
            "Mock response for vision chat without images".to_string()
        };

        Ok(LLMResponse::new(response_text, "mock-vision"))
    }
}
```

**Phase 4: Integration in PDF Crate (Day 6-7)**
Use vision capabilities in the PDF extractor:

```rust
// In edgequake-pdf/src/extractor.rs

use edgequake_llm::traits::{VisionProvider, VisionChatMessage, ChatContent};

impl PdfExtractor {
    async fn process_image_with_vision(
        &self,
        image_data: &[u8],
        mime_type: &str,
        context: Option<&str>,
    ) -> Result<String> {
        let vision_provider = self.llm_provider.as_vision_provider()
            .ok_or_else(|| PdfError::AiProcessing("Vision not supported by LLM provider".to_string()))?;

        let mut content = vec![
            ChatContent::Text("Describe this image in detail for PDF extraction purposes. Focus on text content, diagrams, charts, and any relevant visual information.".to_string()),
            ChatContent::Image {
                mime_type: mime_type.to_string(),
                data: image_data.to_vec(),
            },
        ];

        if let Some(ctx) = context {
            content.insert(0, ChatContent::Text(format!("Context: {}", ctx)));
        }

        let messages = vec![VisionChatMessage {
            role: ChatRole::User,
            content,
        }];

        let options = CompletionOptions::default()
            .with_temperature(0.1);

        let response = vision_provider.chat_with_vision(&messages, Some(&options)).await
            .map_err(|e| PdfError::AiProcessing(e.to_string()))?;

        Ok(response.content)
    }
}
```

**Contingency Plan**: If vision trait extension proves too complex:
- Direct `async-openai` usage for vision-only operations
- Separate vision service/microservice
- Fallback to text-only processing with OCR

#### 3. **Performance Requirements** 🟡
**Issue**: Achieving <10s processing for 100-page PDFs with AI enhancement is ambitious. Each AI call adds 1-3s latency, and batching may not scale linearly.

**Impact**: May not meet performance targets, especially for large documents.

**Mitigation**:
- **Benchmark early**: Test with real PDFs and measure baseline performance
- **Optimize batching**: Implement smart batching strategies (group similar content types)
- **Caching**: Cache AI responses for repeated content
- **Progressive enhancement**: Allow configurable AI usage levels
- **Fallback**: Rule-based only mode for performance-critical scenarios

#### 4. **OCR Fallback Complexity** 🟡
**Issue**: `tesseract` dependency may have platform-specific issues, complex setup, and poor accuracy for complex layouts.

**Impact**: Scanned PDF processing may fail on certain platforms or produce poor results.

**Mitigation**:
- **Container deployment**: Use Docker containers with pre-configured Tesseract
- **Cloud OCR**: Integrate Google Cloud Vision or Azure OCR as fallback
- **Quality gates**: Only use OCR when AI vision fails
- **Testing**: Cross-platform testing (Linux, macOS, Windows)

### ⚠️ **Significant Challenges**

#### 5. **Complex Layout Handling** 🟡
**Issue**: Multi-column layouts, complex tables with merged cells, mathematical equations, and irregular document structures are difficult to parse accurately.

**Impact**: Poor conversion quality for academic papers, financial reports, and technical documents.

**Mitigation**:
- **Layout analysis**: Implement advanced layout detection algorithms
- **AI-first approach**: Use AI for layout understanding before rule-based parsing
- **Specialized handlers**: Create specific handlers for common document types
- **Quality metrics**: Implement accuracy scoring and fallback thresholds

#### 6. **Memory and Resource Management** 🟡
**Issue**: Large PDFs with images can consume significant memory. Concurrent AI processing may overwhelm system resources.

**Impact**: Out-of-memory errors, system instability, poor performance under load.

**Mitigation**:
- **Streaming processing**: Process pages individually rather than loading entire PDF
- **Resource limits**: Implement configurable memory limits and concurrent request caps
- **Progressive loading**: Load and process content on-demand
- **Monitoring**: Add resource usage tracking and alerts

#### 7. **Cost and Rate Limiting** 🟡
**Issue**: AI processing of large PDFs can be expensive and hit API rate limits. A 100-page document might require dozens of AI calls.

**Impact**: High operational costs, service unavailability during peak usage.

**Mitigation**:
- **Cost estimation**: Implement pre-processing cost calculation
- **Smart batching**: Minimize AI calls through intelligent content analysis
- **Caching**: Cache AI responses for similar content
- **Tiered processing**: Allow different quality levels (fast/rule-based vs. accurate/AI-enhanced)
- **Rate limiting**: Integrate with EdgeQuake's existing rate limiting infrastructure

#### 8. **Cross-Platform Compatibility** 🟠
**Issue**: PDF processing libraries and OCR tools may behave differently across platforms. File path handling, encoding issues, and system dependencies.

**Impact**: Inconsistent behavior across development and production environments.

**Mitigation**:
- **Container standardization**: Use Docker for consistent environments
- **Platform-specific testing**: Comprehensive CI/CD matrix testing
- **Abstraction layers**: Abstract file system operations
- **Fallback chains**: Multiple processing paths for different platforms

### 🔧 **Technical Debt and Maintenance**

#### 9. **Dependency Version Conflicts** 🟠
**Issue**: Integrating new dependencies (`pdf_oxide`, `tesseract`, `image`) with existing EdgeQuake workspace may cause version conflicts.

**Impact**: Compilation failures, security vulnerabilities from outdated dependencies.

**Mitigation**:
- **Dependency audit**: Regular security and compatibility audits
- **Workspace management**: Careful version pinning and conflict resolution
- **Isolated testing**: Test crate in isolation before workspace integration

#### 10. **Testing Infrastructure Complexity** 🟠
**Issue**: E2E testing requires diverse PDF test fixtures, mock AI responses, and performance benchmarking infrastructure.

**Impact**: Slow test execution, flaky tests, difficulty maintaining test coverage.

**Mitigation**:
- **Test fixtures**: Curated set of representative PDF documents
- **Mock ecosystem**: Comprehensive mocking for all external dependencies
- **Parallel execution**: Optimize test execution time
- **CI/CD optimization**: Separate fast unit tests from slow integration tests

### 📊 **Risk Assessment Matrix**

| Roadblock | Probability | Impact | Priority | Mitigation Status | Timeline |
|-----------|-------------|--------|----------|-------------------|----------|
| pdf_oxide API | High | Critical | 🔴 P0 | **Detailed fallback strategy ready** | 1-4 days |
| Vision Support | High | Critical | 🔴 P0 | **Complete implementation plan** | 1-7 days |
| Performance | Medium | High | 🟡 P1 | Benchmarking strategy defined | Ongoing |
| OCR Complexity | Medium | Medium | 🟡 P1 | Container/cloud fallbacks | 2-3 weeks |
| Layout Handling | Medium | High | 🟡 P1 | AI-first approach planned | 3-4 weeks |
| Resource Management | Low | Medium | 🟠 P2 | Monitoring framework | 4-6 weeks |
| Cost/Rate Limiting | Medium | Medium | 🟠 P2 | Cost controls implemented | 2-3 weeks |
| Cross-Platform | Low | Low | 🟢 P3 | Container standardization | 4-6 weeks |

### 🎯 **Implementation Priority**

**Phase 1 (Week 1): Foundation & Critical Mitigations** 🔴 Critical Path
1. **Day 1-2**: Research and validate `pdf_oxide` API, implement fallback if needed
2. **Day 3-4**: Extend `edgequake-llm` with `VisionProvider` trait
3. **Day 5-7**: Implement vision support in OpenAI provider and mock provider
4. **Day 7**: Create minimal PDF text extraction working prototype

**Phase 2 (Week 2-3): Core Features** 🟡 High Priority
1. **Day 8-10**: Implement AI enhancement pipeline with vision integration
2. **Day 11-14**: Add table and image processing with vision capabilities
3. **Day 15-17**: Performance optimization and benchmarking
4. **Day 17-21**: Comprehensive error handling and fallback mechanisms

**Phase 3 (Week 4-5): Polish & Advanced Features** 🟠 Medium Priority
1. **Day 22-25**: Advanced layout handling (multi-column, complex tables)
2. **Day 26-28**: Cross-platform testing and container deployment
3. **Day 29-31**: Cost optimization and rate limiting integration
4. **Day 32-35**: Documentation completion and examples

**Phase 4 (Week 6-7): Production Ready** 🟢 Low Priority
1. **Day 36-42**: E2E testing infrastructure and comprehensive test suite
2. **Day 43-45**: Performance monitoring and alerting
3. **Day 46-49**: Production deployment validation
4. **Day 50-52**: Final optimization and documentation

### 🚨 **Go/No-Go Decision Points**

- **Day 2**: pdf_oxide API research complete - proceed or switch to fallback
- **Day 7**: Vision trait implementation complete and tested
- **Day 14**: Basic PDF text extraction + AI enhancement working
- **Day 21**: Performance benchmarks meet requirements (or adjust targets)
- **Day 28**: Core feature set complete and integration tested
- **Day 35**: Advanced features working, cross-platform compatibility verified
- **Day 49**: Full E2E pipeline tested and production-ready

### 🔄 **Updated Contingency Plans**

**Primary Contingencies** (if critical roadblocks cannot be resolved):
1. **pdf_oxide Failure**: Switch to `lopdf` + Python embedding via `pyo3`
2. **Vision Complexity**: Implement separate vision microservice using direct OpenAI API
3. **Performance Issues**: Add "fast mode" (rule-based only) and "accurate mode" (AI-enhanced)
4. **Cost Concerns**: Implement usage quotas, cost estimation, and tiered service levels

**Secondary Contingencies**:
1. **OCR Fallback**: Use cloud OCR services (Google Cloud Vision, Azure OCR)
2. **Memory Issues**: Implement streaming processing and external image storage
3. **Platform Issues**: Docker containerization for consistent deployment
4. **Rate Limiting**: Implement intelligent queuing and request batching

### 📈 **Success Metrics**

**Technical Metrics**:
- ✅ PDF text extraction accuracy: >95% for native PDFs
- ✅ Vision-enhanced processing: >80% accuracy for scanned/complex PDFs
- ✅ Performance: <15s for 100-page PDFs (adjusted from <10s)
- ✅ Memory usage: <500MB for typical documents
- ✅ API reliability: >99.5% success rate with fallbacks

**Quality Metrics**:
- ✅ Test coverage: >90% unit, >80% integration, >70% E2E
- ✅ Error handling: Graceful degradation for all failure modes
- ✅ Cross-platform: Consistent behavior on Linux/macOS/Windows
- ✅ Documentation: Complete API docs and usage examples

**Operational Metrics**:
- ✅ Cost efficiency: <$0.10 per 100-page document
- ✅ Scalability: Handle concurrent requests without degradation
- ✅ Monitoring: Comprehensive logging and metrics
- ✅ Maintainability: Clear code structure and documentation

This roadblock analysis ensures realistic planning and risk mitigation for successful implementation of the `edgequake-pdf` crate.

This comprehensive testing strategy ensures the `edgequake-pdf` crate is reliable, performant, and maintains high quality across all usage scenarios.

---

## 🎯 **Next Steps & Implementation Roadmap**

### **Immediate Actions (Next 24-48 hours)**

1. **Research & Validation Phase**:
   - Investigate `pdf_oxide` API compatibility and current limitations
   - Review existing `edgequake-llm` provider implementations
   - Test basic PDF parsing with available libraries

2. **Foundation Setup**:
   - Create `edgequake/crates/edgequake-pdf/` directory structure
   - Set up basic Cargo.toml with dependencies
   - Implement placeholder structures for all major components

3. **Vision Support Extension**:
   - Extend `edgequake-llm` traits with vision capabilities
   - Implement vision support in OpenAI provider
   - Add vision mocking for testing

### **Week 1: Critical Path Implementation**

**Day 1-2: PDF Parsing Foundation**
- Complete `pdf_oxide` API research and validation
- Implement fallback PDF parsing strategy if needed
- Create basic text extraction functionality

**Day 3-4: Vision Provider Extension**
- Extend `LlmProvider` trait with `VisionProvider` capabilities
- Implement vision support in `OpenAiProvider`
- Update `MockProvider` with vision mocking

**Day 5-7: Integration & Testing**
- Integrate vision capabilities into PDF processing pipeline
- Create comprehensive unit tests for all components
- Validate workspace integration and compilation

### **Week 2-3: Core Feature Development**

**Day 8-14: AI Enhancement Pipeline**
- Implement AI-powered text enhancement
- Add table and image processing with vision
- Create error handling and fallback mechanisms

**Day 15-21: Performance & Optimization**
- Performance benchmarking and optimization
- Memory usage optimization
- Cost monitoring and rate limiting

### **Week 4-5: Advanced Features & Polish**

**Day 22-35: Advanced Capabilities**
- Multi-column layout handling
- Complex table processing
- Cross-platform compatibility testing
- Documentation and examples

### **Week 6-7: Production Readiness**

**Day 36-52: Finalization**
- Complete E2E testing infrastructure
- Production deployment validation
- Final performance tuning and monitoring

### **Recommended Development Workflow**

```bash
# 1. Start with foundation
cd edgequake/crates
cargo new edgequake-pdf --lib

# 2. Add to workspace
echo '    "crates/edgequake-pdf",' >> ../Cargo.toml

# 3. Implement vision extension first
cd edgequake-llm/src
# Add VisionProvider trait and implementations

# 4. Build PDF crate with fallbacks
cd ../edgequake-pdf
cargo build

# 5. Test integration
cd ../..
cargo test -p edgequake-pdf
```

### **Key Dependencies & Prerequisites**

**Required Before Starting**:
- ✅ EdgeQuake workspace setup and compilation
- ✅ OpenAI API key for testing (optional, mock provider available)
- ✅ Basic understanding of PDF structure and parsing
- ✅ Familiarity with async Rust patterns

**Tools & Environment**:
- Rust 1.78+ with Cargo
- PDF test files for validation
- Image processing libraries for vision features
- Benchmarking tools for performance testing

### **Success Validation Checklist**

- [ ] `cargo build` succeeds for entire workspace
- [ ] `cargo test` passes for `edgequake-pdf` crate
- [ ] Basic PDF text extraction works
- [ ] Vision provider extension compiles and integrates
- [ ] Mock provider supports vision operations
- [ ] Performance benchmarks meet adjusted targets (<15s for 100 pages)
- [ ] Error handling provides graceful degradation
- [ ] Documentation builds successfully

### **Final Notes**

This specification provides a comprehensive roadmap for implementing a production-ready PDF-to-Markdown transformation crate within the EdgeQuake ecosystem. The detailed mitigation strategies for critical roadblocks ensure realistic planning and high success probability.

**Key Success Factors**:
1. **Early validation** of PDF parsing APIs and vision capabilities
2. **Incremental implementation** with working prototypes at each phase
3. **Comprehensive testing** from unit to E2E levels
4. **Flexible architecture** supporting multiple fallback strategies
5. **Performance-first design** with cost and resource awareness

The crate will serve as a valuable addition to the EdgeQuake toolkit, enabling users to transform PDF documents into structured Markdown with AI-enhanced accuracy and layout preservation.