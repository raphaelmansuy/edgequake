---
title: 'Troubleshooting Guide'
---

# Troubleshooting Guide

> **Diagnosing and Resolving Common EdgeQuake Issues**

This guide helps you identify and fix common problems when running EdgeQuake.

---

## Quick Diagnostics

### Health Check

```bash
# Check basic health
curl http://localhost:8080/health

# Check readiness with dependencies
curl http://localhost:8080/ready

# Check if backend is responding
curl -I http://localhost:8080/api/v1/workspaces
```

### Service Status

```bash
# Check all services (if using make)
make status

# Check PostgreSQL reachability
pg_isready -h localhost -p 5432

# Check Ollama
curl http://localhost:11434/api/tags
```

---

## Common Issues

### 1. Document Upload Errors

#### Symptom: "Expected request with `Content-Type: application/json`"

**Cause**: Using multipart form data (`-F "file=@..."`) with the wrong endpoint.

**Solution**: EdgeQuake has **two different endpoints** for document upload:

**Option A - Upload Text as JSON** (`/api/v1/documents`):

```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Your text content here...",
    "title": "Document Title"
  }'
```

**Option B - Upload Files (PDF, TXT, MD, etc.)** (`/api/v1/documents/upload`):

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@your-document.pdf" \
  -F "title=My Document"
```

#### Symptom: "Failed to parse the request body as JSON"

**Cause**: Using `-F` flag (multipart) with a JSON endpoint, or mixing content types.

**Solution**: Choose the correct endpoint and format:

| Upload Type | Endpoint                         | Content-Type          | Format         |
| ----------- | -------------------------------- | --------------------- | -------------- |
| Text/JSON   | `/api/v1/documents`              | `application/json`    | `-d '{...}'`   |
| Files       | `/api/v1/documents/upload`       | `multipart/form-data` | `-F "file=@"`  |
| Batch Files | `/api/v1/documents/upload/batch` | `multipart/form-data` | `-F "files=@"` |

**Examples**:

```bash
# ❌ WRONG - multipart to JSON endpoint
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@doc.pdf"

# ✅ CORRECT - multipart to upload endpoint
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@doc.pdf"

# ✅ CORRECT - JSON to documents endpoint
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{"content": "Text here", "title": "My Doc"}'
```

---

### 2. Server Won't Start

#### Symptom: "Address already in use"

**Cause**: Port 8080 is already in use.

**Solution**:

```bash
# Find what's using port 8080
lsof -i :8080

# Kill the process
kill -9 <PID>

# Or use different port
PORT=9090 cargo run
```

#### Symptom: "DATABASE_URL is not valid" or a startup log says PostgreSQL storage failed to initialize

**Cause**: Invalid PostgreSQL connection string, unreachable database, or a database daemon that is not running.

**Solution**:

```bash
# Check format
DATABASE_URL="postgresql://user:password@host:port/database"

# Test connection directly
psql "$DATABASE_URL" -c "SELECT 1"

# Or use a lightweight readiness probe
pg_isready -h localhost -p 5432
```

**Why this matters**: EdgeQuake now fails fast and clearly at startup when PostgreSQL is unavailable. This is intentional and prevents flaky half-started server states.

#### Symptom: OrbStack stops unexpectedly or `make dev` says Docker daemon is unavailable

**Cause**: this is usually a **host network route conflict**, not an EdgeQuake crash. On the affected machine, the route table already claims broad private ranges such as `10/8`, `172.16/12`, or `192.168/16` through another interface (for example a VPN, Homebridge, or router helper). OrbStack/Docker then cannot create its bridge network and reports errors such as "failed to add network" or "conflict with existing route".

**What changed**: `make dev` and `make dev-bg` now use **incremental startup**. They reuse healthy services, avoid blind stop/start cycles, and fail with a clear diagnosis instead of hammering Docker repeatedly.

**Solution**:

```bash
# Show the targeted diagnosis
make db-start

# If OrbStack still reports route conflicts:
# 1. stop the conflicting VPN / bridge helper
# 2. restart OrbStack
# 3. rerun the stack
make dev-bg
```

#### Symptom: "Extension 'vector' not found"

**Cause**: pgvector extension not installed.

**Solution**:

```sql
-- As superuser in PostgreSQL
CREATE EXTENSION IF NOT EXISTS vector;
```

Or rebuild Docker container:

```bash
docker compose down -v
docker compose up -d
```

---

### 3. Document Processing Stuck

#### Symptom: Documents stay in "processing" status

**Diagnosis**:

```bash
# Check pending tasks
curl http://localhost:8080/api/v1/tasks?status=pending

# Check backend logs
docker compose logs -f edgequake

# Or if running locally
tail -f /tmp/edgequake-backend.log
```

**Common Causes**:

| Cause              | Solution                     |
| ------------------ | ---------------------------- |
| LLM rate limit     | Wait and retry               |
| Invalid API key    | Check `OPENAI_API_KEY`       |
| Ollama not running | Start Ollama: `ollama serve` |
| Worker crash       | Restart backend              |

**Solution**:

```bash
# Restart workers cleanly
make stop
make dev

# Or manually retry document
curl -X POST "http://localhost:8080/api/v1/documents/$DOC_ID/reprocess"
```

**Reliability note**: delete, reprocess, and recovery flows are workspace-scoped and restart-safe. A deleted document should not be resurrected by stale task recovery after a backend restart.

---

### 3. CI or local runs feel flaky

#### Symptom: repeated pushes trigger too many redundant CI runs

**Cause**: multiple in-progress jobs for the same branch create stale signal and wasted runner time.

**Solution**:

- use the repository workflow concurrency settings so newer runs cancel older ones
- keep heavy coverage and full-E2E flows out of the fastest blocking loop
- prefer readiness probes over fixed sleeps when starting backend and frontend services
- keep the pinned Rust toolchain active locally so clippy and formatting results match CI

---

### 4. PDF Extraction Issues

PDF extraction can fail or produce poor quality results due to PDF structure, encoding, or layout complexity. This section covers the most common PDF-specific problems.

#### Issue 3.1: No Text Extracted or Low-Content Warning

**Symptom**: After PDF upload, `chunk_count = 0`, chunks are empty, or the UI shows
"Low text content - consider using Vision extraction"

**Diagnosis**:

```bash
# Check document details
curl http://localhost:8080/api/v1/documents/doc-uuid

# Response shows:
{
  "chunk_count": 0,
  "metadata": {"pages": 50, "pdf_extraction_method": "edgeparse"}
}
```

**Cause**:
- PDF is image-based (scanned document, no embedded text layer)
- `edgeparse` was used on a scan or image-only PDF

**Solution 1** - Retry with Vision backend:

```bash
# Re-upload with explicit Vision backend
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@scanned_book.pdf" \
  -F "title=Scanned Book" \
  -F "pdf_parser_backend=vision"
```

**Solution 2** - Set Vision as the workspace default for scan-heavy corpora:

```bash
curl -X PUT http://localhost:8080/api/v1/workspaces/$WORKSPACE_ID \
  -H "Content-Type: application/json" \
  -d '{"pdf_parser_backend":"vision"}'
```

**When to use EdgeParse**:
- Digital-native reports
- Invoices and statements with embedded text
- PDFs where speed and deterministic output matter more than OCR-like robustness

**When to use Vision**:
- Scanned documents
- Image-heavy PDFs
- Layouts with poor embedded text quality

**Cost Warning**: Vision mode consumes LLM calls and is slower than EdgeParse.

**Verification**:

- Check `chunk_count > 0` in response
- Check document lineage shows `pdf_extraction_method = "vision"`
- Download chunks to verify content extracted

**Related**: See [PDF Ingestion Tutorial](/docs/tutorials/pdf-ingestion/) for backend selection guidance.

---

#### Issue 3.2: Tables Not Detected or Malformed

**Symptom**: Tables appear as scrambled text or not detected at all

**Before** (raw extraction):

```
Header1 Header2 Header3 Data1a Data1b
Data1c Data2a Data2b Data2c Data3a
```

**Diagnosis**:

```bash
# Check table detection
curl http://localhost:8080/api/v1/documents/doc-uuid

# Response:
{
  "metadata": {
    "tables_detected": 0  // Should be > 0 if tables exist
  }
}
```

**Cause**: Complex table layout (merged cells, nested structures, no clear borders)

**Solution 1** - Enable Table Enhancement:

```bash
# LLM-based table refinement
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@financial_report.pdf" \
  -F "title=Financial Report" \
  -F 'config={"enhance_tables": true, "ai_temperature": 0.1}'
```

**Solution 2** - Combine with Multi-Column Detection:

```bash
# For academic papers with tables in columns
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@research_paper.pdf" \
  -F 'config={
    "enhance_tables": true,
    "layout": {"detect_columns": true}
  }'
```

**After** (enhanced):

```markdown
| Header 1 | Header 2 | Header 3 |
| -------- | -------- | -------- |
| Data 1a  | Data 1b  | Data 1c  |
| Data 2a  | Data 2b  | Data 2c  |
| Data 3a  | Data 3b  | Data 3c  |
```

**Trade-offs**:

- Processing time: 2x slower
- Cost: ~$0.0001 per table
- Accuracy: Significantly improved for complex tables

**Limitations**:

- Very complex tables (5+ levels of merged cells) may still fail
- Tables without any borders are harder to detect

**Verification**:

- Check `tables_detected > 0` in response
- Inspect chunk content for proper markdown table format
- Verify cell alignments correct

**Related**: See [PDF Processing Deep Dive](/docs/deep-dives/pdf-processing/#table-detection) for algorithm details.

---

#### Issue 3.3: Wrong Text Order (Multi-Column Layout)

**Symptom**: Text from different columns interleaved incorrectly

**Example Problem**:

```
# PDF has 2 columns:
Column 1: "The experiment showed that X leads to Y..."
Column 2: "In conclusion, we recommend Z..."

# Extracted (wrong order):
"The experiment In showed conclusion, that we X recommend leads Z... to Y..."
```

**Diagnosis**:

```bash
# Download chunks and check text order
curl http://localhost:8080/api/v1/documents/doc-uuid/chunks

# Look for interleaved text from different sections
```

**Cause**: PDF has multi-column layout (academic papers, newspapers, magazines)

**Solution** - Enable Column Detection:

```bash
# Detect and respect column boundaries
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@research_paper.pdf" \
  -F "title=Research Paper" \
  -F 'config={
    "layout": {
      "detect_columns": true,
      "column_gap_threshold": 20.0
    }
  }'
```

**Configuration Options**:

- `detect_columns`: Enable multi-column detection (default: `true`)
- `column_gap_threshold`: Minimum gap in points between columns (default: 20.0)
  - Increase for wider column gaps
  - Decrease for narrower column gaps

**Verification**:

- Read first few chunks - text should flow naturally
- Check column boundaries respected
- Verify reading order left-to-right within each column

**Tip**: Academic papers almost always need column detection enabled.

**Related**: See [PDF Processing Deep Dive](/docs/deep-dives/pdf-processing/#layout-analysis) for XY-Cut algorithm.

---

#### Issue 3.4: Encoding Errors (Special Characters)

**Symptom**: `�` or `?` characters appear instead of actual text

**Examples**:

- `"Caf�"` instead of `"Café"`
- `"Na�ve"` instead of `"Naïve"`
- `"????"` instead of Chinese/Arabic text

**Diagnosis**:

```bash
# Check extracted content for garbled characters
curl http://localhost:8080/api/v1/documents/doc-uuid/chunks | jq -r '.chunks[0].content'

# Look for � or ? characters
```

**Cause**: PDF uses custom fonts or non-standard encoding not supported by text extraction

**Solution 1** - Retry with the Vision backend:

```bash
# LLM vision reads the actual glyphs
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@custom_fonts.pdf" \
  -F "pdf_parser_backend=vision"
```

**Solution 2** - Check PDF Font Embedding:

```bash
# Use pdffonts to check font embedding (requires poppler-utils)
pdffonts document.pdf

# Look for fonts marked "no" in "emb" column
# These fonts may cause encoding issues
```

**Workaround**: If Vision is too expensive, consider:

1. Re-generate PDF with embedded fonts
2. Convert PDF to another format (DOCX) then back to PDF
3. Use OCR tool (Tesseract) to create new PDF with text layer

**Verification**:

- Check special characters render correctly
- Verify non-English text (if applicable)
- Compare extracted text with PDF visual

**Related**: Vision mode uses LLM to read actual rendered text, avoiding encoding issues entirely.

---

#### Issue 3.5: Low Chunk Quality or Empty Chunks

**Symptom**: Some chunks are very short, empty, or contain garbage

**Example**:

```json
{
  "chunks": [
    { "content": "Page 1", "token_count": 2 }, // Too short
    { "content": "", "token_count": 0 }, // Empty
    { "content": "||||||||", "token_count": 8 } // Garbage
  ]
}
```

**Diagnosis**:

```bash
# Count empty or short chunks
curl http://localhost:8080/api/v1/documents/doc-uuid/chunks | \
  jq '[.chunks[] | select(.token_count < 10)] | length'

# If many short chunks → extraction quality issue
```

**Causes**:

1. PDF has headers/footers (page numbers, logos)
2. Complex layout confuses chunking
3. Embedded images without captions
4. Poor quality scan

**Solution 1** - Enable Readability Enhancement:

```bash
# LLM cleans up extracted text
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@complex_layout.pdf" \
  -F 'config={"enhance_readability": true}'
```

**Solution 2** - Normalize Spacing:

```bash
# Fix concatenated words and spacing issues
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@poor_spacing.pdf" \
  -F 'config={"normalize_spacing": true, "consolidate_headers": true}'
```

**Solution 3** - Adjust Chunking:

```bash
# Increase chunk size to merge small fragments
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@fragmented.pdf" \
  -F "chunk_size=1024" \
  -F "chunk_overlap=100"
```

**Verification**:

- Check average chunk token count (should be 100-500 tokens)
- Verify no empty chunks
- Inspect chunks for coherent content

---

#### Issue 3.6: Upload Fails or Times Out

**Symptom**: PDF upload returns 500 error or times out

**Common Errors**:

```json
{
  "error": "Request timeout",
  "status": 408
}
```

```json
{
  "error": "File too large",
  "status": 413
}
```

**Diagnosis**:

```bash
# Check file size
ls -lh document.pdf

# Check backend logs
docker compose logs -f edgequake

# Or local logs
tail -f /tmp/edgequake-backend.log
```

**Common Causes**:

| Error                 | Cause              | Solution                            |
| --------------------- | ------------------ | ----------------------------------- |
| 413 Payload Too Large | File > 50MB        | Split PDF or increase limit         |
| 408 Timeout           | Processing > 60s   | Use vision mode or increase timeout |
| 500 Internal Error    | Corrupted PDF      | Repair with pdftk or ghostscript    |
| 500 Memory Error      | Large PDF + Vision | Process in batches with `max_pages` |

**Solution 1** - Test with Page Limit:

```bash
# Process first 10 pages to verify config
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@large_report.pdf" \
  -F 'config={"max_pages": 10}'

# If successful, process full document
```

**Solution 2** - Increase Server Timeouts:

```bash
# Increase timeout in config (if you control server)
REQUEST_TIMEOUT=300  # 5 minutes

# Or split PDF into smaller files
pdftk large.pdf cat 1-50 output part1.pdf
pdftk large.pdf cat 51-100 output part2.pdf
```

**Solution 3** - Repair Corrupted PDF:

```bash
# Using ghostscript to repair
gs -o repaired.pdf -sDEVICE=pdfwrite -dPDFSETTINGS=/prepress original.pdf

# Using pdftk
pdftk original.pdf output repaired.pdf
```

**Verification**:

- Check upload returns 200 OK
- Verify `status: "completed"` in response
- Check no error logs in backend

---

#### Issue 3.7: Vision Extraction Timeout — Provider/Model Mismatch

**Symptom**: PDF processing fails with:
- "Processing failed permanently after 3 attempts"
- "Circuit breaker tripped after 3 consecutive timeouts"
- "Vision extraction timed out after 480s"
- "Provider 'ollama' may be unresponsive"

**Root Cause**: The vision **model** does not belong to the vision **provider**.
For example, `EDGEQUAKE_VISION_MODEL=gpt-4.1-nano` paired with provider `ollama`
will always time out because Ollama cannot serve OpenAI models.

This commonly happens when:
- A Makefile or Docker Compose file sets `EDGEQUAKE_VISION_MODEL` to an OpenAI model
  but `.env` overrides the provider to `ollama`
- Env vars are inherited from a parent process (e.g. Makefile `export`)

**Diagnosis**:

```bash
# Check the effective configuration and mismatch status
curl -s http://localhost:8080/api/v1/config/effective | jq '.areas[] | select(.name == "Vision")'

# Expected response when mismatched:
# {
#   "name": "Vision",
#   "has_mismatch": true,
#   "mismatch_description": "Model 'gpt-4.1-nano' is incompatible with provider 'ollama'.\nHow to fix:\n..."
# }
```

You can also check in the **Settings UI** → **Configuration Explainability** panel.
Sections with mismatches are auto-expanded and show remediation options.

**Fix** — choose one:

| Option | Command                                                   | When to use                                         |
| ------ | --------------------------------------------------------- | --------------------------------------------------- |
| A      | `unset EDGEQUAKE_VISION_MODEL`                            | You want to use the default model for your provider |
| B      | `EDGEQUAKE_VISION_PROVIDER=openai` + set `OPENAI_API_KEY` | You want to use the OpenAI model                    |
| C      | `EDGEQUAKE_VISION_MODEL=gemma4:latest`                    | You want to stay on Ollama                          |

Then restart the backend.

**Verification**:

```bash
# Confirm no mismatch
curl -s http://localhost:8080/api/v1/config/effective | jq '.areas[] | select(.name == "Vision") | .has_mismatch'
# Should return: false

# Re-upload the failed PDF
curl -X POST http://localhost:8080/api/v1/documents/upload -F "file=@document.pdf"
```

**Prevention**: EdgeQuake now auto-corrects mismatches at runtime — if a model
is incompatible with the resolved provider, it is skipped and the next candidate
in the resolution chain is used (with a WARN log). The explainability endpoint
helps you fix the env vars permanently.

---

### PDF Troubleshooting Decision Tree

Use this flowchart to quickly diagnose PDF issues:

```
PDF Upload Issue?
  │
  ├─ chunk_count = 0
  │   ├─ Retry with Vision → {"pdf_parser_backend":"vision"}
  │   ├─ Still 0? → Check if PDF encrypted/protected
  │   └─ Still 0? → File GitHub issue with sample
  │
  ├─ Tables not detected / malformed
  │   ├─ Enable table enhancement → {"enhance_tables": true}
  │   ├─ Still bad? → Try Vision + enhance → {"pdf_parser_backend":"vision", "enhance_tables": true}
  │   └─ Complex table? → Known limitation (file issue)
  │
  ├─ Text order wrong
  │   ├─ Enable column detection → {"layout": {"detect_columns": true}}
  │   └─ Still wrong? → Adjust column_gap_threshold
  │
  ├─ Encoding errors (�, ?)
  │   ├─ Retry with Vision → {"pdf_parser_backend":"vision"}
  │   └─ Still bad? → Check PDF font embedding (pdffonts)
  │
  ├─ Upload fails / timeout
  │   ├─ File > 50MB? → Split PDF or increase limit
  │   ├─ Timeout? → Test with max_pages: 10
  │   ├─ Vision timeout + circuit breaker? → Check provider/model mismatch (Issue 3.7)
  │   └─ Error 500? → Repair PDF (ghostscript, pdftk)
  │
  └─ Poor quality chunks
      ├─ Enable readability → {"enhance_readability": true}
      ├─ Normalize spacing → {"normalize_spacing": true}
      └─ Adjust chunk_size → chunk_size=1024
```

---

### PDF Configuration Quick Reference

Common configurations for different PDF types:

**Digital PDF (good quality)**:

```bash
# Default settings - no config needed
curl -X POST http://localhost:8080/api/v1/documents/upload -F "file=@digital.pdf" http://localhost:8080/api/v1/documents/upload
```

**Scanned Document**:

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload -F "file=@scanned.pdf" \
     -F "pdf_parser_backend=vision" \
     http://localhost:8080/api/v1/documents/upload
```

**Academic Paper (multi-column)**:

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload -F "file=@paper.pdf" \
     -F 'config={"layout": {"detect_columns": true}}' \
     http://localhost:8080/api/v1/documents/upload
```

**Financial Report (complex tables)**:

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload -F "file=@financials.pdf" \
     -F 'config={"enhance_tables": true}' \
     http://localhost:8080/api/v1/documents/upload
```

**Unknown Quality**:

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload -F "file=@unknown.pdf" \
     -F "pdf_parser_backend=edgeparse" \
     http://localhost:8080/api/v1/documents/upload
```

**Critical Document (maximum accuracy)**:

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload -F "file=@critical.pdf" \
     -F 'config={
       "pdf_parser_backend": "vision",
       "enhance_tables": true,
       "enhance_readability": true,
       "vision_dpi": 200
     }' \
     http://localhost:8080/api/v1/documents/upload
```

---

### When to Seek Further Help

If PDF extraction still fails after trying these solutions:

1. **Read Full Documentation**:
   - [PDF Ingestion Tutorial](/docs/tutorials/pdf-ingestion/) - Configuration details
   - [PDF Processing Deep Dive](/docs/deep-dives/pdf-processing/) - Algorithm internals

2. **Check GitHub Issues**:
   - Search existing issues: `https://github.com/org/edgequake/issues`
   - Look for similar PDF problems
   - Check if limitation is already known

3. **File New Issue**:
   - Include PDF metadata (pages, file size, type)
   - Include error messages / logs
   - Attach problematic PDF (if not confidential)
   - Describe expected vs actual behavior

4. **Community Support**:
   - Discord: `#pdf-extraction` channel
   - Stack Overflow: Tag `edgequake pdf`

---

### 4. Empty Query Results

#### Symptom: Query returns empty answer

**Diagnosis**:

```bash
# Check document count
curl "http://localhost:8080/api/v1/documents?workspace_id=$WORKSPACE_ID"

# Check entity count
curl "http://localhost:8080/api/v1/graph/entities?workspace_id=$WORKSPACE_ID"

# Check chunk count
curl "http://localhost:8080/api/v1/workspaces/$WORKSPACE_ID/stats"
```

**Common Causes**:

| Symptom                  | Cause              | Solution            |
| ------------------------ | ------------------ | ------------------- |
| 0 documents              | No uploads         | Upload documents    |
| Documents but 0 entities | Processing failed  | Reprocess documents |
| Entities but no results  | Query not matching | Try different mode  |

**Debug Query**:

```bash
# Try naive mode (vector only) to verify basic retrieval
curl -X POST "http://localhost:8080/api/v1/query?workspace_id=$WORKSPACE_ID" \
  -H "Content-Type: application/json" \
  -d '{"query": "test", "mode": "naive", "max_chunks": 20}'
```

---

### 5. LLM Errors

#### Symptom: "OpenAI API error: Rate limit exceeded"

**Solution**:

```bash
# Wait and retry
sleep 60

# Or switch to different provider
export EDGEQUAKE_LLM_PROVIDER=ollama
```

#### Symptom: "OpenAI API error: Invalid API key"

**Solution**:

```bash
# Check key is set
echo $OPENAI_API_KEY

# Check key starts with sk-
# Keys should look like: sk-proj-abc123...

# Test key directly
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"
```

#### Symptom: "Connection refused to Ollama"

**Solution**:

```bash
# Start Ollama
ollama serve

# Check it's running
curl http://localhost:11434/api/tags

# Pull required model
ollama pull gemma3:12b
ollama pull nomic-embed-text
```

---

### 6. Slow Performance

#### Symptom: Queries taking > 5 seconds

**Diagnosis**:

```bash
# Enable debug logging
RUST_LOG="edgequake=debug" cargo run

# Check query timing in logs
# Look for: "query completed in Xms"
```

**Common Causes**:

| Cause          | Diagnosis        | Solution                |
| -------------- | ---------------- | ----------------------- |
| Cold LLM       | First query slow | Warm up with test query |
| Large context  | Too many chunks  | Reduce `max_chunks`     |
| Slow embedding | Ollama on CPU    | Use GPU or OpenAI       |
| DB connection  | Pool exhausted   | Check connections       |

**Quick Fixes**:

```bash
# Use faster model
curl -X POST "http://localhost:8080/api/v1/query" \
  -d '{"query": "test", "llm_model": "gpt-5-nano"}'

# Reduce context size
curl -X POST "http://localhost:8080/api/v1/query" \
  -d '{"query": "test", "max_chunks": 5, "max_entities": 5}'
```

---

### 7. Database Issues

#### Symptom: "Connection pool exhausted"

**Solution**:

```bash
# Check active connections
psql $DATABASE_URL -c "SELECT count(*) FROM pg_stat_activity WHERE datname='edgequake'"

# Increase pool size in DATABASE_URL
DATABASE_URL="postgresql://user:pass@host:5432/db?max_connections=50"

# Or use PgBouncer for pooling
```

#### Symptom: "relation 'documents' does not exist"

**Cause**: Migrations haven't run.

**Solution**:

```bash
# Migrations run automatically on startup
# Restart the backend
cargo run

# Or run manually if needed
psql $DATABASE_URL -f migrations/001_initial.sql
```

#### Symptom: "disk full" or slow inserts

**Solution**:

```bash
# Check disk usage
df -h

# Check table sizes
psql $DATABASE_URL -c "
SELECT schemaname, relname,
       pg_size_pretty(pg_total_relation_size(relid))
FROM pg_catalog.pg_statio_user_tables
ORDER BY pg_total_relation_size(relid) DESC LIMIT 10;
"

# Vacuum database
psql $DATABASE_URL -c "VACUUM ANALYZE"
```

---

### 8. Graph Issues

#### Symptom: "AGE extension not loaded"

**Solution**:

```sql
-- Load AGE extension
LOAD 'age';
SET search_path = ag_catalog, "$user", public;

-- Create graph if not exists
SELECT create_graph('edgequake_graph');
```

#### Symptom: Entities not connected

**Diagnosis**:

```bash
# Check relationship count
curl "http://localhost:8080/api/v1/graph/relationships?workspace_id=$WORKSPACE_ID"

# If 0 relationships, check extraction logs
```

**Solution**:

```bash
# Reprocess with verbose logging
RUST_LOG="edgequake_pipeline=debug" cargo run

# Then reprocess document
curl -X POST "http://localhost:8080/api/v1/documents/$DOC_ID/reprocess"
```

---

### 9. Frontend Issues

#### Symptom: Frontend can't connect to backend

**Check**:

```bash
# Is backend running?
curl http://localhost:8080/health

# CORS enabled?
# Backend should return Access-Control-Allow-Origin header
curl -I http://localhost:8080/health
```

**Solution**:

```bash
# Start backend first, then frontend
make backend-dev &
sleep 5
make frontend-dev
```

#### Symptom: Graph visualization empty

**Causes**:

1. No entities extracted
2. WebSocket connection failed
3. Sigma.js not loading

**Solution**:

```bash
# Check entities exist via API
curl "http://localhost:8080/api/v1/graph/entities?workspace_id=$WORKSPACE_ID"

# Check browser console for errors
# Open DevTools → Console
```

---

## Diagnostic Commands

### Logs

```bash
# Backend logs
tail -f /tmp/edgequake-backend.log

# Frontend logs
tail -f /tmp/edgequake-frontend.log

# Docker logs
docker compose logs -f

# Specific component
docker compose logs -f edgequake
docker compose logs -f postgres
```

### Database Queries

```sql
-- Check document status
SELECT status, count(*) FROM documents GROUP BY status;

-- Find failed documents
SELECT id, title, error_message FROM documents WHERE status = 'failed';

-- Check entity counts by workspace
SELECT workspace_id, count(*) FROM entities GROUP BY workspace_id;

-- Check embedding dimensions
SELECT embedding_dimension, count(*) FROM embeddings GROUP BY embedding_dimension;
```

### API Debugging

```bash
# Verbose curl output
curl -v http://localhost:8080/api/v1/workspaces

# Pretty print JSON
curl http://localhost:8080/api/v1/workspaces | jq

# Check response headers
curl -I http://localhost:8080/health
```

---

## Error Reference

| Error Code | Meaning             | Solution                  |
| ---------- | ------------------- | ------------------------- |
| 400        | Bad request         | Check request format      |
| 401        | Unauthorized        | Add API key               |
| 404        | Not found           | Check workspace_id exists |
| 422        | Validation error    | Check required fields     |
| 429        | Rate limited        | Wait and retry            |
| 500        | Server error        | Check logs                |
| 503        | Service unavailable | Check DB/LLM connection   |

---

## Getting Help

### Before Asking for Help

1. Check this troubleshooting guide
2. Check logs for specific error messages
3. Verify environment variables are set
4. Try with minimal configuration (PostgreSQL + mock LLM)

### Debug Mode

Start with maximum logging:

```bash
RUST_LOG="edgequake=trace,sqlx=debug,tower_http=debug" cargo run
```

### Report an Issue

Include in your report:

1. EdgeQuake version (`cargo run --version`)
2. Storage mode (PostgreSQL or Memory)
3. LLM provider and model
4. Steps to reproduce
5. Relevant log output
6. Expected vs actual behavior

---

## See Also

- [Configuration Reference](/docs/operations/configuration/) - All settings
- [Monitoring Guide](/docs/operations/monitoring/) - Observability setup
- [Deployment Guide](/docs/operations/deployment/) - Production setup
