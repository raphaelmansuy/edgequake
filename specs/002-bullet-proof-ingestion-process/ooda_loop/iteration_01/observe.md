# Iteration 01 - Observe

**Date**: 2026-01-28 14:55:00

**Mission Status**: ✅ Re-read mission file

## Current System State

### Backend Health Check

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "storage_mode": "postgresql",
  "workspace_id": "default",
  "components": {
    "kv_storage": true,
    "vector_storage": true,
    "graph_storage": true,
    "llm_provider": true
  },
  "llm_provider_name": "ollama",
  "schema": {
    "latest_version": 20,
    "migrations_applied": 19,
    "last_applied_at": "2026-01-28T05:42:25.323903+00:00"
  }
}
```

**Status**: ✅ Backend is healthy, using Ollama provider, PostgreSQL storage

### Ollama Service Status

- **Status**: ✅ Running on localhost:11434
- **Models Available**: 42 models including gemma3:latest, embeddinggemma:latest
- **Runners Active**: 2 model instances loaded (visible in process list)

### Active Processes

- **Backend PID**: 36558 (`target/debug/edgequake`)
- **Frontend PID**: 36638 (Next.js dev server)
- **Ollama Server PID**: 31050 (`ollama serve`)

### Environment Variables (from process)

```bash
EDGEQUAKE_LLM_PROVIDER="ollama"
OLLAMA_HOST="http://localhost:11434"
OLLAMA_MODEL="gemma3:latest"
OLLAMA_EMBEDDING_MODEL="nomic-embed-text"
DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake"
```

## Code Investigation

### Document Upload Handler

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs`
**Function**: `upload_document` (line 497)

#### Processing Flow (Synchronous Mode)

```
User → upload_document() (L497)
        ↓
     Validate content (L512)
        ↓
     Store metadata in KV (L563)
        ↓
     Create workspace pipeline (L652-655)
        ↓
     workspace_pipeline.process() (L657) ← BLOCKING CALL - NO TIMEOUT
        ↓
     Store chunks (L660-673)
        ↓
     Store vectors (L679-721)
        ↓
     Store graph entities (L729+)
        ↓
     Return response
```

#### Key Findings

1. **NO HTTP-LEVEL TIMEOUT**: The `upload_document` handler has NO timeout wrapper
   - Line 657: `workspace_pipeline.process(&document_id, &request.content).await?`
   - This is a direct `.await` with no `tokio::time::timeout` wrapper
   - If pipeline hangs, the HTTP request will hang indefinitely

2. **Pipeline Layer Timeout**: 600s timeout exists in LLM layer only
   - Location: `edgequake-llm/src/safety_limits.rs` (DEFAULT_TIMEOUT_SECS = 600)
   - This timeout applies ONLY to individual LLM API calls
   - Does NOT apply to the entire pipeline execution

3. **Async Mode Available**: Line 583-632 implements async processing via task queue
   - Documents with `async_processing: true` return immediately with task_id
   - Task is queued for background processing
   - BUT: Default mode is synchronous (async flag not set by frontend)

### Test Documents

```bash
aws_2601.08734v1.extracted.md:     86,408 bytes (84KB)
scienti_2601.16282v1.extracted.md: 123,909 bytes (121KB)
```

## Test Results

### Attempt 1: Upload 121KB Document (Synchronous)

```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d @/tmp/test_large.json
```

**Result**: ❌ Request **HANGS INDEFINITELY**

- No response after 3+ minutes
- Ctrl+C required to terminate
- No timeout error in backend logs
- No entity extraction progress logs

### Backend Logs During Hang

```
2026-01-28T06:14:56.518499Z DEBUG edgequake_llm::providers::ollama: Ollama chat request: 1 messages to model gemma3:12b
[... NO FURTHER LOGS ...]
```

**Analysis**: The Ollama chat request is sent but never completes. No timeout is triggered.

## Measurements

### Document Sizes

- Small (aws): 86,408 bytes → Expected processing time: ~2 minutes
- Large (scienti): 123,909 bytes → Expected processing time: ~3-5 minutes

### Network Latency

- Ollama API: localhost (< 1ms)
- Backend API: localhost (< 1ms)

### Processing Time Observations

- Previous successful test (small 875-byte doc): 25.1 seconds with Ollama
- Current 121KB doc: > 180 seconds (no completion, terminated manually)

## Error Patterns

### Pattern 1: No Timeout at HTTP Layer

- **Evidence**: Request hangs indefinitely without timeout error
- **Location**: `handlers/documents.rs:657` - direct `.await` on pipeline
- **Impact**: Client waits forever, no error message, poor UX

### Pattern 2: LLM Timeout Not Propagating

- **Evidence**: Backend logs show "Ollama chat request" but no timeout error after 600s
- **Hypothesis**: Ollama itself may be hanging, or timeout is not configured correctly
- **Impact**: Circuit breaker may not trip if timeout doesn't return Error

### Pattern 3: No Progress Indicators

- **Evidence**: Only initial "job_started" broadcast (line 651)
- **No logs** for: chunking progress, entity extraction progress, relationship extraction
- **Impact**: Impossible to diagnose where pipeline is stuck

## System Constraints (First Principles)

### Immutable Constraints

1. **LLM API calls are slow**: Entity extraction from 121KB can take 5-10 minutes
2. **HTTP timeouts exist**: Most load balancers timeout after 60-120 seconds
3. **Users expect feedback**: No progress = perceived failure
4. **Memory is finite**: Loading entire 121KB in memory for processing

### Challengeable Assumptions

1. ❌ "Must process entire document synchronously"
   - **Challenge**: Can we chunk and stream results?
2. ❌ "600-second timeout is sufficient"
   - **Challenge**: Is this even being applied correctly?
3. ❌ "One LLM call per document"
   - **Challenge**: Can we parallelize extraction across chunks?

## Hypotheses for Next Iteration

### Hypothesis A: HTTP-Layer Timeout Missing

**Probability**: 90%
**Evidence**: Code shows no `tokio::time::timeout` wrapper at handler level
**Test**: Add timeout wrapper around `workspace_pipeline.process()` call
**Expected Outcome**: Request will fail with timeout error after X seconds

### Hypothesis B: Ollama Slow on Large Context

**Probability**: 70%
**Evidence**: 121KB context may overwhelm gemma3:12b model
**Test**: Check Ollama logs, monitor CPU/memory during processing
**Expected Outcome**: Find Ollama process consuming 100% CPU, very slow generation

### Hypothesis C: Entity Extraction Algorithm Inefficient

**Probability**: 50%
**Evidence**: No logs showing chunking progress, may be processing entire doc at once
**Test**: Read pipeline code to understand chunking strategy
**Expected Outcome**: Find that chunks are created but entities extracted from full doc

## Questions for Next Iteration

1. ❓ What is the actual timeout configured in Axum/Tower middleware?
2. ❓ Does the pipeline chunk the document before entity extraction?
3. ❓ Why is there no "Ollama response received" log after the "chat request" log?
4. ❓ Is Ollama actually processing or is the connection hanging?
5. ❓ What is the maximum context window for gemma3:12b model?

## Files to Investigate Next

1. `edgequake/crates/edgequake-pipeline/src/lib.rs` - Pipeline implementation
2. `edgequake/crates/edgequake-llm/src/providers/ollama.rs` - Ollama client code
3. `edgequake/crates/edgequake-api/src/main.rs` - Axum middleware configuration
4. Ollama server logs: `/Users/raphaelmansuy/.ollama/logs/` (if available)

## Next Steps

1. Read pipeline code to understand document processing flow
2. Add HTTP-level timeout wrapper as immediate fix
3. Instrument pipeline with progress logging
4. Test with smaller document chunks
5. Consider switching to async mode for large documents
