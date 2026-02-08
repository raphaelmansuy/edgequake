# OODA-14: OpenAI RAG Evaluation - Task Log

**Date**: 2026-02-08T08:58:00Z  
**Iteration**: 14 of 50 (RAG Evaluation Mission)

## Actions

- Discovered PDF upload requires `X-Tenant-ID` and `X-Workspace-ID` headers as UUIDs
- Found workspace was configured with Ollama defaults (768 dims)
- Updated workspace to use OpenAI (gpt-5-nano, text-embedding-3-small, 1536 dims)
- Uploaded French legal PDF "test_garantie.pdf" (Garantie légale de conformité)
- Verified OpenAI entity extraction: 15 entities, 14 relationships
- Ran RAG query: 51 sources returned with detailed French legal answer

## Decisions

- Use workspace API to configure OpenAI providers (not env vars)
- Fix dimension mismatch by updating `embedding_dimension: 1536`
- Use "local" mode for query to avoid vector dimension conflicts with old docs

## Next Steps

- Fix gpt-5-nano JSON truncation issue (reasoning tokens exhaust output budget)
- Re-embed older documents with OpenAI 1536-dim embeddings
- Upload more EMILE_FREY documents for comprehensive RAG evaluation
- Measure OpenAI token costs vs Ollama

## Lessons/Insights

- **Workspace-specific providers** (SPEC-032) override env vars for document processing
- **gpt-5-nano reasoning mode** uses 8192 reasoning tokens internally, leaving minimal output tokens
- **Entity extraction quality** with OpenAI is excellent for French legal documents
- **Dimension mismatch** (768 vs 1536) breaks hybrid queries - must be consistent per workspace

## Key Metrics

| Metric                  | Value                                        |
| ----------------------- | -------------------------------------------- |
| Documents processed     | 2 (IFRS16 with Ollama, Garantie with OpenAI) |
| Entities extracted      | 15 (from test_garantie.pdf)                  |
| Relationships extracted | 14                                           |
| Query sources retrieved | 51                                           |
| Query time              | ~17 seconds                                  |
| OpenAI tokens used      | 2161                                         |
| Tokens/second           | 129.87                                       |

## API Discovery

```bash
# PDF Upload (correct headers)
curl -X POST "http://localhost:8080/api/v1/documents/pdf" \
  -H "X-Tenant-ID: 00000000-0000-0000-0000-000000000002" \
  -H "X-Workspace-ID: 00000000-0000-0000-0000-000000000003" \
  -F "file=@document.pdf"

# Workspace Config (PUT for updates)
curl -X PUT "http://localhost:8080/api/v1/workspaces/{id}" \
  -H "Content-Type: application/json" \
  -d '{"llm_provider": "openai", "llm_model": "gpt-5-nano",
       "embedding_provider": "openai", "embedding_model": "text-embedding-3-small",
       "embedding_dimension": 1536}'

# RAG Query
curl -X POST "http://localhost:8080/api/v1/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "...", "mode": "local"}'
```

## Status: ✅ COMPLETED

OpenAI E2E RAG evaluation verified working with French legal documents.
