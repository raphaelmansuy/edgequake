# OODA Iteration 01 - Observe

**Date:** 2026-02-02  
**Mission:** Improve PDF to Markdown Conversion Quality

---

## 1. Territory Mapped

### 1.1 Problem PDF Analysis

**File:** `zz-explore/001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf`

**Symptoms:**

- Extracted text is completely garbled: `T itle:A Bnn OaUë BeyondTl l TeH MEA ANUST TransformersUHYO R`
- Characters appear scrambled and interleaved
- Spaces appear in wrong positions
- Reading order is incorrect (columns mixing)

**Root Cause Hypothesis:**

1. **Font Encoding Issue**: PDF uses Type 3 fonts or CID fonts without proper ToUnicode CMap
2. **Glyph ID Mapping**: Font uses custom glyph IDs not mapped to Unicode
3. **Text Positioning**: Characters positioned individually with small offsets (presentation/slide PDF)
4. **Multi-column confusion**: Two-column detection interfering with text flow

### 1.2 Current PDF Extraction Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         PdfExtractor                                 │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    ExtractionEngine                          │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────────────┐     │   │
│  │  │ lopdf      │→ │FontHandler │→ │  ContentParser      │     │   │
│  │  │ (PDF parse)│  │(encoding)  │  │  (text extraction)  │     │   │
│  │  └────────────┘  └────────────┘  └────────────────────┘     │   │
│  │         │              │                  │                  │   │
│  │         ▼              ▼                  ▼                  │   │
│  │  ┌────────────────────────────────────────────────────────┐ │   │
│  │  │                 TextGrouper                             │ │   │
│  │  │  - Group elements into lines                            │ │   │
│  │  │  - Column detection                                     │ │   │
│  │  │  - Reading order                                        │ │   │
│  │  └────────────────────────────────────────────────────────┘ │   │
│  │                           │                                  │   │
│  │                           ▼                                  │   │
│  │  ┌────────────────────────────────────────────────────────┐ │   │
│  │  │                 BlockBuilder                            │ │   │
│  │  │  - Merge lines into paragraphs                          │ │   │
│  │  │  - Detect headings/lists                                │ │   │
│  │  └────────────────────────────────────────────────────────┘ │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                      │
│                              ▼                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    ProcessorChain                            │   │
│  │  1. MarginFilterProcessor                                    │   │
│  │  2. GarbledTextFilterProcessor                               │   │
│  │  3. LayoutProcessor                                          │   │
│  │  4. ListDetectionProcessor                                   │   │
│  │  5. StyleDetectionProcessor                                  │   │
│  │  6. TableDetectionProcessor                                  │   │
│  │  7. HeaderDetectionProcessor                                 │   │
│  │  8. BlockMergeProcessor                                      │   │
│  │  9. PostProcessor                                            │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                      │
│                              ▼                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                   MarkdownRenderer                           │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.3 Encoding Support Analysis

**File:** `edgequake/crates/edgequake-pdf/src/backend/encodings.rs`

Current encoding support:

- ✅ WinAnsi Encoding
- ✅ Standard Encoding
- ✅ MacRoman Encoding
- ✅ ToUnicode CMap (basic parsing)
- ✅ Identity-H (UTF-16BE)
- ⚠️ CID fonts (incomplete)
- ❌ Type 3 fonts (not supported)
- ❌ Custom glyph mappings (not supported)

### 1.4 Current Test Corpus

**Location:** `edgequake/crates/edgequake-pdf/test-data/`

| File                               | Description       | Status     |
| ---------------------------------- | ----------------- | ---------- |
| 001_simple_text.pdf                | Basic text        | ✅ Works   |
| 003_two_columns.pdf                | Two column layout | ⚠️ Partial |
| 004_simple_table_2x3.pdf           | Basic table       | ⚠️ Partial |
| 008_multi_page_5_pages.pdf         | Multi-page        | ⚠️ Partial |
| 023_incomplete_unicode_mapping.pdf | Missing ToUnicode | ❌ Fails   |
| 024_embedded_fonts_obfuscated.pdf  | Subset fonts      | ❌ Fails   |
| 025_rotated_text.pdf               | Rotated text      | ❌ Fails   |

### 1.5 Log Analysis from Problem PDF

Key observations from the extraction logs:

1. **Font detection working:**

   ```
   Page 1 has 6 fonts
   ```

2. **Column detection triggered:**

   ```
   Detected TWO-COLUMN layout with boundary at 299.9
   ```

3. **Text elements extremely fragmented:**

   ```
   LINE-ELEM[0]: X=-5.0 Y=-11.6 text='h'
   LINE-ELEM[1]: X=-5.0 Y=-11.6 text='nC'
   LINE-ELEM[2]: X=-1.3 Y=-10.0 text='alcar azanthony1@gmail.com'
   ```

   Text is coming out as individual characters or short fragments, not words.

4. **Table detection interfering:**
   ```
   Table grid: 3 rows, 5 cols (using grid lines - not clustering)
   Table Check: crossing_ratio=0.09 (14/154)
   📊 BUILDING TABLE: grid=9x4 (rows x cols)
   ```

---

## 2. Current Quality Metrics

**No formal quality metrics exist.** The current system has:

- Unit tests for individual components
- No E2E quality scoring
- No automated regression testing for conversion quality
- No gold standard comparisons

---

## 3. Key Findings

### 3.1 Critical Issues

1. **Font Encoding Gap**: The PDF uses fonts with glyph IDs that don't map to Unicode via standard encodings. Need to:
   - Detect when ToUnicode is missing
   - Try alternate recovery strategies (font name heuristics, OCR fallback)

2. **Character Fragmentation**: Text elements are arriving as single characters or 2-3 char fragments. This is likely due to:
   - PDF using `TJ` operator with individual glyph positioning
   - Each glyph being emitted separately for precise kerning

3. **Reading Order Confusion**: The multi-column detection is seeing the fragmented characters as spanning both columns, causing interleaving.

4. **No Fallback Strategy**: When encoding fails, there's no graceful degradation or alternative approach.

### 3.2 Missing Capabilities

| Capability          | Status          | Impact |
| ------------------- | --------------- | ------ |
| CID Font Decoding   | Missing         | High   |
| Type 3 Font Support | Missing         | High   |
| Glyph Name Fallback | Missing         | Medium |
| OCR Fallback        | Exists (vision) | Low    |
| Quality Scoring     | Missing         | High   |
| Regression Tests    | Missing         | High   |

---

## 4. External Resources to Research

1. **PDF Reference Manual** - Font encoding specifications
2. **CMap specification** - Adobe technical notes
3. **pdf.js approach** - How Mozilla handles font encoding
4. **poppler's approach** - How poppler handles encoding fallback
5. **PyMuPDF** - Python library known for good extraction

---

## 5. Questions to Answer

1. What font types does this PDF use? (Type 1, TrueType, Type 3, CID?)
2. Does the PDF have ToUnicode streams that we're failing to parse?
3. What does `pdftotext` (poppler) output for this file?
4. Can we detect garbled text and fall back to vision/OCR mode?
5. What's a good quality metric for measuring conversion accuracy?
