# Tutorial: Document Ingestion Deep-Dive

> **Understanding and Customizing the Document Pipeline**

This tutorial explores EdgeQuake's document processing pipeline in depth, covering chunking strategies, entity extraction, and how to optimize for your use case.

**Time**: ~25 minutes  
**Level**: Intermediate  
**Prerequisites**: Completed [First RAG App](first-rag-app.md)

---

## The Ingestion Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                   DOCUMENT INGESTION PIPELINE                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Document ─────────────────────────────────────────────────────▶ 
│      │                                                          │
│      ▼                                                          │
│  ┌─────────────┐                                                │
│  │  1. Parse   │ Extract text from PDF, DOCX, TXT, HTML         │
│  └──────┬──────┘                                                │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────┐                                                │
│  │  2. Chunk   │ Split into semantic units (1200 tokens default)│
│  └──────┬──────┘                                                │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────┐                                                │
│  │ 3. Extract  │ LLM extracts entities + relationships          │
│  │   (per chunk)│ Runs in parallel                              │
│  └──────┬──────┘                                                │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────┐                                                │
│  │ 4. Normalize│ Deduplicate entities, merge descriptions       │
│  └──────┬──────┘                                                │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────┐                                                │
│  │  5. Embed   │ Generate embeddings for chunks + entities      │
│  └──────┬──────┘                                                │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────┐                                                │
│  │  6. Store   │ Save to PostgreSQL (pgvector + AGE)            │
│  └─────────────┘                                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Working with PDF Documents

EdgeQuake has advanced PDF extraction capabilities using layout analysis and optional LLM enhancement. This section provides a quick overview - see the [PDF Ingestion Tutorial](pdf-ingestion.md) for complete details.

### Quick PDF Upload Example

```bash
# Upload a PDF with default settings (text mode)
curl -X POST "http://localhost:8080/api/v1/documents/upload" \
  -F "file=@research_paper.pdf" \
  -F "title=AI Research Paper"
```

**What Gets Extracted**:

- ✅ Text (with layout preservation)
- ✅ Tables (with structure detected)
- ✅ Metadata (pages, author, title)
- ✅ Multi-column layouts (academic papers)

**Response**:

```json
{
  "id": "doc-uuid",
  "title": "AI Research Paper",
  "status": "completed",
  "chunk_count": 45,
  "metadata": {
    "pages": 12,
    "tables_detected": 3
  }
}
```

---

### PDF Configuration Modes

EdgeQuake supports three extraction modes:

**Text Mode** (default, fastest):

```bash
# Automatic text extraction from digital PDFs
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@doc.pdf"
```

- Use for: Good quality digital PDFs
- Processing: 2-5 seconds
- Cost: Free

**Vision Mode** (scanned documents):

```bash
# LLM-based OCR for scanned/image PDFs
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@scanned_book.pdf" \
  -F 'config={"mode": "Vision"}'
```

- Use for: Scanned documents, poor quality PDFs
- Processing: 20-50 seconds
- Cost: ~$0.001-0.01 per page

**Hybrid Mode** (automatic quality detection):

```bash
# Automatic fallback to vision for low-quality pages
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@mixed_quality.pdf" \
  -F 'config={"mode": "Hybrid", "quality_threshold": 0.7}'
```

- Use for: Unknown PDF quality
- Processing: Variable (2-50 seconds)
- Cost: Only low-quality pages incur LLM cost

---

### Enhanced Table Detection

For complex tables (merged cells, nested structures):

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@financial_report.pdf" \
  -F 'config={"enhance_tables": true}'
```

**Before** (text mode):

```
Column1 Header Column2 Header
Data1a Data1b Data2a
Data2b Data3a Data3b
```

**After** (enhanced):

```markdown
| Column 1 Header | Column 2 Header |
| --------------- | --------------- |
| Data 1a         | Data 1b         |
| Data 2a         | Data 2b         |
| Data 3a         | Data 3b         |
```

**Trade-off**: 2x slower, ~$0.0001 per table, but significantly better accuracy.

---

### PDF-Specific Chunking Strategies

When EdgeQuake processes PDFs, chunks are created based on document structure:

**Text Content**:

- Paragraphs → Individual chunks
- Sections → Detected via headings
- Reading order → Preserved with layout analysis

**Tables**:

- Entire table → Single chunk
- Preserves cell relationships
- Includes caption if present

**Figures**:

- Caption → Separate chunk
- Image description (if vision mode enabled)

**Example** (12-page research paper):

```
Page 1:  Abstract                  → 1 chunk
Page 2-3: Introduction (4 paras)    → 4 chunks
Page 4:   Table 1                   → 1 chunk
Page 5-7: Methods (6 paras)         → 6 chunks
Page 8:   Figure 2 caption          → 1 chunk
Page 9-11: Results (8 paras + table) → 9 chunks
Page 12:  Conclusion               → 2 chunks

Total: 24 chunks from 12 pages
```

**Tip**: PDF chunks tend to be more structured than plain text chunks due to layout analysis.

---

### PDF Entity Extraction

Entities extracted from PDFs include document-specific elements:

**From Content**:

- Authors, researchers, organizations
- Methods, concepts, metrics
- Locations, datasets

**From Metadata**:

- PDF title → Document entity
- Author field → Person entities
- Creation date → Temporal entity

**Example** (from PDF metadata):

```
Dr. Jane Smith (PERSON) → AuthorOf → "AI Safety Paper" (DOCUMENT)
"AI Safety Paper" (DOCUMENT) → PublishedBy → MIT (ORGANIZATION)
MIT (ORGANIZATION) → LocatedIn → Boston (LOCATION)
```

**Relationship Graph**:

```
Jane Smith ───AuthorOf──▶ Paper ───Cites──▶ Related Work
     │                       │
     │                       │
  WorksAt                 AboutTopic
     │                       │
     ▼                       ▼
    MIT              "Reinforcement Learning"
```

---

### Verifying PDF Extraction Quality

After PDF upload, check extraction metrics:

```bash
curl http://localhost:8080/api/v1/documents/doc-uuid
```

**Response**:

```json
{
  "id": "doc-uuid",
  "metadata": {
    "pages": 12,
    "tables_detected": 3,
    "extraction_mode": "Text"
  },
  "chunk_count": 24,
  "entity_count": 18
}
```

**Quality Indicators**:

- ✅ `chunk_count` matches expected (roughly 2-3 chunks per page)
- ✅ `tables_detected > 0` if PDF has tables
- ✅ `entity_count > 0` indicates successful extraction

**If chunk_count = 0**:

1. Try Vision mode: `{"mode": "Vision"}`
2. Check if PDF is encrypted/protected
3. See [PDF Troubleshooting](../troubleshooting/common-issues.md#pdf-extraction-issues)

---

### PDF Configuration Reference

Common configuration options:

```json
{
  "mode": "Text", // Text | Vision | Hybrid
  "enhance_tables": false, // Enable LLM table refinement
  "quality_threshold": 0.5, // Hybrid mode threshold
  "layout": {
    "detect_columns": true, // Multi-column detection
    "detect_tables": true, // Table detection
    "column_gap_threshold": 20.0 // Column separation (points)
  },
  "vision_dpi": 150, // DPI for vision mode
  "max_pages": null, // Limit pages (null = all)
  "normalize_spacing": true, // Fix concatenated words
  "extract_figure_captions": true // Extract figure captions
}
```

---

### When to Read the Full PDF Tutorial

**Read this section** if:

- First time with EdgeQuake
- Quick reference for PDF upload

**Read [PDF Ingestion Tutorial](pdf-ingestion.md)** if:

- Complex PDFs (tables, scans, multi-column)
- Need detailed configuration guidance
- Troubleshooting extraction issues
- Understanding quality metrics

**Read [PDF Processing Deep Dive](../deep-dives/pdf-processing.md)** if:

- Understanding internal algorithms
- XY-Cut layout analysis details
- Table detection clustering logic
- Contributing to PDF crate

---

### PDF Troubleshooting Quick Reference

**No text extracted**:

- ✅ Try `{"mode": "Vision"}` for scanned PDFs
- ✅ Check PDF is not encrypted

**Tables not detected**:

- ✅ Enable `{"enhance_tables": true}`
- ✅ Verify tables have clear borders

**Wrong text order**:

- ✅ Enable `{"layout": {"detect_columns": true}}`
- ✅ Academic papers benefit from column detection

**More details**: See [PDF Troubleshooting](../troubleshooting/common-issues.md#pdf-extraction-issues)

---

## Step 1: Understanding Chunks

Chunks are the atomic units of retrieval. Too small = missing context. Too large = noise in results.

### Default Chunking

EdgeQuake uses sliding window chunking by default:

- **Chunk size**: 1200 tokens (default)
- **Overlap**: 100 tokens (~8%)
- **Strategy**: Semantic boundaries (sentences, paragraphs)

### Inspect Chunk Output

After uploading a document, view its chunks:

```bash
curl "http://localhost:8080/api/v1/documents/doc_xyz789/chunks"
```

**Response:**

```json
{
  "chunks": [
    {
      "id": "chunk_001",
      "content": "TechCorp Innovation Labs was founded in 2020 by Sarah Chen and Marcus Williams. The company is headquartered in San Francisco, with research offices in Boston and Seattle.",
      "position": 0,
      "token_count": 42,
      "embedding_id": "emb_abc123"
    },
    {
      "id": "chunk_002",
      "content": "Sarah Chen serves as CEO and leads the company's AI research initiatives. She previously worked at Google DeepMind where she led the language model team.",
      "position": 1,
      "token_count": 38,
      "embedding_id": "emb_def456"
    }
  ],
  "total_chunks": 8
}
```

---

## Step 2: Custom Chunking Strategies

Different document types benefit from different chunking approaches:

### Strategy Comparison

| Strategy      | Best For             | Chunk Size    |
| ------------- | -------------------- | ------------- |
| **Fixed**     | General text         | 1200 tokens (default) |
| **Semantic**  | Well-structured docs | Variable      |
| **Paragraph** | Articles, blogs      | 1 paragraph   |
| **Sentence**  | Q&A, definitions     | 1-3 sentences |

### Using Custom Chunk Size

```bash
curl -X POST "http://localhost:8080/api/v1/documents?workspace_id=$WORKSPACE_ID" \
  -F "file=@large_document.pdf" \
  -F "title=Technical Manual" \
  -F "chunk_size=1024" \
  -F "chunk_overlap=100"
```

### When to Adjust

| Scenario            | Recommendation                   |
| ------------------- | -------------------------------- |
| Long technical docs | Increase to 1024 tokens          |
| Short FAQs          | Decrease to 256 tokens           |
| Legal contracts     | Use paragraph chunking           |
| Code documentation  | Use semantic with code awareness |

---

## Step 3: Entity Extraction

The LLM extracts entities and relationships from each chunk.

### Default Entity Types

EdgeQuake extracts these entity types by default:

- **PERSON** - Named individuals
- **ORGANIZATION** - Companies, institutions, teams
- **LOCATION** - Places, cities, countries
- **EVENT** - Meetings, launches, milestones
- **CONCEPT** - Abstract ideas, theories
- **TECHNOLOGY** - Technical tools, frameworks, protocols
- **PRODUCT** - Products, services, commercial offerings

### View Extracted Entities

```bash
curl "http://localhost:8080/api/v1/documents/doc_xyz789/entities"
```

**Response:**

```json
{
  "entities": [
    {
      "name": "SARAH_CHEN",
      "type": "PERSON",
      "description": "CEO of TechCorp Innovation Labs",
      "mentions": [
        { "chunk_id": "chunk_001", "context": "...founded by Sarah Chen..." },
        { "chunk_id": "chunk_002", "context": "...Sarah Chen serves as CEO..." }
      ]
    }
  ],
  "relationships": [
    {
      "source": "SARAH_CHEN",
      "target": "TECHCORP_INNOVATION_LABS",
      "type": "FOUNDED",
      "description": "Co-founded the company in 2020",
      "source_chunk": "chunk_001"
    }
  ]
}
```

### Custom Entity Types

Configure workspace-specific entity types:

```bash
curl -X PATCH "http://localhost:8080/api/v1/workspaces/$WORKSPACE_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "entity_types": [
      "PERSON",
      "COMPANY",
      "DRUG",
      "DISEASE",
      "GENE",
      "PROTEIN"
    ]
  }'
```

This is useful for domain-specific applications (medical, legal, financial).

---

## Step 4: Entity Normalization

EdgeQuake automatically normalizes entity names to prevent duplicates.

### Normalization Rules

```
Input                    → Normalized
─────────────────────────────────────
"Sarah Chen"             → SARAH_CHEN
"Dr. Sarah Chen"         → SARAH_CHEN
"Chen, Sarah"            → SARAH_CHEN
"Ms. Sarah Chen, PhD"    → SARAH_CHEN
"Sarah Chen's work"      → SARAH_CHEN
```

### Merge Detection

When the same entity appears with different descriptions, EdgeQuake merges them:

```
Chunk 1: "Sarah Chen is the CEO of TechCorp"
Chunk 2: "Dr. Chen previously worked at Google DeepMind"

Result:
{
  "name": "SARAH_CHEN",
  "description": "CEO of TechCorp Innovation Labs. Previously led the language model team at Google DeepMind."
}
```

---

## Step 5: Gleaning (Multi-Pass Extraction)

For complex documents, single-pass extraction may miss entities. Enable gleaning for thorough extraction:

```bash
curl -X POST "http://localhost:8080/api/v1/documents?workspace_id=$WORKSPACE_ID" \
  -F "file=@complex_document.pdf" \
  -F "title=Research Paper" \
  -F "gleaning_iterations=2"
```

### How Gleaning Works

```
┌─────────────────────────────────────────────────────────────────┐
│                   GLEANING PROCESS                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Pass 1: Initial Extraction                                     │
│  ─────────────────────────                                      │
│  LLM extracts: [SARAH_CHEN, TECHCORP, NEURALSEARCH]             │
│                                                                 │
│  Pass 2: Glean (review for missed entities)                     │
│  ───────────────────────────────────────────                    │
│  Prompt: "Review text for entities you may have missed"         │
│  LLM extracts: [GOOGLE_DEEPMIND, VENTURE_PARTNERS_CAPITAL]      │
│                                                                 │
│  Combined: 5 entities (vs 3 from single pass)                   │
│  Improvement: +67% recall                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Cost-Benefit

| Gleaning | LLM Calls   | Entity Recall | Cost |
| -------- | ----------- | ------------- | ---- |
| 0 passes | 1 per chunk | Baseline      | $    |
| 1 pass   | 2 per chunk | +15-25%       | $$   |
| 2 passes | 3 per chunk | +25-35%       | $$$  |

Default: 1 gleaning iteration (good balance).

---

## Step 6: Monitor Processing

### Real-Time Status

```bash
# Get processing status
curl "http://localhost:8080/api/v1/documents/doc_xyz789"
```

**Response:**

```json
{
  "id": "doc_xyz789",
  "title": "Research Paper",
  "status": "processing",
  "progress": {
    "phase": "extracting",
    "chunks_total": 45,
    "chunks_processed": 23,
    "percent": 51
  },
  "metrics": {
    "parse_time_ms": 234,
    "chunk_time_ms": 156,
    "extract_time_ms": 12400,
    "tokens_used": 15600
  }
}
```

### Processing Phases

| Phase         | Description            | Duration         |
| ------------- | ---------------------- | ---------------- |
| `parsing`     | Extract text from file | ~100ms           |
| `chunking`    | Split into chunks      | ~50ms            |
| `extracting`  | LLM entity extraction  | ~2-10s per chunk |
| `normalizing` | Deduplicate entities   | ~100ms           |
| `embedding`   | Generate vectors       | ~500ms           |
| `storing`     | Save to database       | ~100ms           |

---

## Step 7: Batch Upload

For large document sets, use batch upload:

```bash
# Create a batch
curl -X POST "http://localhost:8080/api/v1/batches?workspace_id=$WORKSPACE_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Q1 Reports Batch",
    "documents": [
      {"file": "report_jan.pdf", "title": "January Report"},
      {"file": "report_feb.pdf", "title": "February Report"},
      {"file": "report_mar.pdf", "title": "March Report"}
    ]
  }'
```

Or upload a ZIP file:

```bash
curl -X POST "http://localhost:8080/api/v1/documents/bulk?workspace_id=$WORKSPACE_ID" \
  -F "file=@all_reports.zip"
```

---

## Step 8: Reprocess Documents

If you change settings, reprocess existing documents:

```bash
# Reprocess with new entity types
curl -X POST "http://localhost:8080/api/v1/documents/doc_xyz789/reprocess" \
  -H "Content-Type: application/json" \
  -d '{
    "chunk_size": 1024,
    "gleaning_iterations": 2,
    "entity_types": ["PERSON", "DRUG", "DISEASE"]
  }'
```

### What Gets Reprocessed

| Setting Change | Recalculated                 |
| -------------- | ---------------------------- |
| chunk_size     | Chunks, entities, embeddings |
| entity_types   | Entities, relationships      |
| gleaning       | Entities, relationships      |
| LLM model      | Entities, embeddings         |

---

## Step 9: Pipeline Metrics

Analyze pipeline performance:

```bash
curl "http://localhost:8080/api/v1/workspaces/$WORKSPACE_ID/metrics"
```

**Response:**

```json
{
  "workspace_id": "ws_abc123",
  "documents": {
    "total": 150,
    "completed": 148,
    "processing": 2,
    "failed": 0
  },
  "chunks": {
    "total": 4500,
    "avg_size_tokens": 487
  },
  "entities": {
    "total": 1250,
    "by_type": {
      "PERSON": 320,
      "ORGANIZATION": 180,
      "CONCEPT": 450,
      "LOCATION": 150,
      "EVENT": 100,
      "PRODUCT": 50
    }
  },
  "relationships": {
    "total": 2100
  },
  "costs": {
    "llm_tokens_used": 4500000,
    "embedding_tokens_used": 2250000,
    "estimated_cost_usd": 12.5
  }
}
```

---

## Best Practices

### Document Preparation

1. **Clean text** - Remove headers, footers, page numbers if possible
2. **Consistent format** - Use consistent naming for entities
3. **Quality over quantity** - Better documents = better extraction

### Chunk Size Guidelines

| Document Type    | Recommended Size |
| ---------------- | ---------------- |
| General articles | 1200 tokens (default) |
| Technical docs   | 1200 tokens      |
| Short Q&A        | 512 tokens       |
| Legal contracts  | Paragraph-based  |

### Entity Extraction Tips

1. **Domain-specific types** - Add custom types for your domain
2. **Enable gleaning** - For research papers and complex docs
3. **Review extractions** - Spot-check for quality

---

## Troubleshooting

### Low Entity Count

**Problem**: Few entities extracted from detailed document.

**Solutions**:

1. Enable gleaning: `gleaning_iterations=2`
2. Decrease chunk size for finer extraction
3. Check LLM model supports extraction task

### Duplicate Entities

**Problem**: Same entity appears multiple times.

**Solutions**:

1. Check entity normalization is working
2. Review entity descriptions for merge eligibility
3. Consider manual merge via API

### Slow Processing

**Problem**: Documents taking too long.

**Solutions**:

1. Increase worker threads: `WORKER_THREADS=8`
2. Use faster LLM model (gpt-4o-mini)
3. Reduce gleaning iterations
4. Batch documents instead of sequential

---

## What You Learned

✅ How the 6-stage pipeline works  
✅ Chunking strategies and customization  
✅ Entity extraction and normalization  
✅ Gleaning for thorough extraction  
✅ Monitoring processing status  
✅ Batch and bulk upload  
✅ Reprocessing documents  
✅ Pipeline performance metrics

---

## Next Steps

| Tutorial                                    | Description                     |
| ------------------------------------------- | ------------------------------- |
| [Query Optimization](query-optimization.md) | Choosing and tuning query modes |
| [Multi-Tenant Setup](multi-tenant.md)       | Building a SaaS application     |
| [Custom Entity Types](custom-entities.md)   | Domain-specific extraction      |

---

## See Also

- [LightRAG Algorithm](../deep-dives/lightrag-algorithm.md) - Algorithm deep-dive
- [Entity Normalization](../deep-dives/entity-normalization.md) - Deduplication details
- [REST API](../api-reference/rest-api.md) - API reference
