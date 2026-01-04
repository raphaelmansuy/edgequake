# EdgeQuake PDF Converter

A high-quality PDF to Markdown converter written in Rust, featuring advanced layout analysis, SOTA-level multi-column reading order, and optional LLM-powered image OCR.

## Quick Start

```bash
# Convert a PDF to Markdown (output: document.md)
edgequake-pdf document.pdf

# With vision AI for images (requires OPENAI_API_KEY)
edgequake-pdf document.pdf --vision
```

## Features

- ✅ **Text Extraction**: Character-level positioning with proper paragraph detection
- ✅ **Formatting**: Bold, italic, and bold-italic preserved as Markdown
- ✅ **Headers**: H1-H6 detection based on font size analysis
- ✅ **Multi-Column Layouts**: Industry-leading 2-column and 3-column support
- ✅ **Tables**: Automatic detection and Markdown table generation
- ✅ **Code Blocks**: Monospace font detection with triple-backtick fencing
- ✅ **Lists**: Bullet and numbered list detection
- ✅ **Multi-Page**: Seamless page extraction
- ✅ **Image OCR**: Optional LLM-powered image description (GPT-4o-mini)

## Installation

### Using Cargo (Recommended)

```bash
cargo install edgequake-pdf
```

### Using Homebrew (macOS)

```bash
# Add the tap
brew tap raphaelmansuy/edgequake

# Install edgequake-pdf
brew install edgequake-pdf
```

### From Source

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake/edgequake/crates/edgequake-pdf
cargo build --release

# Binary at: target/release/edgequake-pdf
```

## CLI Usage

### Basic Conversion

```bash
# Shorthand: input.pdf → input.md
edgequake-pdf document.pdf

# Explicit output path
edgequake-pdf convert -i document.pdf -o output.md

# Output to stdout
edgequake-pdf convert -i document.pdf --stdout
```

### Vision AI Mode

Enable LLM-powered image descriptions using OpenAI's GPT-4o-mini vision model:

```bash
# Set your API key
export OPENAI_API_KEY="sk-your-key-here"

# Convert with vision (describes images in markdown)
edgequake-pdf document.pdf --vision

# Use a specific vision model
edgequake-pdf document.pdf --vision --vision-model gpt-4o
```

### Options

```bash
# Convert first N pages only
edgequake-pdf document.pdf --max-pages 5

# Add page numbers to output
edgequake-pdf document.pdf --page-numbers

# JSON output format
edgequake-pdf convert -i document.pdf --format json --stdout

# Quiet mode (no progress output)
edgequake-pdf document.pdf -q

# Verbose mode (debug output)
edgequake-pdf document.pdf -v
```

### PDF Information

```bash
# Show PDF metadata
edgequake-pdf info -i document.pdf

# JSON format for scripting
edgequake-pdf info -i document.pdf --format json
```

Example output:

```
📋 PDF Information
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  File:       document.pdf
  Pages:      12
  Version:    1.7
  Size:       2,456,789 bytes (2.34 MB)
  Has images: yes (15 images)
```

### Pipe Mode

```bash
# Process from stdin
cat document.pdf | edgequake-pdf pipe > output.md

# With vision enabled
cat document.pdf | edgequake-pdf pipe --vision > output.md
```

## Library Usage

```rust
use edgequake_pdf::{extract_to_markdown, ExtractionOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Basic extraction
    let options = ExtractionOptions::default();
    let markdown = extract_to_markdown("document.pdf", &options)?;
    println!("{}", markdown);
    Ok(())
}
```

### With Custom Options

```rust
use edgequake_pdf::{extract_to_markdown, ExtractionOptions};

let options = ExtractionOptions {
    max_pages: Some(10),
    include_page_numbers: true,
    ..Default::default()
};

let markdown = extract_to_markdown("document.pdf", &options)?;
```

### With Vision AI

```rust
use edgequake_pdf::{Extractor, ImageOcrConfig};
use edgequake_llm::openai::OpenAIProvider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let provider = Arc::new(OpenAIProvider::new(api_key));

    let ocr_config = ImageOcrConfig {
        enabled: true,
        model: "gpt-4o-mini".to_string(),
    };

    let extractor = Extractor::new()
        .with_llm_provider(provider)
        .with_ocr_config(ocr_config);

    let pdf_bytes = std::fs::read("document.pdf")?;
    let document = extractor.extract_document(&pdf_bytes).await?;

    println!("{}", document.to_markdown());
    Ok(())
}
```

## Environment Variables

| Variable         | Description                          | Default               |
| ---------------- | ------------------------------------ | --------------------- |
| `OPENAI_API_KEY` | OpenAI API key for vision features   | (required for vision) |
| `RUST_LOG`       | Log level (debug, info, warn, error) | `info`                |

## Homebrew Tap Setup

To create your own Homebrew tap for distribution:

### 1. Create a Tap Repository

Create a new GitHub repository named `homebrew-<tap-name>`:

```bash
# Example: homebrew-edgequake
gh repo create raphaelmansuy/homebrew-edgequake --public
```

### 2. Create the Formula

Create `Formula/edgequake-pdf.rb`:

```ruby
class EdgequakePdf < Formula
  desc "High-quality PDF to Markdown converter with AI vision"
  homepage "https://github.com/raphaelmansuy/edgequake"
  url "https://github.com/raphaelmansuy/edgequake/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "YOUR_SHA256_HERE"
  license "MIT"

  depends_on "rust" => :build

  def install
    cd "edgequake/crates/edgequake-pdf" do
      system "cargo", "build", "--release", "--locked"
      bin.install "target/release/edgequake-pdf"
    end
  end

  test do
    system "#{bin}/edgequake-pdf", "--version"
  end
end
```

### 3. Users Install Via

```bash
brew tap raphaelmansuy/edgequake
brew install edgequake-pdf
```

## Key Algorithms

### Adaptive Column Detection

Uses histogram projection with an adaptive threshold:

- 15% of max bin count (not average) catches narrow gaps
- Works reliably for 2, 3, and 4+ column layouts

### Fill Ratio Heuristic

Discriminates tables from text columns:

- **fill_ratio** = avg_item_width / avg_column_width
- Tables: fill_ratio < 0.45 (short items like numbers)
- Text columns: fill_ratio > 0.6 (full sentences)

### Sequential Column Processing

Reading order algorithm processes columns left-to-right:

- No interleaving of column content
- Spanning elements (headers/footers) inserted at appropriate Y position

## Test Suite

Run all tests:

```bash
cargo test
```

Run CLI integration tests:

```bash
cargo test --test cli_tests
```

## Quality Assessment

**Overall Score**: 88/100 (APPROACHING SOTA)

| Category        | Score  |
| --------------- | ------ |
| Text Extraction | 95/100 |
| Formatting      | 95/100 |
| Two-Column      | 98/100 |
| Three-Column    | 90/100 |
| Tables          | 85/100 |
| Code Blocks     | 90/100 |

See [SOTA_ASSESSMENT.md](SOTA_ASSESSMENT.md) for full analysis.

## Known Limitations

- Merged table cells: Content extracted but not marked as spanning
- Math formulas: Subscripts/superscripts may appear fragmented
- Scanned PDFs: Requires `--vision` mode with OCR-capable model

## Documentation

**Comprehensive technical documentation** available in 5 focused documents:

- **[README_DOCS.md](README_DOCS.md)**: Documentation index and navigation guide
- **[ARCHITECTURE.md](ARCHITECTURE.md)**: System overview, module relationships (796 lines, 45+ code refs)
- **[PIPELINE.md](PIPELINE.md)**: 13-processor chain analysis (1177 lines, 65+ code refs)
- **[TABLE_DETECTION.md](TABLE_DETECTION.md)**: Lattice algorithm deep dive (1123 lines, 35+ code refs)
- **[EXTRACTION_ENGINE.md](EXTRACTION_ENGINE.md)**: Backend extraction internals (950+ lines, 55+ code refs)
- **[TEST_PROTOCOL.md](TEST_PROTOCOL.md)**: Testing methodology (400+ lines)

**Total**: 4,500+ lines of high-signal documentation with 200+ code references and 35+ ASCII diagrams.

---

## License

MIT
