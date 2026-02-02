# EdgeQuake: Document Processing Failure - Fix Plan

## Date: 2026-02-02

## Author: AI Assistant (Claude Sonnet 4.5)

## Executive Summary

Investigation of "stuck documents" revealed TWO distinct critical issues affecting document processing reliability:

1. **Stuck Document Transition** (4+ documents): PDFs complete conversion but never advance to chunking
2. **Entity Extraction Failures** (Qwen.pdf, 2 attempts): LLM returns empty/invalid responses

Both issues prevent documents from completing the ingestion pipeline and must be fixed for production reliability.

---

## Issue #1: Stuck Document Transition after PDF Conversion

### Symptoms

- Documents show: `status="processing"`, `current_stage="converting"`, `stage_progress=1.0`
- Stage message: `"Converting PDF to Markdown: page 15/15 (100%)"`
- Documents stuck for 30+ minutes with no progression

### Affected Documents (from API query)

```json
{
  "id": "ab101929-49b9-468b-810b-c33e7e0cbaa4",
  "file_name": "Gmail PDF",
  "current_stage": "converting",
  "stage_message": "Converting PDF to Markdown: page 2/2 (100%)",
  "created_at": "2026-02-02T04:36:07"
}
{
  "id": "4ec46339-4f50-4285-836a-7f7ba87220d6",
  "file_name": "001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf",
  "current_stage": "converting",
  "stage_message": "Converting PDF to Markdown: page 15/15 (100%)",
  "created_at": "2026-02-02T04:44:21"
}
{
  "id": "69af43fc-fef2-46e0-8598-db5a32e9a935",
  "file_name": "001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf",
  "current_stage": "converting",
  "stage_message": "Converting PDF to Markdown: page 15/15 (100%)",
  "created_at": "2026-02-02T04:47:41"
}
{
  "id": "01d1bbfc-f373-41de-9c6f-8759c5575e70",
  "file_name": "BYD HAN.pdf",
  "current_stage": "converting",
  "stage_message": "Converting PDF to Markdown: page 6/6 (100%)",
  "created_at": "2026-02-02T05:08:26"
}
```

### Root Cause Hypothesis

PDF conversion completes successfully (100%) but the orchestrator doesn't trigger the state transition to the next stage (chunking).

**Likely Issues:**

1. **Missing state update**: PDF processor completes but doesn't call `update_status()` to advance to `chunking` stage
2. **Callback not fired**: Pipeline progress callback for page 100% not triggering next stage
3. **Race condition**: Status update happens but gets overwritten or lost
4. **Dead task**: Background processing task crashed silently

### Investigation Required

```bash
# Check orchestrator code for stage transitions
cat edgequake/src/pipeline/processor.rs | grep -A 30 "stage.*converting\|stage.*transition"

# Look for PDF completion handling
cat edgequake/crates/edgequake-pdf/src/*.rs | grep -A 20 "100%\|complete"

# Check pipeline state machine
cat edgequake/crates/edgequake-pipeline/src/pipeline.rs | grep -A 30 "converting.*chunking"
```

### Fix Strategy

1. **Add explicit stage transition** after PDF conversion completes
2. **Add timeout detection** for stuck documents (>5 minutes at 100% = stuck)
3. **Implement recovery mechanism** to retry stage transition
4. **Enhanced logging** to trace exactly where the progression stops

### Code Locations to Fix

- `edgequake/src/pipeline/processor.rs` - Orchestrator stage transitions
- `edgequake/crates/edgequake-pdf/src/extractor.rs` - PDF completion callback
- `edgequake/crates/edgequake-pipeline/src/pipeline.rs` - Pipeline state machine

---

## Issue #2: Entity Extraction - Empty LLM Responses

### Symptoms

```
error_message: "Pipeline processing failed: Entity extraction error: All 1 chunks failed extraction.
Failures: Chunk 0: Entity extraction error: Invalid JSON: expected value at line 1 column 1"
```

### Affected Documents

- `97d8260f-838c-4604-809c-402da68aea91` (Qwen.pdf, first attempt)
- `340f6485-1723-4d16-8dbe-fefd2363cc33` (Qwen.pdf, second attempt - same PDF binary `9f44a258-0918-415a-bc6d-ebba8315c844`)

### Evidence from Logs

```
[backend] 2026-02-02T05:14:50.609199Z  WARN edgequake_pipeline::pipeline:
Chunk extraction failed, will retry chunk_index=0 chunk_id=fbc0f10f-0ade-4dd2-8aaf-956c7dab6fed-chunk-0
attempt=1 max_retries=3 error=Entity extraction error: Invalid JSON: expected value at line 1 column 1
```

System IS retrying (attempt 1 of 3), but all retries fail with same error.

### Test Results

**Manual Ollama Test** (SUCCESS):

````bash
curl http://localhost:11434/api/generate -d '{
  "model": "gemma3:12b",
  "prompt": "Extract entities... return JSON..."
}'

Response:
{
  "response": "```json\n{\n  \"entities\": [...], \"relationships\": []\n}\n```",
  "done": true,
  "done_reason": "stop"
}
````

✅ **Ollama CAN generate valid JSON** (wrapped in markdown blocks)
✅ **Parser SHOULD handle markdown wrapping** via `extract_json_from_response()`

### Root Cause Analysis

**Current Understanding:**

1. Ollama gemma3:12b returns valid JSON wrapped in ` ```json ... ``` `
2. The `extract_json_from_response()` function ([parser.rs:477-502](edgequake/crates/edgequake-pipeline/src/prompts/parser.rs#L477-L502)) SHOULD extract the JSON from markdown
3. Something in the actual entity extraction flow is causing empty responses

**Possible Causes:**

1. **Timeout Issue**: gemma3:12b timeout (120s) too short for complex PDFs
   - Qwen.pdf (833KB) may exceed 120s processing time
   - OpenAI timeout: 120s, Ollama recommended: 300s
2. **SOTA Prompt Incompatibility**: Tuple-format prompts don't work with gemma3:12b
   - System uses SOTA tuple format: `entity<|#|>Name<|#|>TYPE<|#|>Description`
   - gemma3:12b might not understand this format
3. **Empty Response from LLM**: Ollama returns empty string under certain conditions
   - Large context window exhaustion
   - Model crashes/hangs
   - Network issues with Ollama server

### Evidence: Debug Logging Added

Added debug statement in [extractor.rs](edgequake/crates/edgequake-pipeline/src/extractor.rs) to capture raw LLM response:

```rust
tracing::debug!(
    chunk_id = %chunk.id,
    attempt = attempt,
    response_len = response.content.len(),
    response_preview = %&response.content[..response.content.len().min(500)],
    finish_reason = ?response.finish_reason,
    "Raw LLM response received"
);
```

**Next Step**: Rebuild and test to see actual LLM response content.

### Fix Strategy (Priority Order)

#### Fix 1: Increase Ollama Timeout (IMMEDIATE)

```rust
// In edgequake-llm/src/providers/ollama.rs
const DEFAULT_TIMEOUT_SECS: u64 = 300; // Increase from 120s to 300s
```

#### Fix 2: Add Response Validation & Enhanced Logging

```rust
// In extractor.rs after LLM call
if response.content.trim().is_empty() {
    tracing::error!(
        chunk_id = %chunk.id,
        attempt = attempt,
        prompt_tokens = response.prompt_tokens,
        "LLM returned EMPTY response!"
    );
    return Err(PipelineError::ExtractionError("Empty LLM response".to_string()));
}

tracing::debug!(
    chunk_id = %chunk.id,
    response_len = response.content.len(),
    response_first_100 = %&response.content[..response.content.len().min(100)],
    "LLM response received"
);
```

#### Fix 3: Fallback to Simple JSON Prompt

If SOTA tuple prompts fail, retry with simpler JSON-based prompts:

```rust
if attempt > 1 && last_error.to_string().contains("Invalid JSON") {
    // Fallback: Try simple JSON prompt instead of SOTA tuple format
    let simple_prompt = format!(
        "Extract entities from this text. Return ONLY valid JSON:\n\
        {{\n  \"entities\": [...],\n  \"relationships\": []\n}}\n\nText:\n{}",
        chunk.content
    );
    // Retry with simplified prompt
}
```

#### Fix 4: Model Fallback Chain

```rust
// If gemma3:12b fails, try other models
const FALLBACK_MODELS: &[&str] = &[
    "gemma3:12b",    // Primary
    "gemma3:latest", // Fallback 1
    "llama3:8b",     // Fallback 2
];
```

### Testing Plan

1. **Rebuild with new logging**: `cd edgequake && cargo build --release`
2. **Test with Qwen.pdf**: Upload via UI and monitor logs
3. **Capture raw LLM response**: Check `/tmp/backend-manual.log` for debug output
4. **Test simple vs SOTA prompts**: Compare success rates
5. **Test timeout increase**: Verify documents complete successfully

---

## Implementation Priority

### Phase 1: Immediate Fixes (Today)

1. ✅ Add debug logging for raw LLM responses
2. ⏳ Increase Ollama timeout from 120s → 300s
3. ⏳ Add empty response validation
4. ⏳ Test with Qwen.pdf and capture actual LLM response

### Phase 2: Stage Transition Fix (Tomorrow)

1. ⏳ Investigate PDF completion → chunking transition
2. ⏳ Add explicit stage transition after 100% conversion
3. ⏳ Implement timeout detection for stuck documents
4. ⏳ Add recovery mechanism to retry transitions

### Phase 3: Robustness Improvements (This Week)

1. ⏳ Implement prompt fallback (SOTA → Simple JSON)
2. ⏳ Add model fallback chain
3. ⏳ Enhanced error messages in UI
4. ⏳ Retry button for failed documents

---

## Success Criteria

### Issue #1 (Stuck Documents)

- ✅ All 4 stuck documents can be reprocessed successfully
- ✅ New documents never get stuck at 100% conversion
- ✅ Automatic recovery if a document gets stuck
- ✅ Clear error messages if stage transition fails

### Issue #2 (Entity Extraction)

- ✅ Qwen.pdf processes successfully to completion
- ✅ <5% entity extraction failure rate
- ✅ Clear error messages showing exact LLM failure
- ✅ Automatic retry with fallback prompts

---

## Logging & Monitoring

### Critical Log Points

1. PDF conversion completion (100%) - before transition
2. Stage transition triggered - confirm chunking starts
3. LLM request - prompt length, model, timeout
4. LLM response - raw content, length, parse success
5. Extraction failure - exact error, attempt number

### Metrics to Track

- Documents stuck >5 minutes at any stage
- Entity extraction failure rate by model
- Average extraction time by chunk size
- Retry success rate by attempt number

---

## Related Code Files

### Issue #1 Files

- `edgequake/src/pipeline/processor.rs` - PDF processing orchestrator
- `edgequake/crates/edgequake-pdf/src/extractor.rs` - PDF extraction
- `edgequake/crates/edgequake-pipeline/src/pipeline.rs` - Pipeline state machine

### Issue #2 Files

- `edgequake/crates/edgequake-pipeline/src/extractor.rs` - Entity extraction (MODIFIED with debug logging)
- `edgequake/crates/edgequake-pipeline/src/prompts/parser.rs` - Response parsing
- `edgequake/crates/edgequake-llm/src/providers/ollama.rs` - Ollama provider

---

## Next Actions

1. **Restart services** with newly built binary containing debug logging
2. **Upload test document** (Qwen.pdf or similar) to trigger entity extraction
3. **Capture raw LLM response** from debug logs to see what gemma3:12b actually returns
4. **Analyze failure** - empty response? malformed JSON? timeout?
5. **Implement appropriate fix** based on findings
6. **Test all stuck documents** to verify they can be recovered

---

## Notes

- User feedback: "It far from perfect --> I have many documents stuck"
- User expectation: Deep analysis + comprehensive fix
- Previous session: Successfully fixed WebSocket warnings (70+ → 0)
- Current session: Must fix document processing reliability

**Status**: Investigation complete, fixes identified, awaiting rebuild and test
