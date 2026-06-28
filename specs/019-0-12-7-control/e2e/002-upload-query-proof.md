# Option 1 — Upload + Query E2E Proof (v0.12.7)

**Status:** ✅ PASS  
**Date:** 2026-06-07 02:10 UTC  
**Stack:** GHCR `0.12.7` quickstart compose @ API `18080`, UI `13000`  
**LLM:** Ollama (`gemma4:latest` + `embeddinggemma:latest`)

## Runners

```bash
# API proof only (requires running Option 1 stack)
bash specs/019-0-12-7-control/e2e/run_upload_query_proof.sh

# UI query proof (Playwright)
cd edgequake_webui
EQ_BACKEND_URL=http://127.0.0.1:18080 PLAYWRIGHT_BASE_URL=http://127.0.0.1:13000 \
  bunx playwright test e2e/spec019-option1-upload-query.spec.ts --project=chromium
```

Full install + upload + query:

```bash
bash specs/019-0-12-7-control/e2e/run_option1_install_proof.sh
```

## API results

| Step | Artifact | Result |
|------|----------|--------|
| Sync upload | `009-upload-response.json` | `chunk_count: 1`, `entity_count: 12`, `status: processed` |
| Hybrid query | `010-query-response.json` | `sources_retrieved: 27`, answer cites **Sarah Chen — Senior Engineer @ EDGEQUAKE** |
| Run log | `011-upload-query-run.log` | PASS |

### Upload excerpt

```json
{
  "document_id": "a92e8b41-e2f8-4cb3-9fd3-8286591b5c6e",
  "status": "processed",
  "chunk_count": 1,
  "entity_count": 12,
  "relationship_count": 7
}
```

### Query excerpt

```json
{
  "answer": "## Sarah Chen's Role at EDGEQUAKE ... Senior Engineer ... GraphRAG Features",
  "mode": "hybrid",
  "stats": { "sources_retrieved": 27, "llm_provider": "ollama" }
}
```

## UI screenshots

| File | Verdict |
|------|---------|
| `05-documents-after-upload.png` | Documents list shows **Completed** rows with entity counts (8–12) |
| `06-query-page-ready.png` | Query shell healthy pre-interaction |
| `07-query-answer-ui.png` | UI hybrid query returns Sarah Chen profile with citations |

## Prerequisites discovered (Option 1)

1. **Ollama on host** must be running (`host.docker.internal:11434`).
2. **Clear leaked API keys** on install host — empty `OPENAI_API_KEY` when using Ollama default:
   ```bash
   OPENAI_API_KEY= EDGEQUAKE_EMBEDDING_PROVIDER=ollama docker compose ... up -d
   ```
3. **Tenant/workspace headers** for raw API calls:
   - `X-Tenant-ID: 00000000-0000-0000-0000-000000000002`
   - `X-Workspace-ID: 00000000-0000-0000-0000-000000000003`
   (Web UI sets these automatically for Default Workspace.)

## Verdict

**GO** — Option 1 Docker install supports full ingest → graph → hybrid query loop on v0.12.7.
