# OODA Iteration 09: ACT

## Actions Taken

### 1. Fixed Provider Factory Detection (factory.rs)

**Problem:** The factory uses `EDGEQUAKE_LLM_PROVIDER` (not `EDGEQUAKE_DEFAULT_LLM_PROVIDER`) for explicit provider selection.

**Solution:** Updated Makefile to use the correct environment variable name.

### 2. Fixed Ollama Embedding Truncation (ollama.rs)

Added text truncation before calling Ollama embedding API:

```rust
/// Maximum tokens for embedding to prevent Ollama context overflow
/// nomic-embed-text has 2048 token context, we use 2000 to be safe
embedding_max_tokens: usize,

/// Truncate text to fit within embedding model's context window
fn truncate_for_embedding(&self, text: &str) -> String {
    let max_chars = self.embedding_max_tokens * 4; // ~4 chars per token
    if text.len() <= max_chars {
        return text.to_string();
    }
    tracing::warn!(
        text_len = text.len(),
        max_chars = max_chars,
        "Truncating text for embedding (exceeds model context)"
    );
    text.chars().take(max_chars).collect()
}
```

### 3. Fixed Makefile dev-bg Target

**Root Cause:** The `dev-bg` target had its own backend startup code that:

1. Used wrong env var name (`EDGEQUAKE_DEFAULT_LLM_PROVIDER` instead of `EDGEQUAKE_LLM_PROVIDER`)
2. Always set `OLLAMA_HOST` which triggered Ollama auto-detection even with OpenAI configured

**Fix:** Updated `dev-bg` to conditionally set environment variables based on OPENAI_API_KEY:

```makefile
@if [ -n "$(OPENAI_API_KEY)" ]; then \
    cd $(BACKEND_DIR) && \
        DATABASE_URL="$(DATABASE_URL)" \
        PDFIUM_DYNAMIC_LIB_PATH="$(PDFIUM_LIB_PATH)" \
        OPENAI_API_KEY="$(OPENAI_API_KEY)" \
        EDGEQUAKE_LLM_PROVIDER="openai" \
        nohup cargo run > /tmp/edgequake-backend.log 2>&1 & \
else \
    cd $(BACKEND_DIR) && \
        DATABASE_URL="$(DATABASE_URL)" \
        PDFIUM_DYNAMIC_LIB_PATH="$(PDFIUM_LIB_PATH)" \
        EDGEQUAKE_LLM_PROVIDER="ollama" \
        OLLAMA_HOST="http://localhost:11434" \
        OLLAMA_MODEL="gemma3:latest" \
        OLLAMA_EMBEDDING_MODEL="nomic-embed-text" \
        nohup cargo run > /tmp/edgequake-backend.log 2>&1 & \
fi
```

**Key insight:** OLLAMA_HOST is NOT set when using OpenAI, preventing the factory's auto-detect from falling through to Ollama.

### 4. Updated Status Output

Fixed the status message to accurately reflect which provider is being used.

## Verification

```bash
$ curl -s http://localhost:8080/health | jq '{status, llm_provider_name, storage_mode}'
{
  "status": "healthy",
  "llm_provider_name": "openai",
  "storage_mode": "postgresql"
}
```

**OpenAI is now correctly selected when OPENAI_API_KEY is set.**

## Test Results

- ✅ 199 tests pass in edgequake-llm crate
- ✅ Backend starts with OpenAI provider
- ✅ Health endpoint reports `"llm_provider_name": "openai"`

## Files Modified

1. `edgequake/crates/edgequake-llm/src/providers/ollama.rs` - Added embedding truncation
2. `Makefile` - Fixed dev-bg target for proper provider selection

## Next Steps

1. Test document upload with OpenAI
2. Verify entity extraction produces results
3. Test delete document with cleanup
4. Battle test with audit comparison documents

## Lessons Learned

1. **Check which Make target is actually executing** - `dev-bg` had its own copy of backend startup logic separate from `backend-bg`
2. **Factory auto-detect priority matters** - Setting OLLAMA_HOST triggers Ollama even if OPENAI_API_KEY is set
3. **Use correct env var names** - `EDGEQUAKE_LLM_PROVIDER` for factory detection, not `EDGEQUAKE_DEFAULT_LLM_PROVIDER`
