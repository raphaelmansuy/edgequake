# PDF Ingestion Tutorial

EdgeQuake extracts text, tables, and metadata from PDF documents using advanced layout analysis. This tutorial shows you how to upload PDFs and configure extraction for optimal results.

## What You'll Learn

- Upload a PDF document (5 minutes)
- Configure extraction options (10 minutes)
- Verify extraction quality (5 minutes)
- Query PDF content (5 minutes)

**Prerequisites**:

- EdgeQuake server running (see [Quick Start](../quick-start.md))
- A PDF file to upload
- `curl` or `httpie` installed

**Time Estimate**: 25 minutes

## When to Read This

**Read this tutorial** if:

- First time uploading PDFs
- Need quick reference for configuration options
- Want to verify extraction quality

**Read [PDF Processing Deep Dive](../deep-dives/pdf-processing.md)** if:

- Understanding extraction internals
- Advanced table detection algorithms
- Contributing to PDF crate

**Read [Troubleshooting Guide](../troubleshooting/common-issues.md#pdf-extraction-issues)** if:

- Extraction fails or produces poor quality
- Tables not detected correctly
- Need detailed error solutions

**Theory vs Practice**:

- This tutorial: "How do I upload and configure?"
- Deep dive: "How does table detection work internally?"
- Both are valuable - start here, dig deeper as needed.

---

## Quick Start: Your First PDF Upload

### Step 1: Upload the PDF

```bash
# Upload with default settings (text mode)
curl -X POST \
  -H "Content-Type: multipart/form-data" \
  -F "file=@/path/to/paper.pdf" \
  -F "title=Research Paper" \
  http://localhost:8080/api/v1/documents
```

**What Happens**:

```
Upload → Parse PDF → Extract text → Detect tables → Build chunks → Index → Ready
```

**Response**:

```json
{
  "id": "doc-uuid-1234",
  "title": "Research Paper",
  "status": "completed",
  "content_hash": "sha256:abc123...",
  "chunk_count": 45,
  "entity_count": 23,
  "relationship_count": 18,
  "created_at": "2024-01-15T10:30:00Z",
  "processing_time_ms": 2340
}
```

**Key Fields**:

- `id`: Use this to reference the document in queries
- `status`: `completed` means extraction succeeded
- `chunk_count`: Number of text chunks created (paragraphs, tables)
- `processing_time_ms`: Extraction took ~2.3 seconds

**Note**: Base URL is `http://localhost:8080` by default. If your server uses a different port, adjust accordingly.

---

### Step 2: Verify Upload Succeeded

```bash
# Check document status
curl http://localhost:8080/api/v1/documents/doc-uuid-1234
```

**Response**:

```json
{
  "id": "doc-uuid-1234",
  "title": "Research Paper",
  "status": "indexed",
  "metadata": {
    "pages": 12,
    "tables_detected": 3,
    "figures": 5
  }
}
```

**Look for**:

- ✅ `status: "indexed"` - ready to query
- ✅ `chunk_count > 0` - text extracted successfully
- ✅ `entity_count > 0` - knowledge graph built
- ⚠️ `status: "failed"` - see [troubleshooting](#troubleshooting-quick-reference)

**Tip**: For complex PDFs with tables, consider enabling table enhancement (see [Configuration](#configuration-options)).

---

### Step 3: Query the PDF Content

```bash
# Ask a question about the document
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What are the key findings?",
    "mode": "hybrid"
  }' \
  http://localhost:8080/api/v1/query
```

**Response**:

```json
{
  "answer": "The key findings show that...",
  "sources": [
    {
      "document_id": "doc-uuid-1234",
      "chunk_id": "chunk-5",
      "relevance": 0.94,
      "page": 3,
      "content": "The results demonstrate..."
    }
  ],
  "response_time_ms": 1200
}
```

**Success**: You've uploaded, indexed, and queried a PDF in < 5 minutes! 🎉

---

## Upload Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      PDF Upload Flow                            │
└─────────────────────────────────────────────────────────────────┘

  User                EdgeQuake Server                  Knowledge Graph
   │                          │                                 │
   │  POST /api/v1/documents  │                                 │
   │  (file + metadata)       │                                 │
   ├────────────────────────> │                                 │
   │                          │                                 │
   │                          │ 1. Parse PDF                    │
   │                          │    (extract pages)              │
   │                          │                                 │
   │                          │ 2. Extract Text                 │
   │                          │    (with layout)                │
   │                          │                                 │
   │                          │ 3. Detect Tables                │
   │                          │    (spatial clustering)         │
   │                          │                                 │
   │                          │ 4. Build Chunks                 │
   │                          │    (semantic units)             │
   │                          │                                 │
   │                          │ 5. Extract Entities             │
   │                          ├────────────────────────────────>│
   │                          │    (people, orgs, concept s)    │
   │                          │                                 │
   │                          │ 6. Index for Search             │
   │                          │<────────────────────────────────┤
   │  Response:               │                                 │
   │  {id, status, chunks}    │                                 │
   │<──────────────────────── ┤                                 │
   │                          │                                 │
   │  Query request           │                                 │
   ├────────────────────────> │ 7. Query Graph                  │
   │                          ├────────────────────────────────>│
   │                          │    (find relevant chunks)       │
   │  Response:               │<────────────────────────────────┤
   │  {answer, sources}       │                                 │
   │<──────────────────────── ┤                                 │

Total time: 2-5 seconds (text mode) | 20-50 seconds (vision mode)
```

---

## Configuration Options

EdgeQuake supports three extraction modes: Text, Vision, and Hybrid. Choose the mode based on your PDF quality and requirements.

### When to Use What

**Text Mode** (default, fastest):

- ✅ Good quality digital PDFs
- ✅ Standard fonts and encoding
- ✅ Simple to moderately complex layouts
- **Processing Time**: 2-5 seconds per document
- **Cost**: Free (no LLM API calls)

**Vision Mode** (slowest, most accurate):

- ⚠️ Scanned documents (images)
- ⚠️ Poor quality PDFs
- ⚠️ No text layer (image-only PDFs)
- ⚠️ Complex layouts or handwriting
- **Processing Time**: 20-50 seconds per document
- **Cost**: ~$0.001-0.01 per page (LLM vision API)

**Hybrid Mode** (automatic fallback):

- ⚠️ Mixed quality (some pages good, some poor)
- ⚠️ Unsure about PDF quality
- **Processing Time**: Variable (2-50 seconds)
- **Cost**: Only vision pages incur LLM cost

**Table Enhancement**:

- ⚠️ Complex table layouts
- ⚠️ Merged cells
- ⚠️ Nested tables
- **Trade-off**: 2x slower, better table accuracy
- **Cost**: ~$0.0001 per table (LLM refinement)

---

### Example 1: Text Mode (Default)

```bash
# Default text mode - fastest, free
curl -X POST \
  -H "Content-Type: multipart/form-data" \
  -F "file=@report.pdf" \
  -F "title=Annual Report" \
  http://localhost:8080/api/v1/documents
```

**Use for**: 80% of digital PDFs  
**Processing**: 2-5 seconds  
**Cost**: Free

---

### Example 2: Vision Mode (Scanned PDFs)

For scanned documents or image-based PDFs, explicitly set mode to Vision:

```bash
# Vision mode for scanned documents
curl -X POST \
  -H "Content-Type: multipart/form-data" \
  -F "file=@scanned_book.pdf" \
  -F "title=Scanned Book" \
  -F 'config={"mode": "Vision", "vision_dpi": 150}' \
  http://localhost:8080/api/v1/documents
```

**Configuration Fields**:

- `mode`: `"Text"`, `"Vision"`, or `"Hybrid"`
- `vision_dpi`: DPI for rendering (150 = good quality, 200 = higher accuracy but slower)

**Use for**: Scanned books, poor quality PDFs, image-only PDFs  
**Processing**: 20-50 seconds depending on page count  
**Cost**: ~$0.001-0.01 per page (OpenAI GPT-4o-mini)

**Cost Example**: 50-page book at $0.005/page = $0.25 total

---

### Example 3: Hybrid Mode (Automatic)

Hybrid mode uses text extraction first, then falls back to vision for low-quality pages:

```bash
# Hybrid mode - automatic quality detection
curl -X POST \
  -H "Content-Type: multipart/form-data" \
  -F "file=@mixed_quality.pdf" \
  -F "title=Mixed Quality Document" \
  -F 'config={"mode": "Hybrid", "quality_threshold": 0.7}' \
  http://localhost:8080/api/v1/documents
```

**Configuration Fields**:

- `quality_threshold`: If text extraction confidence < this value, use vision (0.0-1.0)
- Default: `0.5` (switch to vision for confidence < 50%)

**Use for**: Unknown PDF quality, mixed content documents  
**Processing**: 2-50 seconds depending on quality  
**Cost**: Only low-quality pages incur vision costs

---

### Example 4: Enhanced Table Detection

For PDFs with complex tables (merged cells, nested structures):

```bash
# Enable LLM-based table enhancement
curl -X POST \
  -H "Content-Type: multipart/form-data" \
  -F "file=@financial_report.pdf" \
  -F "title=Financial Report" \
  -F 'config={"enhance_tables": true, "mode": "Text"}' \
  http://localhost:8080/api/v1/documents
```

**Configuration Fields**:

- `enhance_tables`: Enable LLM refinement for tables (default: `false`)
- `ai_temperature`: LLM temperature for table enhancement (0.0-1.0, default: 0.1)

**Use for**: Financial reports, spreadsheets, data-heavy documents  
**Processing**: 2x slower than default  
**Cost**: ~$0.0001 per table

**Result**: Tables with merged cells and complex layouts correctly preserved in markdown.

---

### Example 5: Multi-Column Layout

For academic papers and newspaper-style layouts:

```bash
# Enable column detection
curl -X POST \
  -H "Content-Type: multipart/form-data" \
  -F "file=@research_paper.pdf" \
  -F "title=Research Paper" \
  -F 'config={"layout": {"detect_columns": true, "column_gap_threshold": 20.0}}' \
  http://localhost:8080/api/v1/documents
```

**Configuration Fields**:

- `layout.detect_columns`: Enable multi-column detection (default: `true`)
- `layout.column_gap_threshold`: Minimum gap in points for column separation (default: 20.0)

**Use for**: Academic papers, newspapers, magazines  
**Processing**: Minimal overhead  
**Cost**: Free

---

### Example 6: Full Enhancement (Complex Documents)

For critical documents where accuracy > speed:

```bash
# Enable all enhancements
curl -X POST \
  -H "Content-Type: multipart/form-data" \
  -F "file=@complex_report.pdf" \
  -F "title=Complex Report" \
  -F 'config={
    "mode": "Vision",
    "enhance_tables": true,
    "layout": {"detect_columns": true},
    "enhance_readability": true,
    "vision_dpi": 200
  }' \
  http://localhost:8080/api/v1/documents
```

**Use for**: Legal documents, critical reports, archival  
**Processing**: 10x slower  
**Cost**: ~$0.01 per page

**Trade-off**: Maximum accuracy, but significantly slower and more expensive.

---

## Configuration Reference

### PdfConfig Fields

Complete reference of available configuration options:

```json
{
  "mode": "Text", // Text | Vision | Hybrid
  "output_format": "Markdown", // Markdown | Json | Html | Chunks
  "ocr_threshold": 0.8, // OCR confidence threshold (0.0-1.0)
  "max_pages": null, // Limit pages to process (null = all)
  "include_page_numbers": true, // Include page numbers in output
  "extract_images": true, // Extract embedded images
  "enhance_tables": false, // LLM table refinement
  "ai_temperature": 0.1, // LLM temperature (0.0 = deterministic)
  "normalize_spacing": true, // Fix concatenated words
  "consolidate_headers": true, // Merge broken headers
  "extract_figure_captions": true, // Extract figure captions
  "enhance_readability": false, // AI full-page enhancement
  "vision_dpi": 150, // DPI for vision mode (150-300)
  "quality_threshold": 0.5, // Hybrid mode threshold
  "layout": {
    "detect_columns": true, // Multi-column detection
    "detect_tables": true, // Table detection
    "detect_equations": true, // Equation detection
    "column_gap_threshold": 20.0, // Column gap in points
    "use_xy_cut": true // XY-Cut algorithm for layout
  }
}
```

**Defaults**: Most fields have sensible defaults. Override only when needed.

---

## Verifying Extraction Quality

### Understanding Chunk Counts

After upload, check `chunk_count` to verify extraction succeeded:

```json
{
  "chunk_count": 45, // Number of semantic chunks created
  "entity_count": 23, // Number of entities extracted
  "relationship_count": 18
}
```

**What Affects Chunk Count**:

- PDF length (more pages → more chunks)
- Layout complexity (tables, figures → separate chunks)
- Text density (dense text → more chunks)

**Typical Ranges**:

- 10-page report: 20-40 chunks
- 50-page book: 100-200 chunks
- 100-page thesis: 300-500 chunks

**If chunk_count = 0**: Extraction failed. See [troubleshooting](#troubleshooting-quick-reference).

---

### Checking Extraction Details

Get detailed metadata about the document:

```bash
# Get document details
curl http://localhost:8080/api/v1/documents/doc-uuid-1234
```

**Response**:

```json
{
  "id": "doc-uuid-1234",
  "title": "Research Paper",
  "status": "indexed",
  "metadata": {
    "pages": 12,
    "tables_detected": 3,
    "figures": 5,
    "extraction_mode": "Text",
    "processing_time_ms": 2340
  },
  "chunks": [
    {
      "id": "chunk-1",
      "content": "Abstract: This paper presents...",
      "page": 1,
      "type": "text"
    },
    {
      "id": "chunk-2",
      "content": "| Column 1 | Column 2 |\n|----------|----------|\n| A | B |",
      "page": 3,
      "type": "table"
    }
  ]
}
```

**Key Metadata**:

- `tables_detected`: Number of tables found
- `figures`: Number of figures/images
- `extraction_mode`: Mode used (Text, Vision, Hybrid)
- Chunks array shows actual extracted content

---

### When to Retry with Different Config

**If chunk_count < expected**:

1. Check if PDF is scanned → Try Vision mode
2. Check if tables malformed → Enable `enhance_tables`
3. Check if text order wrong → Enable `detect_columns`

**Example Iteration**:

```bash
# First try: Default text mode
curl -F "file=@doc.pdf" http://localhost:8080/api/v1/documents
# Result: chunk_count = 5 (expected 50+) ❌

# Second try: Enable vision mode
curl -F "file=@doc.pdf" \
     -F 'config={"mode": "Vision"}' \
     http://localhost:8080/api/v1/documents
# Result: chunk_count = 52 ✅
```

---

## Common Patterns

### Pattern 1: Multi-Page Reports

**Scenario**: 50-page annual report with text + tables

**Approach**:

```bash
# Start with default
curl -X POST \
  -F "file=@annual_report.pdf" \
  -F "title=Annual Report 2024" \
  http://localhost:8080/api/v1/documents
```

**Check results**:

- If `tables_detected > 0` and chunks look good → ✅ Done
- If tables malformed → Re-upload with `enhance_tables: true`

**Large Document Tip**: Use `max_pages` to test on first 10 pages:

```bash
curl -F "file=@report.pdf" \
     -F 'config={"max_pages": 10}' \
     http://localhost:8080/api/v1/documents
```

---

### Pattern 2: Academic Papers (Multi-Column)

**Scenario**: Research paper with two-column layout, figures, equations

**Approach**:

```bash
# Enable column detection
curl -X POST \
  -F "file=@research_paper.pdf" \
  -F "title=AI Research Paper" \
  -F 'config={
    "layout": {"detect_columns": true},
    "extract_figure_captions": true
  }' \
  http://localhost:8080/api/v1/documents
```

**Tips**:

- Column detection ensures text reads left-to-right within columns
- Figure captions extracted separately for better context
- Equations may not extract perfectly (vision mode helps)

---

### Pattern 3: Scanned Books

**Scenario**: 200-page scanned book, faded text, skewed pages

**Approach**:

```bash
# Vision mode for scanned documents
curl -X POST \
  -F "file=@scanned_book.pdf" \
  -F "title=Historical Book" \
  -F 'config={
    "mode": "Vision",
    "vision_dpi": 150,
    "enhance_readability": true
  }' \
  http://localhost:8080/api/v1/documents
```

**Cost Estimate**: 200 pages × $0.005/page = $1.00 total

**Processing Time**: ~200 pages × 10 seconds/page = 33 minutes

**Tip**: For long books, use `max_pages` to test first 10 pages, then upload full book.

---

### Pattern 4: Financial Reports (Complex Tables)

**Scenario**: Quarterly report with merged cells, nested tables, footnotes

**Approach**:

```bash
# Enable table enhancement
curl -X POST \
  -F "file=@financial_report.pdf" \
  -F "title=Q4 2024 Financials" \
  -F 'config={
    "enhance_tables": true,
    "ai_temperature": 0.1
  }' \
  http://localhost:8080/api/v1/documents
```

**Expected Results**:

- Tables preserved in markdown format
- Merged cells handled correctly
- Footnotes linked to table cells

---

### Pattern 5: Non-English Documents

**Scenario**: PDF in Spanish, Chinese, Arabic

**Approach**:

```bash
# Vision mode with LLM handles non-English better
curl -X POST \
  -F "file=@spanish_doc.pdf" \
  -F "title=Documento en Español" \
  -F 'config={"mode": "Vision"}' \
  http://localhost:8080/api/v1/documents
```

**LLM Language Support**:

- OpenAI GPT-4o: 100+ languages
- Ollama: Depends on model (check model docs)

**Tip**: Vision mode typically handles non-English better than text mode due to font encoding issues.

---

## Troubleshooting Quick Reference

**See full guide**: [Common Issues - PDF Section](../troubleshooting/common-issues.md#pdf-extraction-issues)

### Quick Fixes Table

| Issue               | Solution                 | Config                                 |
| ------------------- | ------------------------ | -------------------------------------- |
| No text extracted   | Enable vision mode       | `{"mode": "Vision"}`                   |
| Tables broken       | Enable table enhancement | `{"enhance_tables": true}`             |
| Wrong text order    | Enable multi-column      | `{"layout": {"detect_columns": true}}` |
| chunk_count = 0     | Try vision mode          | `{"mode": "Vision"}`                   |
| Upload fails        | Check file size/format   | PDF only, < 50MB                       |
| Encoding errors (�) | Use vision mode          | `{"mode": "Vision"}`                   |

### Most Common Issues

**1. No text extracted (chunk_count = 0)**:

- **Cause**: PDF is image-based (scanned)
- **Solution**: `{"mode": "Vision"}`

**2. Tables not detected**:

- **Cause**: Complex table layout
- **Solution**: `{"enhance_tables": true}`

**3. Text order scrambled**:

- **Cause**: Multi-column layout
- **Solution**: `{"layout": {"detect_columns": true}}`

### When to Seek Help

- chunk_count still 0 after vision mode
- Specific table layout not detected
- Custom fonts not supported
- Upload fails repeatedly

**Next Steps**:

- Read [PDF Processing Deep Dive](../deep-dives/pdf-processing.md) for internals
- Check [Troubleshooting Guide](../troubleshooting/common-issues.md#pdf-extraction-issues) for detailed solutions
- File GitHub issue with PDF sample

---

## Next Steps

### Beginner Path

1. ✅ Uploaded first PDF (this tutorial)
2. ➡️ Read [Document Ingestion](document-ingestion.md) for chunking details
3. ➡️ Read [Query Optimization](query-optimization.md) for RAG techniques

### Advanced Path

1. ✅ Mastered PDF configuration (this tutorial)
2. ➡️ Read [PDF Processing Deep Dive](../deep-dives/pdf-processing.md) for algorithms
3. ➡️ Read [Contributing Guide](../contributing/development-setup.md) to improve PDF crate

### Troubleshooting Path

1. ⚠️ Encountered PDF extraction issues
2. ➡️ Read [Common Issues](../troubleshooting/common-issues.md#pdf-extraction-issues)
3. ➡️ File GitHub issue if problem persists

---

## Related Documentation

- [PDF Processing Deep Dive](../deep-dives/pdf-processing.md) - Algorithms and internals
- [Document Ingestion](document-ingestion.md) - Chunking and entity extraction
- [REST API Reference](../api-reference/rest-api.md#documents-api) - Complete API docs
- [Troubleshooting](../troubleshooting/common-issues.md#pdf-extraction-issues) - Error solutions
- [Quick Start](../quick-start.md) - Server setup
