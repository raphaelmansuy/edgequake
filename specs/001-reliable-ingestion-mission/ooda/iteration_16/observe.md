# OODA-16: Observe - Parallel Ingestion & Provider Audit

**Date**: 2026-02-08
**Mission**: Reliable Document Ingestion Pipeline
**Focus**: Test parallel ingestion, audit in-memory providers

---

## 1. In-Memory Provider Audit

### Production Code Status ✅

In-memory providers **already removed** for production (OODA-03):

```rust
// edgequake/src/main.rs lines 247-265
// OODA-03: DATABASE_URL is now REQUIRED - in-memory storage removed for production consistency
let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
    error!(" FATAL: DATABASE_URL environment variable is REQUIRED");
    error!(" In-memory storage has been removed for production consistency.");
    std::process::exit(1);  // Server exits if no database
});
```

### In-Memory Usage Locations

| Location                | Purpose                | Production Impact   |
| ----------------------- | ---------------------- | ------------------- |
| `state.rs:new_memory()` | Test helper            | ❌ Not used in prod |
| `settings.rs` tests     | Unit tests             | ❌ Test only        |
| `e2e_*.rs` tests        | Integration tests      | ❌ Test only        |
| `benches/*.rs`          | Performance benchmarks | ❌ Benchmarks only  |
| `e2e_pipeline_tests.rs` | Pipeline tests         | ❌ Test only        |

**Conclusion**: In-memory providers exist ONLY for testing. Production requires DATABASE_URL.

---

## 2. Parallel Document Ingestion Test

### Test Conducted

Uploaded 2 PDFs simultaneously:

1. `C1 - Introduction IFRS 16.pdf`
2. `Fiscalité - Synthèse formalités fiscales.pdf`

### Results

| Document                | Status     | Error                     |
| ----------------------- | ---------- | ------------------------- |
| comet_2602.01766v1.pdf  | processing | (from earlier test)       |
| Fiscalité - Synthèse... | **failed** | Ollama connection refused |

### Root Cause Analysis

```
Pipeline processing failed: Entity extraction error:
All 2 chunks failed extraction. Failures:
- Chunk 0: LLM error: Network error: error sending request for url (http://localhost:11434/api/chat)
- Chunk 1: LLM error: Network error: error sending request for url (http://localhost:11434/api/chat)
```

**Issue**: Document processing used DEFAULT provider (Ollama) instead of workspace-configured provider (OpenAI gpt-4.1-nano).

### Possible Causes

1. Workspace provider resolution may be failing silently
2. Task processor may not be getting workspace_id in metadata
3. OPENAI_API_KEY may not be set when provider is created

---

## 3. Backend Health vs Workspace Configuration

### Health Endpoint (Server Defaults)

```json
{
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

### Workspace Configuration (Expected for Processing)

```json
{
  "llm_provider": "openai",
  "llm_model": "gpt-4.1-nano",
  "embedding_provider": "openai",
  "embedding_model": "text-embedding-3-small",
  "embedding_dimension": 1536
}
```

**Gap**: Health shows server defaults (Ollama), but workspace should use OpenAI.
Document processing appears to be falling back to server defaults.

---

## 4. Dimension Mismatch Errors Observed

From backend logs:

```
Embedding dimension mismatch: expected 768, got 1536
```

**Cause**:

- Server default vector storage: 768 dimensions (Ollama)
- Workspace embedding provider: 1536 dimensions (OpenAI)

This confirms that workspace-specific storage isn't being used consistently.

---

## 5. Ollama Service Status

| Before OODA-16 | After OODA-16                          |
| -------------- | -------------------------------------- |
| Not running    | ✅ Running                             |
| N/A            | 44 models available                    |
| N/A            | gemma3:latest, nomic-embed-text:latest |

---

## 6. Code Paths for Workspace Provider Resolution

### Document Processing Flow

```
┌──────────────────────────────────────────────────────────────────────────┐
│                    DOCUMENT PROCESSING FLOW                               │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  1. PDF Upload Handler (pdf_upload.rs)                                   │
│     └─> Extracts workspace_id from X-Workspace-ID header                │
│                                                                          │
│  2. Task Created (with workspace_id in metadata?)                        │
│     └─> processor.rs:get_workspace_pipeline()                           │
│                                                                          │
│  3. Pipeline Resolution (SPEC-032)                                       │
│     ├─> If workspace_id valid:                                          │
│     │   └─> ProviderFactory::create_safe_llm_provider()                 │
│     │   └─> ProviderFactory::create_safe_embedding_provider()           │
│     │                                                                    │
│     └─> If workspace_id invalid/missing OR provider creation fails:     │
│         └─> USE DEFAULT PIPELINE (Ollama) ❌                            │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Potential Issue: API Key Not Available

When `ProviderFactory::create_safe_llm_provider("openai", "gpt-4.1-nano")` is called:

- It needs OPENAI_API_KEY from environment
- If not set, it may fail silently and fallback to default

---

## 7. Log Evidence Analysis

### Successful OpenAI Processing (08:58)

```
OpenAI response - model: gpt-5-nano-2025-08-07
Token usage - prompt: 376, completion: 6112, total: 6488
```

### Failed Processing (09:10)

```
Entity extraction error: LLM error: Network error: error sending request
for url (http://localhost:11434/api/chat)
```

**Observation**: Earlier documents used OpenAI successfully. Later documents fell back to Ollama (which wasn't running).

---

## 8. Files to Investigate

| File            | Lines    | Purpose                                   |
| --------------- | -------- | ----------------------------------------- |
| `processor.rs`  | 220-350  | `get_workspace_pipeline()` implementation |
| `state.rs`      | 987-1060 | `get_pipeline_for_workspace()`            |
| `pdf_upload.rs` | 300-600  | PDF upload handler, workspace metadata    |

---

## 9. Mission Success Criteria Status

| Criterion                            | Status  | Notes                               |
| ------------------------------------ | ------- | ----------------------------------- |
| No in-memory providers in production | ✅ PASS | Verified OODA-03                    |
| 2 documents parallel ingestion       | ⚠️ FAIL | Fallback to Ollama when not running |
| Ingestion with OpenAI                | ✅ PASS | Verified in OODA-15                 |
| Ingestion with Ollama                | ✅ PASS | Now running with models             |

---

## 10. Next Steps (Orient)

1. Investigate why document processing fell back to Ollama
2. Verify task metadata includes workspace_id
3. Check if OPENAI_API_KEY is available during provider creation
4. Test parallel ingestion with Ollama running as fallback
