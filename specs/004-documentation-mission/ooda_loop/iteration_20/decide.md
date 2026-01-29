# OODA Iteration 20: DECIDE

**Focus**: PDF Ingestion Tutorial - Implementation Plan
**Date**: 2026-01-29

---

## Decision Statement

**We will create a focused PDF ingestion tutorial (430 lines) that bridges the theory (iteration 19 deep dive) to practice, enabling 80% of users to successfully upload and query PDFs within 25 minutes.**

**Scope**:
1. Create `docs/tutorials/pdf-ingestion.md` (430 lines)
2. Update `docs/tutorials/document-ingestion.md` (add 150 lines)
3. Update `docs/troubleshooting/common-issues.md` (add 120 lines)

**Rationale**: High user impact, completes PDF documentation story, achievable in one iteration.

---

## Tutorial Structure: pdf-ingestion.md

### Full Table of Contents

```markdown
# PDF Ingestion Tutorial

## Introduction (50 lines)
- What you'll learn
- Prerequisites
- Time estimate (25 minutes)
- When to use this tutorial vs deep dive

## Quick Start: Your First PDF Upload (100 lines)
- Simplest possible example (curl)
- Verify upload succeeded
- Query the PDF content
- Understand the response
- ASCII Diagram: Upload flow

## Configuration Options (100 lines)
- Decision tree: When to use each option
- Basic config example
- Table enhancement example
- Vision mode example
- Performance tuning

## Verifying Extraction Quality (80 lines)
- Check quality metrics
- Interpret quality scores
- When quality is "good enough"
- When to adjust config

## Common Patterns (70 lines)
- Multi-page reports
- Mixed content (text + tables)
- Poor quality scans
- Non-English documents

## Troubleshooting Quick Reference (30 lines)
- Link to detailed troubleshooting
- Most common issues
- When to enable what

Total: 430 lines
```

---

## Section 1: Introduction (50 lines)

### Content Plan

```markdown
# PDF Ingestion Tutorial

EdgeQuake extracts text, tables, and metadata from PDF documents using advanced
layout analysis. This tutorial shows you how to upload PDFs and configure extraction.

**What You'll Learn**:
- Upload a PDF document (5 minutes)
- Configure extraction options (10 minutes)
- Verify extraction quality (5 minutes)
- Query PDF content (5 minutes)

**Prerequisites**:
- EdgeQuake server running (see quick-start.md)
- A PDF file to upload
- curl or httpie installed

**Time Estimate**: 25 minutes

**When to Read This**:
- First time uploading PDFs → **Read this tutorial**
- Understanding extraction internals → See [PDF Processing Deep Dive](../deep-dives/pdf-processing.md)
- Advanced table detection → See deep dive
- Troubleshooting → See [Common Issues](../troubleshooting/common-issues.md)

**Theory vs Practice**:
- This tutorial: "How do I upload and configure?"
- Deep dive: "How does table detection work internally?"
- Both are valuable - start here, dig deeper as needed.
```

**Lines**: 50  
**Examples**: 0  
**ASCII**: 0

---

## Section 2: Quick Start (100 lines)

### Content Plan

#### 2.1 Simplest Upload (40 lines)

```markdown
## Quick Start: Your First PDF Upload

### Step 1: Upload the PDF

```bash
# Upload with default settings
curl -X POST \
  -F "file=@/path/to/paper.pdf" \
  http://localhost:8080/api/v1/workspaces/default/upload
```

**What Happens**:
```
Upload → Extract text → Extract tables → Build knowledge graph → Return
```

**Response**:
```json
{
  "document_id": "doc_1234",
  "status": "completed",
  "pages_processed": 12,
  "tables_detected": 3,
  "extraction_quality": 0.89
}
```

**Key Fields**:
- `document_id`: Use this to query the document
- `status`: `completed` means success
- `extraction_quality`: 0.8+ is good, 0.9+ is excellent
```

#### 2.2 Verify Upload (30 lines)

```markdown
### Step 2: Verify Upload Succeeded

```bash
# Check document status
curl http://localhost:8080/api/v1/workspaces/default/documents/doc_1234
```

**Look for**:
- ✅ `status: "indexed"` - ready to query
- ✅ `chunks_created > 0` - text extracted
- ✅ `entities_extracted > 0` - knowledge graph built
- ⚠️ `status: "failed"` - see troubleshooting

**Tip**: If extraction quality < 0.7, consider enabling enhancements (see Configuration).
```

#### 2.3 Query Content (30 lines)

```markdown
### Step 3: Query the PDF Content

```bash
# Ask a question about the document
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"query": "What are the key findings?", "mode": "hybrid"}' \
  http://localhost:8080/api/v1/workspaces/default/query
```

**Response**:
```json
{
  "answer": "The key findings are...",
  "sources": [
    {"document_id": "doc_1234", "page": 3, "relevance": 0.94}
  ]
}
```

**Success**: You've uploaded, indexed, and queried a PDF in < 5 minutes! 🎉
```

**Lines**: 100  
**Examples**: 3 (upload, verify, query)  
**ASCII**: 0 (simple text flow)

---

## Section 3: Configuration Options (100 lines)

### Content Plan

#### 3.1 Decision Tree (40 lines)

```markdown
## Configuration Options

### When to Use What

**Default Settings** (no config needed):
- ✅ Good quality digital PDFs
- ✅ Simple text documents
- ✅ Standard fonts
- **Quality Score**: Usually 0.8+

**Enable `enhance_tables: true`** when:
- ⚠️ Complex table layouts
- ⚠️ Merged cells
- ⚠️ Nested tables
- **Trade-off**: 2x slower, but better table accuracy
- **Quality Score**: Improves 0.7 → 0.85

**Enable `vision_mode: true`** when:
- ⚠️ Scanned documents (images)
- ⚠️ Poor quality PDFs
- ⚠️ No text layer
- **Trade-off**: 10x slower, LLM API cost ($0.001-0.01/page)
- **Quality Score**: Improves 0.5 → 0.8+

**Enable `multi_column: true`** when:
- ⚠️ Newspaper-style layout
- ⚠️ Academic papers (two-column)
- **Trade-off**: Slight overhead, better text order
```

#### 3.2 Config Examples (60 lines)

```markdown
### Example 1: Basic Configuration

```bash
curl -X POST \
  -F "file=@report.pdf" \
  -F 'config={"enhance_tables": false, "vision_mode": false}' \
  http://localhost:8080/api/v1/workspaces/default/upload
```

**Use for**: 80% of digital PDFs

---

### Example 2: Enhanced Table Detection

```bash
curl -X POST \
  -F "file=@financial_report.pdf" \
  -F 'config={"enhance_tables": true, "llm_provider": "openai"}' \
  http://localhost:8080/api/v1/workspaces/default/upload
```

**Use for**: PDFs with complex tables (spreadsheets, financial data)

**Result**: Tables with merged cells correctly detected

---

### Example 3: Vision Mode (Scanned PDFs)

```bash
curl -X POST \
  -F "file=@scanned_book.pdf" \
  -F 'config={"vision_mode": true, "llm_provider": "openai"}' \
  http://localhost:8080/api/v1/workspaces/default/upload
```

**Use for**: Scanned documents, poor quality PDFs

**Cost**: ~$0.001-0.01 per page (OpenAI GPT-4 Vision)

**Result**: Text extracted from images via OCR

---

### Example 4: Full Enhancement

```bash
curl -X POST \
  -F "file=@complex_report.pdf" \
  -F 'config={"enhance_tables": true, "vision_mode": true, "multi_column": true}' \
  http://localhost:8080/api/v1/workspaces/default/upload
```

**Use for**: Critical documents where accuracy > speed

**Trade-off**: 10x slower, LLM API cost
```

**Lines**: 100  
**Examples**: 4 (basic, tables, vision, full)  
**ASCII**: 0

---

## Section 4: Verifying Quality (80 lines)

### Content Plan

#### 4.1 Quality Metrics (40 lines)

```markdown
## Verifying Extraction Quality

### Understanding Quality Scores

**extraction_quality** (0.0 - 1.0):
- **0.9 - 1.0**: Excellent - use as-is
- **0.8 - 0.9**: Good - sufficient for most use cases
- **0.7 - 0.8**: Fair - consider enabling `enhance_tables`
- **0.5 - 0.7**: Poor - enable `vision_mode` or `enhance_tables`
- **< 0.5**: Very poor - enable both enhancements

**What Affects Quality**:
- PDF quality (digital vs scanned)
- Font complexity (standard vs custom)
- Table complexity (simple vs nested)
- Layout (single vs multi-column)

**Example Output**:
```json
{
  "extraction_quality": 0.87,
  "quality_breakdown": {
    "text_extraction": 0.95,
    "table_detection": 0.78,
    "layout_analysis": 0.88
  }
}
```
```

#### 4.2 Quality Improvement (40 lines)

```markdown
### When Quality is "Good Enough"

**For Search/RAG** (you need this tutorial):
- ✅ 0.8+ is sufficient
- Text retrieval doesn't require perfect extraction
- Entity extraction is robust to minor errors

**For Data Extraction** (tables, structured data):
- ✅ 0.9+ recommended
- Use `enhance_tables: true`
- Verify table contents manually

**For Archival/Legal**:
- ✅ 0.95+ required
- Use full enhancements
- Consider manual verification

### Improving Quality

**If quality < 0.8**:
1. Re-upload with `enhance_tables: true`
2. If still < 0.8, try `vision_mode: true`
3. If still poor, PDF may have complex layout

**Example Iteration**:
```bash
# First try: 0.72 (fair)
curl -F "file=@doc.pdf" http://...

# Second try: 0.85 (good) ✅
curl -F "file=@doc.pdf" -F 'config={"enhance_tables": true}' http://...
```
```

**Lines**: 80  
**Examples**: 2 (quality breakdown JSON, iteration)  
**ASCII**: 0

---

## Section 5: Common Patterns (70 lines)

### Content Plan

```markdown
## Common Patterns

### Pattern 1: Multi-Page Reports

**Scenario**: 50-page annual report with text + tables

**Approach**:
```bash
# Use default first, check quality
curl -F "file=@annual_report.pdf" http://localhost:8080/api/v1/workspaces/default/upload

# If tables are complex, re-upload with enhancement
curl -F "file=@annual_report.pdf" \
     -F 'config={"enhance_tables": true}' \
     http://localhost:8080/api/v1/workspaces/default/upload
```

**Tip**: Large documents benefit from batching (see API reference).

---

### Pattern 2: Mixed Content (Text + Tables)

**Scenario**: Research paper with figures, tables, equations

**Approach**:
```bash
# Enable multi-column for academic papers
curl -F "file=@research_paper.pdf" \
     -F 'config={"multi_column": true, "enhance_tables": true}' \
     http://localhost:8080/api/v1/workspaces/default/upload
```

**Tip**: Equations may not extract perfectly - vision mode helps.

---

### Pattern 3: Poor Quality Scans

**Scenario**: Scanned book, faded text, skewed pages

**Approach**:
```bash
# Vision mode for scanned documents
curl -F "file=@scanned_book.pdf" \
     -F 'config={"vision_mode": true}' \
     http://localhost:8080/api/v1/workspaces/default/upload
```

**Cost**: ~$0.01 per page for 200-page book = $2 total

---

### Pattern 4: Non-English Documents

**Scenario**: PDF in Spanish, Chinese, Arabic

**Approach**:
```bash
# Vision mode handles non-English better
curl -F "file=@spanish_doc.pdf" \
     -F 'config={"vision_mode": true, "llm_provider": "openai"}' \
     http://localhost:8080/api/v1/workspaces/default/upload
```

**Tip**: Ensure LLM supports target language (OpenAI supports 100+ languages).
```

**Lines**: 70  
**Examples**: 4 (reports, mixed, scans, non-English)  
**ASCII**: 0

---

## Section 6: Troubleshooting Quick Reference (30 lines)

### Content Plan

```markdown
## Troubleshooting Quick Reference

**See full guide**: [Common Issues - PDF Section](../troubleshooting/common-issues.md#pdf-extraction)

**Quick fixes**:

| Issue | Solution | Config |
|-------|----------|--------|
| No text extracted | Enable vision mode | `{"vision_mode": true}` |
| Tables broken | Enable table enhancement | `{"enhance_tables": true}` |
| Wrong text order | Enable multi-column | `{"multi_column": true}` |
| Quality < 0.7 | Try enhancements | Both options |
| Upload fails | Check file size/format | PDF only, < 100MB |

**When to Seek Help**:
- Quality still < 0.5 after enhancements
- Specific table layout not detected
- Custom fonts not supported

**Next Steps**:
- Read [PDF Processing Deep Dive](../deep-dives/pdf-processing.md) for internals
- Check [Common Issues](../troubleshooting/common-issues.md) for detailed troubleshooting
- File GitHub issue with PDF sample
```

**Lines**: 30  
**Examples**: 1 (table)  
**ASCII**: 0

---

## ASCII Diagram: Upload Flow

### Diagram Plan

**Purpose**: Show user what happens during upload

**Complexity**: Simple flowchart

**Location**: Section 2 (Quick Start)

**Design**:

```
┌─────────────────────────────────────────────────────────────────┐
│                      PDF Upload Flow                            │
└─────────────────────────────────────────────────────────────────┘

  User                EdgeQuake Server                  Knowledge Graph
   │                          │                                 │
   │  POST /upload           │                                 │
   │  (file + config)        │                                 │
   ├────────────────────────>│                                 │
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
   │                          │ 4. Calculate Quality            │
   │                          │    (metrics: 0.0-1.0)           │
   │                          │                                 │
   │                          │ 5. Build Chunks                 │
   │                          │    (semantic units)             │
   │                          │                                 │
   │                          │ 6. Extract Entities            │
   │                          ├────────────────────────────────>│
   │                          │    (people, orgs, concepts)    │
   │                          │                                 │
   │                          │ 7. Index for Search            │
   │                          │<────────────────────────────────┤
   │  Response:              │                                 │
   │  {document_id, status,  │                                 │
   │   quality: 0.89}        │                                 │
   │<────────────────────────┤                                 │
   │                          │                                 │
   │  Query request          │                                 │
   ├────────────────────────>│ 8. Query Graph                  │
   │                          ├────────────────────────────────>│
   │                          │    (find relevant chunks)       │
   │  Response:              │<────────────────────────────────┤
   │  {answer, sources}      │                                 │
   │<────────────────────────┤                                 │

Total time: 2-5 seconds (default) | 20-50 seconds (enhanced)
```

**Lines**: ~35 lines

**Annotations**:
- Clear step numbers (1-8)
- Time estimate at bottom
- Shows quality score calculation
- Emphasizes knowledge graph integration

---

## Update Plan: document-ingestion.md

### Insertion Point

**Current structure** (from observe.md):
```
1. Introduction
2. Step 1: Understanding Chunks
3. Step 2: Entity Extraction
4. Step 3: Customizing Pipeline
5. Monitoring and Troubleshooting
```

**Insert after**: Introduction (before Step 1)

**New section title**: "Working with PDF Documents"

### Content Plan (150 lines)

```markdown
## Working with PDF Documents

EdgeQuake has advanced PDF extraction capabilities using layout analysis and
optional LLM enhancement. This section provides a quick overview - see the
[PDF Ingestion Tutorial](pdf-ingestion.md) for complete details.

### Quick Example

```bash
# Upload a PDF with default settings
curl -X POST \
  -F "file=@research_paper.pdf" \
  http://localhost:8080/api/v1/workspaces/default/upload
```

**What Gets Extracted**:
- ✅ Text (with layout preservation)
- ✅ Tables (with structure)
- ✅ Metadata (title, author, pages)
- ✅ Quality metrics (0.0-1.0 score)

### PDF Configuration Options

**Basic Upload** (80% of cases):
```bash
curl -F "file=@doc.pdf" http://localhost:8080/api/v1/workspaces/default/upload
```

**Enhanced Table Detection** (complex tables):
```bash
curl -F "file=@doc.pdf" \
     -F 'config={"enhance_tables": true}' \
     http://localhost:8080/api/v1/workspaces/default/upload
```

**Vision Mode** (scanned PDFs):
```bash
curl -F "file=@doc.pdf" \
     -F 'config={"vision_mode": true}' \
     http://localhost:8080/api/v1/workspaces/default/upload
```

### PDF-Specific Chunking Strategies

When EdgeQuake processes PDFs, chunks are created based on:
- Paragraphs (text flow)
- Tables (entire table = one chunk)
- Sections (detected via headings)

**Example** (research paper):
- Page 1: Abstract → 1 chunk
- Page 2-3: Introduction (3 paragraphs) → 3 chunks
- Page 4: Table 1 → 1 chunk
- Page 5-6: Methods (5 paragraphs) → 5 chunks

**Total**: 10 chunks from 6 pages

### PDF Entity Extraction

Entities extracted from PDFs include:
- **People**: Authors, researchers mentioned
- **Organizations**: Universities, companies
- **Concepts**: Domain terms, methods, metrics
- **Relationships**: "AuthorOf", "AffiliatedWith", "Cites"

**Example** (from PDF metadata):
```
Dr. Jane Smith (PERSON) → WorksAt → MIT (ORGANIZATION)
MIT (ORGANIZATION) → Published → "AI Safety" (CONCEPT)
```

### PDF Quality Metrics

After upload, check `extraction_quality`:
- **0.9-1.0**: Excellent - no action needed
- **0.8-0.9**: Good - use as-is
- **0.7-0.8**: Fair - consider `enhance_tables`
- **<0.7**: Poor - enable enhancements

**Tip**: Quality affects retrieval accuracy. If < 0.8, re-upload with enhancements.

### When to Read the Full PDF Tutorial

**Read this section** if:
- First time uploading PDFs
- Quick reference needed

**Read [PDF Ingestion Tutorial](pdf-ingestion.md)** if:
- Complex PDFs (tables, scans)
- Need detailed configuration
- Troubleshooting extraction issues
- Understanding quality metrics

**Read [PDF Processing Deep Dive](../deep-dives/pdf-processing.md)** if:
- Understanding internal algorithms
- Custom table detection logic
- Contributing to PDF crate

### PDF Troubleshooting Quick Reference

**No text extracted**:
- ✅ Try `vision_mode: true`
- ✅ Check PDF has text layer (not just images)

**Tables not detected**:
- ✅ Try `enhance_tables: true`
- ✅ Check table has clear borders

**Wrong text order**:
- ✅ Try `multi_column: true`
- ✅ Academic papers need this

**More details**: See [PDF Troubleshooting](../troubleshooting/common-issues.md#pdf-extraction)

---

## Step 1: Understanding Chunks

[Existing content continues...]
```

**Lines**: 150  
**Examples**: 5 (basic, enhanced, vision, chunking, entities)  
**ASCII**: 0  
**Links**: 3 (tutorial, deep dive, troubleshooting)

---

## Update Plan: common-issues.md

### Insertion Point

**Current structure** (from observe.md):
```
1. Server Startup Issues
2. Document Processing Issues
3. Query Issues
4. Performance Issues
5. Database Issues
```

**Insert after**: Document Processing Issues (new section 3)

**New section title**: "PDF Extraction Issues"

### Content Plan (120 lines)

```markdown
## PDF Extraction Issues

### Issue 1: No Text Extracted

**Symptom**: `extraction_quality: 0.0` or empty chunks

**Cause**: PDF is image-based (no text layer)

**Solution**:
```bash
# Enable vision mode to extract text from images
curl -F "file=@scanned_doc.pdf" \
     -F 'config={"vision_mode": true, "llm_provider": "openai"}' \
     http://localhost:8080/api/v1/workspaces/default/upload
```

**Cost**: ~$0.001-0.01 per page (OpenAI GPT-4 Vision)

**Verification**:
- Check `extraction_quality` > 0.7
- Check `chunks_created` > 0

---

### Issue 2: Tables Not Detected

**Symptom**: Table data mixed with regular text

**Cause**: Complex table layout (merged cells, nested tables)

**Solution**:
```bash
# Enable LLM-enhanced table detection
curl -F "file=@financial_report.pdf" \
     -F 'config={"enhance_tables": true, "llm_provider": "openai"}' \
     http://localhost:8080/api/v1/workspaces/default/upload
```

**Verification**:
- Check `tables_detected` > 0
- Check `quality_breakdown.table_detection` > 0.8

**Limitations**:
- Very complex tables (5+ merged cells) may still fail
- Tables without borders harder to detect

---

### Issue 3: Wrong Text Order (Multi-Column Layout)

**Symptom**: Text from different columns interleaved

**Example**:
```
# PDF layout:
Column 1: "The results show..."
Column 2: "In conclusion..."

# Extracted (wrong order):
"The results In show... conclusion..."
```

**Cause**: PDF has multi-column layout (newspapers, academic papers)

**Solution**:
```bash
# Enable multi-column detection
curl -F "file=@research_paper.pdf" \
     -F 'config={"multi_column": true}' \
     http://localhost:8080/api/v1/workspaces/default/upload
```

**Verification**:
- Read first few chunks - text should be in correct order

---

### Issue 4: Encoding Errors (Special Characters)

**Symptom**: `�` or `?` characters in extracted text

**Cause**: PDF uses custom fonts or non-standard encoding

**Solution**:
```bash
# LLM enhancement handles encoding issues
curl -F "file=@doc.pdf" \
     -F 'config={"vision_mode": true}' \
     http://localhost:8080/api/v1/workspaces/default/upload
```

**Alternative**: If vision mode too slow/expensive, check PDF font embedding

**Verification**:
- Check extracted text for correct characters

---

### Issue 5: Low Quality Score (< 0.7)

**Symptom**: `extraction_quality < 0.7` after upload

**Diagnosis**:
1. Check `quality_breakdown` in response
2. Identify weak component (text, tables, layout)

**Solutions**:

**Low text extraction** (< 0.7):
```bash
# Try vision mode
curl -F "file=@doc.pdf" -F 'config={"vision_mode": true}' http://...
```

**Low table detection** (< 0.7):
```bash
# Try enhanced tables
curl -F "file=@doc.pdf" -F 'config={"enhance_tables": true}' http://...
```

**Low layout analysis** (< 0.7):
```bash
# Try multi-column + enhancements
curl -F "file=@doc.pdf" \
     -F 'config={"multi_column": true, "enhance_tables": true}' \
     http://...
```

**If still < 0.7**: PDF may have complex layout - file GitHub issue with sample

---

### When to Use Which Enhancement

**Decision tree**:
```
Is quality < 0.8?
  ├─ Yes: Check quality_breakdown
  │   ├─ text_extraction < 0.7? → Enable vision_mode
  │   ├─ table_detection < 0.7? → Enable enhance_tables
  │   └─ layout_analysis < 0.7? → Enable multi_column
  └─ No: Use default settings (fast, free)
```

**Trade-offs**:
- `vision_mode`: 10x slower, $0.001-0.01/page
- `enhance_tables`: 2x slower, $0.0001/page
- `multi_column`: Minimal overhead, free

---

### Advanced: Custom PDF Extraction

For programmatic control, use the Rust API:

```rust
use edgequake_pdf::{PdfExtractor, PdfConfig};

let config = PdfConfig {
    enhance_tables: true,
    vision_mode: false,
    multi_column: true,
    ..Default::default()
};

let extractor = PdfExtractor::new(config);
let result = extractor.extract_full("path/to/doc.pdf").await?;

println!("Quality: {}", result.quality);
println!("Tables: {}", result.tables.len());
```

**See**: [PDF Processing Deep Dive](../deep-dives/pdf-processing.md) for full API reference

---

## Query Issues

[Existing content continues...]
```

**Lines**: 120  
**Examples**: 6 (vision, tables, multi-column, encoding, quality, Rust API)  
**ASCII**: 1 (decision tree)  
**Links**: 1 (deep dive)

---

## Verification Checklist

### Before Commit

**Content Verification**:
- [ ] All code examples tested against actual API
- [ ] All config field names match edgequake-pdf/src/config.rs
- [ ] All endpoint URLs match edgequake-api/src/routes.rs
- [ ] All quality metrics match actual response format
- [ ] All links point to existing files

**Code Example Sources**:
- [ ] Tutorial examples verified against examples/production_pipeline.rs
- [ ] Rust API examples verified against edgequake-pdf/src/lib.rs
- [ ] curl examples verified against API documentation

**Completeness**:
- [ ] Tutorial: 430+ lines ✅
- [ ] document-ingestion update: 150 lines ✅
- [ ] common-issues update: 120 lines ✅
- [ ] Total: 700+ lines ✅
- [ ] ASCII diagrams: 1-2 ✅
- [ ] Code examples: 10+ ✅

**Quality**:
- [ ] Tutorial actionable in < 25 minutes
- [ ] No duplicate content (tutorial vs deep dive)
- [ ] Clear links between docs
- [ ] Troubleshooting covers 80% of issues
- [ ] Examples use real file paths and endpoints

**Integration**:
- [ ] Tutorial discoverable from document-ingestion.md
- [ ] Deep dive linked from tutorial
- [ ] Troubleshooting linked from both
- [ ] No broken cross-references

---

## File-by-File Verification Plan

### File 1: docs/tutorials/pdf-ingestion.md

**Verify Against**:
- `edgequake-pdf/src/config.rs` - Config field names
- `edgequake-api/src/routes/upload.rs` - Endpoint URL
- `examples/production_pipeline.rs` - Usage patterns

**Check**:
- [ ] Config JSON matches PdfConfig struct
- [ ] Endpoint URL correct (POST /api/v1/workspaces/{workspace}/upload)
- [ ] Response fields match actual API
- [ ] Quality metric ranges accurate

### File 2: docs/tutorials/document-ingestion.md

**Verify Against**:
- Existing content (lines 1-519)
- Tutorial (for consistency)

**Check**:
- [ ] PDF section fits existing style
- [ ] Links to tutorial correct
- [ ] No duplicate content
- [ ] Smooth flow from existing content

### File 3: docs/troubleshooting/common-issues.md

**Verify Against**:
- `edgequake-pdf/src/errors.rs` - Error types
- Tutorial (for consistency)

**Check**:
- [ ] Error symptoms match actual errors
- [ ] Solutions match config options
- [ ] Decision tree accurate
- [ ] Links to tutorial/deep dive correct

---

## Estimated Line Count Summary

| File | Section | Lines |
|------|---------|-------|
| **pdf-ingestion.md** | Introduction | 50 |
| | Quick Start | 100 |
| | Configuration | 100 |
| | Verifying Quality | 80 |
| | Common Patterns | 70 |
| | Troubleshooting | 30 |
| | **Subtotal** | **430** |
| **document-ingestion.md** | PDF Section | 150 |
| **common-issues.md** | PDF Extraction | 120 |
| **TOTAL** | | **700** |

**Exceeds Goal**: ✅ 700 lines > 670 line target

---

## Implementation Order

### Phase 1: Core Tutorial (Priority P0)
1. Create pdf-ingestion.md skeleton
2. Write Introduction (50 lines)
3. Write Quick Start (100 lines)
4. Write Configuration (100 lines)
5. Write Verifying Quality (80 lines)
6. Add ASCII diagram (35 lines)

**Checkpoint**: 365 lines, ~2 hours

### Phase 2: Patterns & Integration (Priority P1)
7. Write Common Patterns (70 lines)
8. Write Troubleshooting Quick Reference (30 lines)
9. Update document-ingestion.md (150 lines)

**Checkpoint**: 615 lines, ~1.5 hours

### Phase 3: Troubleshooting (Priority P0)
10. Update common-issues.md (120 lines)

**Checkpoint**: 735 lines, ~1 hour

### Phase 4: Verification
11. Verify all code examples
12. Test all links
13. Check consistency
14. Polish writing

**Checkpoint**: ~1 hour

**Total Time**: ~5.5 hours (within iteration budget)

---

## Risk Mitigation

**Risk**: Tutorial too long, users feel overwhelmed  
**Mitigation**: Clear section headers, "Read this section if..." callouts

**Risk**: Examples don't work  
**Mitigation**: Verify against actual API before commit

**Risk**: Duplicate content with deep dive  
**Mitigation**: Tutorial = "How", Deep dive = "Why"

**Risk**: Troubleshooting incomplete  
**Mitigation**: Focus on 80% of issues, link to GitHub for edge cases

---

## Success Metrics (Repeated from ORIENT)

**Quantitative**:
- ✅ 700+ lines (exceeds 670 target)
- ✅ 10+ code examples
- ✅ 1 ASCII diagram
- ✅ 3 files updated/created

**Qualitative**:
- ✅ User can upload PDF in < 5 minutes
- ✅ User can troubleshoot 80% of issues
- ✅ Clear progression: basic → advanced
- ✅ Best-in-class PDF documentation

---

## Final Decision

**Recommendation**: ✅ **PROCEED TO ACT PHASE**

**Confidence**: 98% - Detailed plan ready, all content mapped

**Next**: ACT phase to implement all 3 files and verify
