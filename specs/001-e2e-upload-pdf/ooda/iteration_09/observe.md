# OODA Iteration 09 - Observe

## Mission Re-read Confirmation

✅ Mission file read at: 2026-02-07
✅ Critical Safety Mandate: COMPLIED

## Focus: OpenAI Provider Configuration

### Current State

**Backend Health Check**:

```json
{
  "status": "healthy",
  "llm_provider_name": "ollama"
}
```

**Problem**: System is using Ollama, but user requires OpenAI for model and embedding.

### Provider Selection Logic

**Location**: `edgequake/crates/edgequake-llm/src/factory.rs:110-145`

**Priority Order**:

1. `EDGEQUAKE_LLM_PROVIDER` environment variable (explicit selection)
2. Auto-detect: `OLLAMA_HOST` → `LMSTUDIO_HOST` → `OPENAI_API_KEY` → Mock

**Current Detection**:

- `OLLAMA_HOST` or `OLLAMA_MODEL` is set → selects Ollama
- `OPENAI_API_KEY` is NOT set or is "test-key" → skipped

### Environment Variable Analysis

```bash
# Current environment (from Makefile)
OLLAMA_MODEL=gemma3:12b              # ← Forces Ollama selection
OLLAMA_EMBEDDING_MODEL=embeddinggemma:latest
# OPENAI_API_KEY is not set or empty
```

### Required Changes

1. **For OpenAI provider**:
   - Set `OPENAI_API_KEY` with valid key
   - Unset `OLLAMA_HOST` and `OLLAMA_MODEL` (or set `EDGEQUAKE_LLM_PROVIDER=openai`)

2. **OpenAI Model Configuration**:
   - Chat model: `gpt-4o-mini` (default in OpenAI provider)
   - Embedding model: `text-embedding-3-small` (1536 dimensions)

### Makefile Analysis

**Current backend target** (`Makefile`):

```makefile
backend-dev:
    PDFIUM_DYNAMIC_LIB_PATH=... \
    OLLAMA_MODEL=gemma3:12b \
    OLLAMA_EMBEDDING_MODEL=embeddinggemma:latest \
    cargo run
```

This explicitly sets Ollama variables, which triggers Ollama provider selection.

### Solution Options

1. **Option A**: Add `openai-dev` Makefile target
   - New target that uses OpenAI instead of Ollama
   - User passes `OPENAI_API_KEY` from environment

2. **Option B**: Make provider configurable via env var
   - Check `EDGEQUAKE_LLM_PROVIDER` first
   - Default to Ollama for local dev, OpenAI for tests

3. **Option C**: Add provider flag to existing target
   - `make backend-dev PROVIDER=openai`
   - More flexible but adds complexity

### Recommended Approach

**Option A** is cleanest:

- Add `backend-openai` Makefile target
- Requires `OPENAI_API_KEY` environment variable
- Uses `gpt-4o-mini` for chat, `text-embedding-3-small` for embeddings

## Next Steps (Orient Phase)

1. Analyze Makefile structure
2. Design new target for OpenAI
3. Update documentation
