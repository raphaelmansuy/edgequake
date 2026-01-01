# EdgeQuake PDF Converter

A high-quality PDF to Markdown converter written in Rust, featuring advanced layout analysis and SOTA-level multi-column reading order.

## Features

- ✅ **Text Extraction**: Character-level positioning with proper paragraph detection
- ✅ **Formatting**: Bold, italic, and bold-italic preserved as Markdown
- ✅ **Headers**: H1-H6 detection based on font size
- ✅ **Multi-Column Layouts**: Industry-leading 2-column and 3-column support
- ✅ **Tables**: Automatic detection and Markdown table generation
- ✅ **Code Blocks**: Monospace font detection with triple-backtick fencing
- ✅ **Lists**: Bullet and numbered list detection
- ✅ **Multi-Page**: Seamless page extraction

## Installation

```bash
cd edgequake/crates/edgequake-pdf
cargo build --release
```

## Usage

### CLI

```bash
# Convert PDF to Markdown
edgequake-pdf convert -i input.pdf

# Get PDF info
edgequake-pdf info -i input.pdf
```

### Library

```rust
use edgequake_pdf::{extract_to_markdown, ExtractionOptions};

let options = ExtractionOptions::default();
let markdown = extract_to_markdown("document.pdf", &options)?;
println!("{}", markdown);
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

30 test PDFs covering:
- Single/multi-column layouts
- Simple and complex tables
- Formatted text (bold/italic)
- Code blocks
- Lists (bullet/numbered)
- Multi-page documents
- Unicode and special characters

Run tests:
```bash
cargo test
```

## Quality Assessment

**Overall Score**: 88/100 (APPROACHING SOTA)

| Category | Score |
|----------|-------|
| Text Extraction | 95/100 |
| Formatting | 95/100 |
| Two-Column | 98/100 |
| Three-Column | 90/100 |
| Tables | 85/100 |
| Code Blocks | 90/100 |

See [SOTA_ASSESSMENT.md](SOTA_ASSESSMENT.md) for full analysis.

## Known Limitations

- Merged table cells: Content extracted but not marked as spanning
- Math formulas: Subscripts/superscripts may appear fragmented
- Scanned PDFs: Requires OCR preprocessing

## License

MIT
