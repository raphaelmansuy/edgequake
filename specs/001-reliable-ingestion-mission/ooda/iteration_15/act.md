# OODA-15: Act - Price/Performance Configuration Testing

**Date**: 2026-02-08
**Mission**: Reliable Document Ingestion Pipeline
**Focus**: Test and document optimal model configuration

---

## Actions Completed

### A1: Updated Workspace Configuration ✅

Changed workspace to use gpt-4.1-nano for reliable JSON extraction:

```bash
curl -X PUT "http://localhost:8080/api/v1/workspaces/00000000-0000-0000-0000-000000000003" \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID: 00000000-0000-0000-0000-000000000002" \
  -H "X-Workspace-ID: 00000000-0000-0000-0000-000000000003" \
  -d '{"llm_provider": "openai", "llm_model": "gpt-4.1-nano", "embedding_provider": "openai", "embedding_model": "text-embedding-3-small", "embedding_dimension": 1536}'
```

**Result**: Workspace updated successfully at 2026-02-08T09:04:56.495041+00:00

### A2: Uploaded Test Document ✅

Uploaded "Projet Loi de Finances 2026.pdf" (470KB) for testing:

```bash
curl -X POST "http://localhost:8080/api/v1/documents/pdf" \
  -H "X-Tenant-ID: 00000000-0000-0000-0000-000000000002" \
  -H "X-Workspace-ID: 00000000-0000-0000-0000-000000000003" \
  -F "file=@.../Projet Loi de Finances 2026.pdf"
```

**Result**:

- PDF ID: 32d2b796-096d-438e-be05-0c7acf4cfe07
- Task ID: pdf-5844abb9-e5b8-41dd-abbf-9e90a27e47dd
- Status: processing → completed
- Estimated time: 31 seconds

### A3: Verified Entity Extraction ✅

| Metric   | Value        |
| -------- | ------------ |
| Status   | completed    |
| Entities | 20           |
| Cost     | $0.00112585  |
| Model    | gpt-4.1-nano |

**JSON extraction was reliable** - no truncation issues observed.

### A4: Tested RAG Query ✅

Query: "Quels sont les changements fiscaux prévus pour 2026?"

| Metric            | Value        |
| ----------------- | ------------ |
| Sources Retrieved | 92           |
| Embedding Time    | 1182ms       |
| Retrieval Time    | 12ms         |
| Generation Time   | 1943ms       |
| Total Time        | 3171ms       |
| Tokens Used       | 35           |
| Tokens/Second     | 18.01        |
| LLM Provider      | openai       |
| LLM Model         | gpt-4.1-nano |

### A5: Playwright UI Verification ✅

Navigated to http://localhost:3000/documents via Playwright:

- Observed 8 documents with "Completed" status
- Pipeline idle
- All documents display entity counts and costs

---

## Model Comparison Results

| Model            | Input/1M | Output/1M | JSON Reliability          | Test Result  |
| ---------------- | -------- | --------- | ------------------------- | ------------ |
| gpt-5-nano       | $0.05    | $0.40     | Medium (reasoning tokens) | May truncate |
| **gpt-4.1-nano** | $0.10    | $0.40     | High (no reasoning)       | ✅ Reliable  |

**Winner for Production**: `gpt-4.1-nano`

- 2x more expensive for input ($0.10 vs $0.05)
- Same output price ($0.40)
- More reliable JSON extraction
- Delta: ~$0.005 per document (worth it for reliability)

---

## Cost Analysis

### Per-Document Cost (100 pages)

| Configuration                | Expected | Observed            |
| ---------------------------- | -------- | ------------------- |
| gpt-4.1-nano + embed-3-small | ~$0.03   | $0.00113 (14 pages) |

Extrapolated to 100 pages: ~$0.008 per 100 pages (very cost-effective)

---

## Files Changed

None (runtime configuration only via API)

---

## Commit Reference

N/A - Configuration changes via REST API, no code changes.

---

## Evidence

### Backend Health Check

```json
{
  "status": "healthy",
  "llm_provider_name": "ollama",
  "providers": {
    "llm": { "name": "ollama", "model": "gemma3:latest" },
    "embedding": {
      "name": "ollama",
      "model": "nomic-embed-text",
      "dimension": 768
    }
  }
}
```

Note: Health shows default providers, workspace-specific settings override.

### Workspace Settings (After Update)

```json
{
  "llm_provider": "openai",
  "llm_model": "gpt-4.1-nano",
  "embedding_provider": "openai",
  "embedding_model": "text-embedding-3-small",
  "embedding_dimension": 1536
}
```

### Document Processing Confirmation

```json
{
  "title": "Projet Loi de Finances 2026.pdf",
  "status": "completed",
  "entity_count": 20,
  "cost_usd": 0.00112585
}
```

---

## Next OODA Iteration

OODA-16 Focus:

1. Test parallel document ingestion (2 documents simultaneously)
2. Verify ingestion works with both Ollama and OpenAI
3. Update documentation with price/performance guide

---

## Summary

| Decision                  | Outcome                |
| ------------------------- | ---------------------- |
| Use gpt-4.1-nano          | ✅ Works reliably      |
| 1536 dimension embeddings | ✅ Consistent          |
| E2E pipeline test         | ✅ Passed              |
| RAG query test            | ✅ 92 sources returned |
