# OODA Iteration 19: DECIDE

**Focus**: PDF Processing Deep Dive - Detailed Plan
**Date**: 2026-01-29

---

## Decision Summary

**CREATE**: `docs/deep-dives/pdf-processing.md` (~900 lines)

**RATIONALE**: PDF processing is EdgeQuake's competitive advantage but completely undocumented. This deep dive will enable users to:

1. Extract structured content from PDFs
2. Understand table detection capabilities
3. Handle character encoding issues
4. Interpret quality metrics
5. Troubleshoot common problems

---

## Document Structure

### Outline

```markdown
# PDF Processing Deep Dive

## 1. Introduction (80 lines)

- What problem does EdgeQuake PDF solve?
- Why existing tools fail
- EdgeQuake's approach
- When to use PDF extraction

## 2. Architecture (150 lines)

- Processing pipeline diagram
- Key components
- Data flow
- Design decisions (WHY)

## 3. Basic Usage (120 lines)

- Quick start example
- API overview
- Common patterns
- Error handling

## 4. Table Detection (180 lines)

- How it works (algorithm)
- Coordinate clustering
- Column detection
- Confidence scores
- ASCII flow diagram

## 5. Character Encoding (160 lines)

- Supported encodings
- Auto-detection
- Normalization
- Ligature handling
- Decision tree diagram

## 6. Quality Metrics (130 lines)

- Extraction confidence
- Table accuracy
- Encoding confidence
- Empty page detection
- Scoring model diagram

## 7. Advanced Topics (150 lines)

- Custom extraction settings
- Performance tuning
- Pipeline integration
- Batch processing

## 8. Troubleshooting (100 lines)

- Common issues
- Encoding problems
- Table detection failures
- Performance issues
- Debug techniques

## 9. Comparison (50 lines)

- vs PyPDF2
- vs pdfplumber
- vs Camelot
- EdgeQuake advantages

## 10. References (30 lines)

- Source code links
- Test examples
- Related docs
- External resources
```

**Total**: ~900 lines

---

## ASCII Diagrams (4 required)

### Diagram 1: Processing Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    PDF PROCESSING PIPELINE                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  INPUT                                                            │
│  ┌──────────┐                                                    │
│  │ PDF File │                                                    │
│  │ (bytes)  │                                                    │
│  └─────┬────┘                                                    │
│        │                                                          │
│        ▼                                                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STAGE 1: Parse PDF Structure                             │   │
│  │ • Load PDF using lopdf                                   │   │
│  │ • Extract metadata (pages, fonts, encodings)            │   │
│  │ • Build page object tree                                 │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STAGE 2: Extract Raw Text + Positions                   │   │
│  │ • Iterate through page content streams                   │   │
│  │ • Parse text operators (Tj, TJ, ')                       │   │
│  │ • Capture X/Y coordinates for each text element          │   │
│  │ • Track font changes and sizes                           │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STAGE 3: Detect & Normalize Encodings                   │   │
│  │ • Detect encoding (Latin-1, UTF-8, Windows-1252, etc.)  │   │
│  │ • Normalize characters (é, ñ, ligatures)                │   │
│  │ • Handle custom font encodings                           │   │
│  │ • Calculate confidence score                             │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STAGE 4: Detect Tables                                   │   │
│  │ • Cluster Y-coordinates (rows)                           │   │
│  │ • Analyze X-coordinates (columns)                        │   │
│  │ • Build cell grid                                        │   │
│  │ • Generate Markdown tables                               │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STAGE 5: Reconstruct Structure                           │   │
│  │ • Detect headings (font size > avg)                      │   │
│  │ • Detect lists (bullet points, numbers)                  │   │
│  │ • Reconstruct paragraphs                                 │   │
│  │ • Add page markers                                       │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  OUTPUT                                                           │
│  ┌────────────────────────────────────────────────────────┐     │
│  │ Markdown Text                                          │     │
│  │ • Headings: # Title                                    │     │
│  │ • Tables: | Col1 | Col2 |                             │     │
│  │ • Lists: - Item                                        │     │
│  │ • Metadata: confidence=0.95                            │     │
│  └────────────────────────────────────────────────────────┘     │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘

WHY THIS DESIGN:
1. Staged pipeline allows early failure detection
2. Position tracking enables table detection
3. Encoding normalization prevents garbage text
4. Structure preservation maintains semantic meaning
5. Confidence scores enable quality filtering
```

### Diagram 2: Table Detection Algorithm

```
┌─────────────────────────────────────────────────────────────────┐
│                    TABLE DETECTION ALGORITHM                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  INPUT: Text elements with X,Y coordinates                       │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Text("Name",   x=50,  y=100, font_size=10)               │   │
│  │ Text("Age",    x=200, y=100, font_size=10)               │   │
│  │ Text("City",   x=350, y=100, font_size=10)               │   │
│  │ Text("Alice",  x=50,  y=85,  font_size=10)               │   │
│  │ Text("25",     x=200, y=85,  font_size=10)               │   │
│  │ Text("NYC",    x=350, y=85,  font_size=10)               │   │
│  └──────────────────────────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STEP 1: Cluster Y-Coordinates (ROWS)                     │   │
│  │                                                           │   │
│  │ Algorithm:                                                │   │
│  │   1. Sort all Y values: [100, 100, 100, 85, 85, 85]     │   │
│  │   2. Group within threshold (5px):                       │   │
│  │      Row 1: y ≈ 100 → [Name, Age, City]                 │   │
│  │      Row 2: y ≈ 85  → [Alice, 25, NYC]                  │   │
│  │                                                           │   │
│  │ WHY: Text on same line has similar Y-coordinate          │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STEP 2: Analyze X-Coordinates (COLUMNS)                  │   │
│  │                                                           │   │
│  │ Algorithm:                                                │   │
│  │   1. Extract X positions from all rows:                  │   │
│  │      [50, 200, 350] (repeated)                           │   │
│  │   2. Cluster X values within threshold (10px)            │   │
│  │      Col 1: x ≈ 50  (Name, Alice)                        │   │
│  │      Col 2: x ≈ 200 (Age, 25)                            │   │
│  │      Col 3: x ≈ 350 (City, NYC)                          │   │
│  │                                                           │   │
│  │ WHY: Text in same column has similar X-coordinate        │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STEP 3: Build Cell Grid                                  │   │
│  │                                                           │   │
│  │ Grid[row][col]:                                           │   │
│  │   Grid[0][0] = "Name"   Grid[0][1] = "Age"  Grid[0][2] = "City" │   │
│  │   Grid[1][0] = "Alice"  Grid[1][1] = "25"   Grid[1][2] = "NYC"  │   │
│  │                                                           │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STEP 4: Calculate Confidence                             │   │
│  │                                                           │   │
│  │ Factors:                                                  │   │
│  │   • Rows have same column count: +30%                    │   │
│  │   • Y-spacing consistent: +20%                           │   │
│  │   • X-spacing consistent: +20%                           │   │
│  │   • Header detected (font/style): +15%                   │   │
│  │   • Minimum 2 rows: +15%                                 │   │
│  │                                                           │   │
│  │ Confidence = 100% (perfect table!)                       │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  OUTPUT: Markdown Table                                          │
│  ┌────────────────────────────────────────────────────────┐     │
│  │ | Name  | Age | City |                                 │     │
│  │ |-------|-----|------|                                 │     │
│  │ | Alice | 25  | NYC  |                                 │     │
│  │                                                        │     │
│  │ Metadata: confidence=1.0, rows=2, cols=3              │     │
│  └────────────────────────────────────────────────────────┘     │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘

EDGE CASES HANDLED:
1. Ragged tables (uneven columns) → Lower confidence
2. Merged cells → Best-effort reconstruction
3. Multi-line cells → Y-threshold adjustment
4. No clear headers → Generic column names (Col1, Col2)
5. Single-row tables → Rejected (confidence=0)
```

### Diagram 3: Character Encoding Detection

```
┌─────────────────────────────────────────────────────────────────┐
│                CHARACTER ENCODING DETECTION                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  INPUT: Raw bytes from PDF                                       │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Bytes: [0xE9, 0xF1, 0xFC, ...]                           │   │
│  │ Font: Arial (standard)                                    │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ DECISION 1: Check PDF Encoding Hints                     │   │
│  │                                                           │   │
│  │ ┌───────────────────────┐                                │   │
│  │ │ PDF has ToUnicode map?├─YES─► Use Unicode mapping      │   │
│  │ └───────────┬───────────┘                                │   │
│  │             │ NO                                          │   │
│  │             ▼                                             │   │
│  │ ┌───────────────────────┐                                │   │
│  │ │ Font has encoding?    ├─YES─► Use font encoding        │   │
│  │ └───────────┬───────────┘                                │   │
│  │             │ NO                                          │   │
│  │             ▼                                             │   │
│  │         Continue to heuristics                            │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ DECISION 2: Analyze Byte Patterns                        │   │
│  │                                                           │   │
│  │ Check for common patterns:                                │   │
│  │                                                           │   │
│  │ ┌────────────────────────────┐                           │   │
│  │ │ 0x00-0x7F only?            ├─YES─► ASCII (confidence=1.0) │   │
│  │ └─────────┬──────────────────┘                           │   │
│  │           │ NO                                            │   │
│  │           ▼                                               │   │
│  │ ┌────────────────────────────┐                           │   │
│  │ │ Valid UTF-8 sequences?     ├─YES─► UTF-8 (confidence=0.9) │   │
│  │ └─────────┬──────────────────┘                           │   │
│  │           │ NO                                            │   │
│  │           ▼                                               │   │
│  │ ┌────────────────────────────┐                           │   │
│  │ │ 0x80-0xFF high frequency?  ├─YES─► Latin-1/Win1252    │   │
│  │ └─────────┬──────────────────┘                           │   │
│  │           │ NO                                            │   │
│  │           ▼                                               │   │
│  │        Unknown encoding (fallback: Latin-1)               │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ DECISION 3: Test Against Character Frequency             │   │
│  │                                                           │   │
│  │ For Latin-1 vs Windows-1252:                              │   │
│  │                                                           │   │
│  │ Check bytes 0x80-0x9F:                                    │   │
│  │   • Latin-1: Control characters (invalid in text)        │   │
│  │   • Win-1252: Printable (€, ‚, ƒ, „, etc.)              │   │
│  │                                                           │   │
│  │ ┌────────────────────────────┐                           │   │
│  │ │ Printable chars found?     ├─YES─► Win-1252           │   │
│  │ └─────────┬──────────────────┘                           │   │
│  │           │ NO                                            │   │
│  │           ▼                                               │   │
│  │        Latin-1                                            │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STEP 4: Normalize Characters                             │   │
│  │                                                           │   │
│  │ Apply transformations:                                    │   │
│  │   • Convert to UTF-8                                     │   │
│  │   • Expand ligatures: "fi" → "f" + "i"                  │   │
│  │   • Normalize combining chars: é (e + ´) → é (single)   │   │
│  │   • Remove zero-width chars                              │   │
│  │   • Handle custom glyphs (font-specific)                 │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  OUTPUT: Normalized UTF-8 Text                                   │
│  ┌────────────────────────────────────────────────────────┐     │
│  │ "café" (correctly decoded)                             │     │
│  │ Metadata: encoding=Latin-1, confidence=0.85            │     │
│  └────────────────────────────────────────────────────────┘     │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘

SUPPORTED ENCODINGS (15+):
• UTF-8, UTF-16LE, UTF-16BE
• ASCII, Latin-1 (ISO-8859-1)
• Windows-1252 (Western European)
• Windows-1251 (Cyrillic)
• MacRoman
• Custom font encodings
• Embedded Unicode mappings
```

### Diagram 4: Quality Scoring Model

```
┌─────────────────────────────────────────────────────────────────┐
│                    QUALITY SCORING MODEL                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                OVERALL EXTRACTION QUALITY                 │   │
│  │                                                           │   │
│  │  Final Score = weighted average of:                       │   │
│  │    • Text Extraction: 40%                                │   │
│  │    • Encoding Quality: 30%                               │   │
│  │    • Structure Quality: 20%                              │   │
│  │    • Table Quality: 10%                                  │   │
│  └─────────────────────┬────────────────────────────────────┘   │
│                        │                                         │
│         ┌──────────────┼──────────────┐                         │
│         │              │              │                         │
│         ▼              ▼              ▼                         │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐    │
│  │   TEXT   │   │ ENCODING │   │ STRUCTURE│   │  TABLE   │    │
│  │  QUALITY │   │ QUALITY  │   │ QUALITY  │   │ QUALITY  │    │
│  └────┬─────┘   └────┬─────┘   └────┬─────┘   └────┬─────┘    │
│       │              │              │              │            │
│       ▼              ▼              ▼              ▼            │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ TEXT EXTRACTION QUALITY (40% weight)                     │   │
│  │                                                           │   │
│  │ Score = (successful_chars / total_chars)                 │   │
│  │                                                           │   │
│  │ Penalties:                                                │   │
│  │   • No text extracted: score = 0.0                       │   │
│  │   • Mostly whitespace: score *= 0.5                      │   │
│  │   • Many control chars: score *= 0.8                     │   │
│  │                                                           │   │
│  │ Bonuses:                                                  │   │
│  │   • Complete paragraphs: score *= 1.1                    │   │
│  │   • Proper spacing: score *= 1.05                        │   │
│  │                                                           │   │
│  │ Example: "This is text" → score = 1.0 * 1.1 * 1.05 = 1.0│   │
│  └──────────────────────────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ ENCODING QUALITY (30% weight)                            │   │
│  │                                                           │   │
│  │ Score = encoding_confidence * char_validity              │   │
│  │                                                           │   │
│  │ encoding_confidence:                                      │   │
│  │   • ToUnicode map present: 1.0                           │   │
│  │   • Valid UTF-8 detected: 0.95                           │   │
│  │   • Latin-1 heuristic: 0.85                              │   │
│  │   • Fallback encoding: 0.70                              │   │
│  │   • Unknown/mixed: 0.50                                  │   │
│  │                                                           │   │
│  │ char_validity:                                            │   │
│  │   • All printable: 1.0                                   │   │
│  │   • <5% invalid: 0.95                                    │   │
│  │   • <10% invalid: 0.80                                   │   │
│  │   • >10% invalid: 0.50                                   │   │
│  │                                                           │   │
│  │ Example: UTF-8 (0.95) * all printable (1.0) = 0.95      │   │
│  └──────────────────────────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STRUCTURE QUALITY (20% weight)                           │   │
│  │                                                           │   │
│  │ Score = detected_elements / expected_elements            │   │
│  │                                                           │   │
│  │ Elements scored:                                          │   │
│  │   • Headings detected: +0.3                              │   │
│  │   • Lists detected: +0.2                                 │   │
│  │   • Paragraphs separated: +0.3                           │   │
│  │   • Page markers preserved: +0.2                         │   │
│  │                                                           │   │
│  │ Example:                                                  │   │
│  │   Detected: 2 headings, 1 list, 3 paragraphs, pages     │   │
│  │   Score = 0.3 + 0.2 + 0.3 + 0.2 = 1.0                   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ TABLE QUALITY (10% weight)                               │   │
│  │                                                           │   │
│  │ Score = avg(table_confidences) if tables detected       │   │
│  │       = 1.0 if no tables in document                     │   │
│  │                                                           │   │
│  │ table_confidence (per table):                             │   │
│  │   • Consistent columns: +30%                             │   │
│  │   • Consistent row spacing: +20%                         │   │
│  │   • Consistent col spacing: +20%                         │   │
│  │   • Header detected: +15%                                │   │
│  │   • Min 2 rows: +15%                                     │   │
│  │                                                           │   │
│  │ Example: 2 tables with conf [1.0, 0.8] → avg = 0.9      │   │
│  └──────────────────────────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ FINAL SCORE CALCULATION                                   │   │
│  │                                                           │   │
│  │ Overall = (text * 0.4) + (encoding * 0.3) +              │   │
│  │           (structure * 0.2) + (table * 0.1)              │   │
│  │                                                           │   │
│  │ Example:                                                  │   │
│  │   Text:      1.0  * 0.4 = 0.40                           │   │
│  │   Encoding:  0.95 * 0.3 = 0.285                          │   │
│  │   Structure: 1.0  * 0.2 = 0.20                           │   │
│  │   Table:     0.9  * 0.1 = 0.09                           │   │
│  │                           ─────                           │   │
│  │   Overall:              = 0.975                          │   │
│  │                                                           │   │
│  │ ┌────────────────────────────────┐                       │   │
│  │ │ QUALITY THRESHOLDS              │                       │   │
│  │ │ • Excellent: ≥ 0.90             │                       │   │
│  │ │ • Good:      ≥ 0.75             │                       │   │
│  │ │ • Fair:      ≥ 0.60             │                       │   │
│  │ │ • Poor:      < 0.60             │                       │   │
│  │ └────────────────────────────────┘                       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘

INTERPRETATION:
• score ≥ 0.90: Production-ready extraction
• score ≥ 0.75: Usable with minor issues
• score ≥ 0.60: Review recommended
• score < 0.60: Manual review required
```

---

## Code Examples (10+ required)

### Example 1: Basic Extraction

```rust
use edgequake_pdf::PdfExtractor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize extractor
    let extractor = PdfExtractor::new();

    // Extract from file
    let result = extractor.extract_from_file("document.pdf").await?;

    // Access extracted content
    println!("Extracted text:\n{}", result.text);
    println!("Quality score: {:.2}", result.metadata.confidence);
    println!("Pages processed: {}", result.metadata.page_count);

    Ok(())
}
```

**Source**: `edgequake/crates/edgequake-pdf/src/lib.rs:30-50`

### Example 2: Custom Settings

```rust
use edgequake_pdf::{PdfExtractor, ExtractionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Custom configuration
    let config = ExtractionConfig {
        enable_table_detection: true,
        table_confidence_threshold: 0.75,
        enable_ocr: false,
        max_pages: Some(100),
        encoding_hints: vec!["UTF-8", "Latin-1"],
    };

    // Create extractor with config
    let extractor = PdfExtractor::with_config(config);

    // Extract
    let result = extractor.extract_from_file("complex.pdf").await?;

    // Check quality
    if result.metadata.confidence < 0.60 {
        eprintln!("Warning: Low quality extraction!");
    }

    Ok(())
}
```

**Source**: `edgequake/crates/edgequake-pdf/src/extractor.rs:50-80`

### Example 3: Table Handling

```rust
use edgequake_pdf::PdfExtractor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let extractor = PdfExtractor::new();
    let result = extractor.extract_from_file("tables.pdf").await?;

    // Access detected tables
    for (i, table) in result.tables.iter().enumerate() {
        println!("Table {}: {} rows, {} cols, confidence={:.2}",
                 i + 1,
                 table.rows.len(),
                 table.cols.len(),
                 table.confidence);

        // Print as Markdown
        println!("{}", table.to_markdown());
    }

    Ok(())
}
```

**Source**: `edgequake/crates/edgequake-pdf/src/table_detector.rs:200-250`

### Example 4: Error Handling

```rust
use edgequake_pdf::{PdfExtractor, PdfError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let extractor = PdfExtractor::new();

    match extractor.extract_from_file("document.pdf").await {
        Ok(result) => {
            println!("Success! Extracted {} chars", result.text.len());
        }
        Err(PdfError::CorruptedFile(msg)) => {
            eprintln!("Corrupted PDF: {}", msg);
        }
        Err(PdfError::UnsupportedEncoding(encoding)) => {
            eprintln!("Unsupported encoding: {}", encoding);
        }
        Err(PdfError::IoError(e)) => {
            eprintln!("File error: {}", e);
        }
        Err(e) => {
            eprintln!("Unknown error: {}", e);
        }
    }

    Ok(())
}
```

**Source**: `edgequake/crates/edgequake-pdf/src/error.rs:10-40`

### Example 5: Quality Checking

```rust
use edgequake_pdf::PdfExtractor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let extractor = PdfExtractor::new();
    let result = extractor.extract_from_file("document.pdf").await?;

    // Check overall quality
    let quality = result.metadata.confidence;
    match quality {
        q if q >= 0.90 => println!("✓ Excellent extraction"),
        q if q >= 0.75 => println!("✓ Good extraction"),
        q if q >= 0.60 => println!("⚠ Fair extraction - review recommended"),
        _ => println!("✗ Poor extraction - manual review required"),
    }

    // Check encoding quality
    if result.metadata.encoding_confidence < 0.80 {
        eprintln!("Warning: Low encoding confidence, text may be garbled");
    }

    // Check for empty pages
    if result.metadata.empty_pages > 0 {
        println!("Info: {} empty pages skipped", result.metadata.empty_pages);
    }

    Ok(())
}
```

**Source**: `edgequake/crates/edgequake-pdf/tests/quality_tests.rs:50-100`

### Example 6: Batch Processing

```rust
use edgequake_pdf::PdfExtractor;
use std::path::Path;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let extractor = PdfExtractor::new();
    let pdf_dir = Path::new("pdfs/");

    // Read directory
    let mut entries = fs::read_dir(pdf_dir).await?;

    // Process each PDF
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.extension().map(|s| s == "pdf").unwrap_or(false) {
            println!("Processing: {:?}", path);

            match extractor.extract_from_file(&path).await {
                Ok(result) => {
                    println!("  ✓ {} chars, quality={:.2}",
                             result.text.len(),
                             result.metadata.confidence);
                }
                Err(e) => {
                    eprintln!("  ✗ Error: {}", e);
                }
            }
        }
    }

    Ok(())
}
```

**Source**: `edgequake/crates/edgequake-pdf/examples/batch_processing.rs:10-60`

### Example 7: Pipeline Integration

```rust
use edgequake_core::pipeline::PipelineBuilder;
use edgequake_pdf::PdfExtractor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create pipeline with PDF support
    let pipeline = PipelineBuilder::new()
        .with_pdf_extractor(PdfExtractor::new())
        .with_chunking(512)
        .with_llm(/* LLM config */)
        .with_storage(/* Storage config */)
        .build()?;

    // Ingest PDF document
    let doc_id = pipeline
        .ingest_file("research-paper.pdf")
        .await?;

    println!("Ingested document: {}", doc_id);

    // Query the document
    let results = pipeline
        .query("What are the main findings?")
        .await?;

    println!("Answer: {}", results.answer);

    Ok(())
}
```

**Source**: `edgequake/crates/edgequake-core/examples/production_pipeline.rs:40-80`

### Additional Examples (3 more)

- Example 8: Encoding Detection
- Example 9: Performance Tuning
- Example 10: Debug Logging

(Abbreviated for brevity - full examples in final doc)

---

## Verification Checklist

### Code Verification

- [ ] All code examples compile
- [ ] All examples tested against test suite
- [ ] File paths verified in source
- [ ] Line numbers accurate (±5 lines)
- [ ] API signatures match current code
- [ ] Error types match actual enums

### Content Verification

- [ ] Architecture diagram matches `src/lib.rs`
- [ ] Table detection algorithm matches `src/table_detector.rs`
- [ ] Encoding detection matches `src/encodings/mod.rs`
- [ ] Quality scoring matches `src/quality.rs`
- [ ] Config options match `src/config.rs`

### Quality Verification

- [ ] First principles explanations (WHY)
- [ ] No speculation or unverified claims
- [ ] High signal-to-noise ratio
- [ ] Follows existing doc style
- [ ] 4+ ASCII diagrams included
- [ ] 10+ code examples verified

---

## Success Criteria

**Quantitative**:

- ✅ 900+ lines of content
- ✅ 4+ ASCII diagrams
- ✅ 10+ code examples
- ✅ All examples compile and run
- ✅ 100% claim verification

**Qualitative**:

- ✅ User can extract PDF in <5 minutes after reading
- ✅ User understands table detection algorithm
- ✅ User can troubleshoot encoding issues
- ✅ User can interpret quality scores
- ✅ Competitive advantages highlighted

**Process**:

- ✅ No speculative content
- ✅ All claims traced to source code
- ✅ First principles thinking applied
- ✅ High signal-to-noise ratio maintained

---

## Git Commit Plan

**Commit Message**:

```
OODA-19: Add PDF Processing deep dive

- Create docs/deep-dives/pdf-processing.md (920 lines)
- Document table detection algorithm with ASCII diagrams
- Explain character encoding handling
- Add quality scoring model
- Include 10 verified code examples
- All content verified against edgequake-pdf source

Closes gap: Users now understand PDF extraction capabilities
```

---

## Next Iteration Preview

**Iteration 20** will:

1. Create `docs/tutorials/pdf-ingestion.md`
2. Update `docs/tutorials/document-ingestion.md` with PDF examples
3. Update `docs/troubleshooting/common-issues.md` with PDF section

---

## Final Review

**Decision**: APPROVED - Proceed to ACT phase

**Confidence**: 95% - All planning complete, ready for implementation

**Next**: Create `docs/deep-dives/pdf-processing.md` with full content
