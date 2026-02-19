# Product Manual

# EdgeQuake PDF Extractor - User Manual

## Table of Contents

1. Introduction
2. Installation
3. Quick Start
4. Advanced Configuration
5. Troubleshooting
6. API Reference

## 1. Introduction

The EdgeQuake PDF Extractor is a professional-grade tool for converting PDFs to Markdown format.

**Key Features:**

- **Multi-Column Support**: Handles 2-4 column layouts
- **Table Detection**: Automatic table extraction
- **Formatting Preservation**: Maintains bold, italic, and code formatting
- **Fast Processing**: Extract 50+ documents per second

**System Requirements:**

- macOS 10.15+ or Linux (Ubuntu 18.04+)
- 4GB RAM minimum
- 500MB disk space

## 2. Installation

### macOS

```bash
brew install edgequake-pdf
edgequake-pdf --version
```

### Linux

```bash
apt-get install edgequake-pdf
edgequake-pdf --version
```

## 3. Quick Start

### Basic Extraction

```bash
edgequake-pdf extract input.pdf -o output.md
```

### With Configuration

```bash
edgequake-pdf extract input.pdf -o output.md --config config.toml
```

## 4. Advanced Configuration

Create a `config.toml` file:

```toml
[extraction]
detect_columns = true
extract_tables = true
preserve_formatting = true

[layout]
column_threshold = 0.15
margin_left = 36
margin_right = 36
```

## 5. Troubleshooting

### Problem: Incomplete text extraction

**Solution:** Check page margins settings in configuration

### Problem: Tables not detected

**Solution:** Ensure table formatting is clear with visible borders

## 6. API Reference

### extract_to_markdown()

Extracts PDF content to Markdown format.

**Parameters:**

- `pdf_path` (string): Path to input PDF file
- `options` (ExtractionOptions): Configuration options

**Returns:**

- `Result<String>`: Markdown content or error

**Example:**

```rust
let md = extractor.extract_to_markdown("doc.pdf", &options)?;
println!("{}", md);
```
