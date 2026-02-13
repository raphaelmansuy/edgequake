# Metadata Debugging Guide

> How to diagnose and fix lineage issues in EdgeQuake

---

## Overview

This guide helps operators troubleshoot common lineage and metadata issues in the EdgeQuake pipeline. It covers diagnostic commands, common failure modes, and repair strategies.

---

## Diagnostic Checklist

When metadata or lineage appears incorrect, work through these checks in order:

```
1. ✅ Is the document status "Completed"?
2. ✅ Does the metadata KV entry exist?
3. ✅ Are chunks stored with position data?
4. ✅ Is the lineage KV entry populated?
5. ✅ Do entities reference valid chunk IDs?
6. ✅ Are model names recorded?
```

---

## Quick Diagnostics

### Check Document Status

```bash
curl -s http://localhost:8080/api/v1/documents/{document_id} | jq '.status'
```

Expected: `"completed"`. If `"failed"` or `"processing"`, the pipeline didn't finish.

### Check Metadata Exists

```bash
curl -s http://localhost:8080/api/v1/documents/{document_id}/metadata | jq 'keys'
```

Expected: Array of metadata keys including `document_type`, `sha256_checksum`, etc.

### Check Chunk Count

```bash
curl -s http://localhost:8080/api/v1/documents/{document_id}/lineage | jq '.chunks | length'
```

Expected: Number > 0. If 0, chunking failed or chunks weren't stored.

### Check Entity Count

```bash
curl -s http://localhost:8080/api/v1/lineage/documents/{document_id} | jq '.extraction_stats'
```

Expected: `total_entities` > 0. If 0, entity extraction failed (check LLM provider).

---

## Common Issues

### Issue 1: Missing Metadata Fields

**Symptom**: `/documents/{id}/metadata` returns empty or minimal fields.

**Cause**: Document was ingested before lineage enhancement (OODA-01 through OODA-06).

**Diagnosis**:
```bash
# Check what fields are present
curl -s http://localhost:8080/api/v1/documents/{document_id}/metadata | jq 'keys'
```

**Fix**: Re-ingest the document. New ingestion will populate all lineage fields:
- `document_type`, `file_size`, `sha256_checksum`
- `llm_model`, `embedding_model`
- `processed_at`

### Issue 2: Chunks Without Line Numbers

**Symptom**: `start_line` and `end_line` are `null` in chunk lineage response.

**Cause**: Chunk was created before position tracking was added, or chunking strategy doesn't support line tracking.

**Diagnosis**:
```bash
# Check a specific chunk
curl -s http://localhost:8080/api/v1/chunks/{chunk_id}/lineage | jq '{start_line, end_line}'
```

**Fix**: Re-ingest the document. Position metadata is computed during chunking and stored in KV storage.

### Issue 3: Entity Extraction Failed

**Symptom**: Document shows "Completed" but has 0 entities.

**Cause**: LLM provider was unavailable during processing.

**Diagnosis**:
```bash
# Check if Ollama is running
curl -s http://localhost:11434/api/tags | jq '.models[].name'

# Check backend logs for extraction errors
grep -i "entity.*error\|extraction.*fail" /tmp/edgequake-backend.log
```

**Fix**:
1. Ensure LLM provider is running:
   ```bash
   ollama serve &
   ollama pull gemma3:latest
   ```
2. Re-upload or re-process the document

### Issue 4: Missing LLM/Embedding Model Names

**Symptom**: `llm_model` and `embedding_model` are `null` in chunk lineage.

**Cause**: Pipeline didn't propagate model info to chunks (pre-OODA-02/OODA-05).

**Diagnosis**:
```bash
curl -s http://localhost:8080/api/v1/chunks/{chunk_id}/lineage | jq '{llm_model, embedding_model}'
```

**Fix**: Re-ingest. Since OODA-05, the processor stamps model names on each chunk during storage.

### Issue 5: Broken PDF → Document Link

**Symptom**: Document has no `pdf_id` even though it was uploaded as PDF.

**Cause**: Document was processed before bidirectional linking (OODA-04).

**Diagnosis**:
```bash
curl -s http://localhost:8080/api/v1/documents/{document_id}/metadata | jq '.pdf_id'
```

**Fix**: Re-upload the PDF. Since OODA-04, `pdf_id` is set during `process_task()`.

### Issue 6: Lineage Not Persisted

**Symptom**: `/documents/{id}/lineage` returns data but lineage KV key doesn't exist.

**Cause**: `enable_lineage_tracking` was `false` (pre-OODA-06).

**Note**: Since OODA-06, lineage tracking defaults to `true`. The `/documents/{id}/lineage` endpoint constructs lineage from chunk and entity data even without the KV entry.

---

## Backend Logs

### Log Locations

| Service  | Location                       |
| -------- | ------------------------------ |
| Backend  | `/tmp/edgequake-backend.log`   |
| Frontend | `/tmp/edgequake-frontend.log`  |

### Useful Log Searches

```bash
# Find extraction errors
grep -i "extract.*error\|extract.*fail" /tmp/edgequake-backend.log

# Find metadata storage events
grep -i "metadata.*store\|metadata.*set" /tmp/edgequake-backend.log

# Find chunk storage events
grep -i "chunk.*store\|chunk.*upsert" /tmp/edgequake-backend.log

# Find lineage persistence events
grep -i "lineage.*persist\|lineage.*store" /tmp/edgequake-backend.log

# Check processing times
grep -i "processing.*complete\|pipeline.*finish" /tmp/edgequake-backend.log
```

### Enable Debug Logging

```bash
export RUST_LOG=debug
make dev
```

For more granular control:
```bash
export RUST_LOG="edgequake_api=debug,edgequake_pipeline=debug,edgequake_core=info"
```

---

## Verification Commands

### Verify Full Lineage Chain

Use this script to validate a document's complete lineage:

```bash
#!/bin/bash
DOC_ID=$1

echo "=== Document Status ==="
curl -s "http://localhost:8080/api/v1/documents/$DOC_ID" | jq '.status'

echo -e "\n=== Metadata ==="
curl -s "http://localhost:8080/api/v1/documents/$DOC_ID/metadata" | jq '{document_type, sha256_checksum, llm_model, embedding_model}'

echo -e "\n=== Lineage Summary ==="
curl -s "http://localhost:8080/api/v1/documents/$DOC_ID/lineage" | jq '{chunks: (.chunks | length), entities: (.entities | length)}'

echo -e "\n=== Extraction Stats ==="
curl -s "http://localhost:8080/api/v1/lineage/documents/$DOC_ID" | jq '.extraction_stats'

echo -e "\n=== First Chunk Lineage ==="
CHUNK_ID="${DOC_ID}-chunk-0"
curl -s "http://localhost:8080/api/v1/chunks/$CHUNK_ID/lineage" | jq '{start_line, end_line, llm_model, embedding_model, entity_count}'
```

### Validate API Health

```bash
curl -s http://localhost:8080/health | jq
```

Expected:
```json
{
  "status": "healthy",
  "storage_mode": "postgresql",
  "components": {
    "kv_storage": true,
    "vector_storage": true,
    "graph_storage": true,
    "llm_provider": true
  }
}
```

---

## Repair Strategies

### Strategy 1: Re-ingest Document

The simplest fix for missing metadata — delete and re-upload:

```bash
# Delete document
curl -X DELETE http://localhost:8080/api/v1/documents/{document_id}

# Re-upload
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@/path/to/document.pdf"
```

### Strategy 2: Check Provider Configuration

```bash
# Verify LLM provider
curl http://localhost:8080/health | jq '.llm_provider_name'

# Verify Ollama models available
curl http://localhost:11434/api/tags | jq '.models[].name'

# If using OpenAI, verify key
echo $OPENAI_API_KEY | head -c 10
```

### Strategy 3: Database Verification

```bash
# Check PostgreSQL is running
docker ps | grep edgequake-postgres

# Restart database if needed
make postgres-stop && make postgres-start && sleep 5

# Restart backend
make stop && make dev-bg
```

---

## Performance Monitoring

### Query Latency

Track lineage query performance:

```bash
# Time a lineage query
time curl -s http://localhost:8080/api/v1/documents/{document_id}/lineage > /dev/null
```

Target: < 200ms for P95. If slower, check:
- Number of chunks in document
- Number of entities in graph
- PostgreSQL connection pool utilization

### Storage Size

```bash
# Check document count
curl -s http://localhost:8080/api/v1/documents | jq '.total'

# Check entity count
curl -s http://localhost:8080/api/v1/graph/stats | jq '.node_count'
```

---

## Related Documentation

- [Architecture: Lineage Tracking](../architecture/lineage-tracking.md)
- [API Reference: Lineage Endpoints](../api-reference/lineage-endpoints.md)
- [Tutorial: Tracing Entity Sources](../tutorials/tracing-entity-sources.md)
