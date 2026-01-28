# Iteration 01 - Orient

**Date**: 2026-01-28 15:10:00

**Mission Status**: ✅ Re-read mission file

## First Principles Analysis

### Fundamental Requirement
**Transform text → knowledge graph via LLM-based extraction**

**Immutable Constraints**:
1. **LLM API calls are slow**: Entity extraction from 121KB document takes 5-10 minutes minimum
2. **LLM context limits**: Models have maximum context windows (gemma3:12b ≈ 4096 tokens)
3. **Network timeouts**: HTTP requests cannot wait indefinitely
4. **Memory limits**: Loading 121KB in single LLM call may exceed context window
5. **Rate limits**: LLM providers enforce requests/minute limits

**Challengeable Assumptions**:
1. ❌ **"Must process entire document synchronously"**
   - **Challenge**: HTTP clients expect response within 60-120 seconds
   - **Reality**: Large documents require 5-10 minutes processing
   - **Solution**: Use async mode with task queue

2. ❌ **"One LLM call per document"**
   - **Challenge**: 121KB likely exceeds model context window
   - **Reality**: Pipeline DOES chunk (found in code: ChunkerConfig)
   - **Solution**: Pipeline already handles chunking correctly

3. ✅ **"600-second timeout is sufficient"**
   - **Validate**: Pipeline config shows `chunk_extraction_timeout_secs: 60` (default)
   - **Reality**: Timeout is PER CHUNK, not total document
   - **Issue**: No timeout at HTTP handler level

## Root Cause Analysis

### Hypothesis A: Missing HTTP-Level Timeout ✅ CONFIRMED
**Probability**: 95%

**Evidence Supporting**:
1. `handlers/documents.rs:657` shows direct `.await` on `workspace_pipeline.process()`
2. No `tokio::time::timeout` wrapper found in handler code
3. Request hangs indefinitely when tested (> 3 minutes, no response)
4. Backend logs show "Ollama chat request" but never complete

**Evidence Contradicting**:
- None

**Conclusion**: **ROOT CAUSE IDENTIFIED**

### Hypothesis B: Ollama Slow on Large Context ✅ VALIDATED
**Probability**: 85%

**Evidence Supporting**:
1. Pipeline chunks document into smaller pieces (ChunkerConfig default: 1200 tokens)
2. Each chunk processed separately with 60-second timeout per chunk
3. 121KB document ≈ 30,000 tokens ≈ 25 chunks
4. Total processing time = 25 chunks × extraction time per chunk
5. If each chunk takes 2-3 minutes, total = 50-75 minutes!

**Calculation**:
```
Document Size: 123,909 bytes ≈ 30,975 tokens (4 bytes/token)
Chunk Size: 1200 tokens with 100 token overlap
Number of Chunks: ⌈30,975 / 1200⌉ ≈ 26 chunks

IF per-chunk extraction takes 2 minutes:
Total Time = 26 × 2 min = 52 minutes

IF per-chunk extraction takes 3 minutes:
Total Time = 26 × 3 min = 78 minutes
```

**Evidence Contradicting**:
- Previous small test (875 bytes) took only 25 seconds total
- Suggests per-chunk processing can be fast with Ollama

**Conclusion**: **SECONDARY ISSUE** - Ollama is slow, but HTTP timeout is the blocking issue

### Hypothesis C: Per-Chunk Timeout Too Aggressive ❓ NEEDS VALIDATION
**Probability**: 50%

**Evidence Supporting**:
1. Pipeline config: `chunk_extraction_timeout_secs: 60` (default)
2. If Ollama takes > 60s per chunk, extraction will fail
3. No error logs showing timeout failures → timeout may not be triggering

**Evidence Contradicting**:
1. Small test document (875 bytes) succeeded in 25s
2. Suggests 60s timeout is sufficient for small chunks

**Testing Needed**:
1. Check Ollama processing logs to see actual per-chunk time
2. Instrument pipeline with per-chunk timing logs
3. Verify timeout is actually being applied (code review)

## Risk Assessment

### Option A: Add HTTP-Level Timeout (Immediate Fix)
**Description**: Wrap `workspace_pipeline.process()` call with `tokio::time::timeout`

**Pros**:
- ✅ Simple fix (5 lines of code)
- ✅ Prevents indefinite hangs
- ✅ Provides clear error message to user
- ✅ Fast to implement and test

**Cons**:
- ❌ Doesn't solve underlying issue (large docs still fail)
- ❌ User gets timeout error instead of hang (better, but not ideal)

**Estimated Effort**: 15 minutes

**Blast Radius**: Low - only affects synchronous upload path

**Rollback Strategy**: Simple revert (single file change)

### Option B: Force Async Mode for Large Documents
**Description**: Auto-detect document size and force `async_processing: true` for docs > 50KB

**Pros**:
- ✅ Solves hang issue for large documents
- ✅ Leverages existing task queue infrastructure
- ✅ User gets immediate response with task_id for polling
- ✅ No timeout errors

**Cons**:
- ❌ Requires frontend changes to poll task status
- ❌ Changes API behavior (breaking change for large docs)
- ❌ Doesn't help if user explicitly requests sync mode

**Estimated Effort**: 2 hours (backend + frontend changes)

**Blast Radius**: Medium - changes upload behavior for large documents

**Rollback Strategy**: Moderate - requires frontend deployment

### Option C: Optimize Ollama Processing Speed
**Description**: Investigate why Ollama is slow, optimize prompts, batch processing

**Pros**:
- ✅ Long-term solution improving overall performance
- ✅ Benefits all document sizes
- ✅ Reduces costs and latency

**Cons**:
- ❌ Time-consuming investigation (unknown duration)
- ❌ May not be fixable (model inherent speed)
- ❌ Doesn't solve HTTP timeout issue

**Estimated Effort**: 4-8 hours investigation + implementation

**Blast Radius**: High - changes core extraction logic

**Rollback Strategy**: Complex - requires performance testing

### Option D: Implement Streaming Progress (Long-Term)
**Description**: Use WebSocket or Server-Sent Events to stream extraction progress

**Pros**:
- ✅ Best UX - user sees real-time progress
- ✅ No HTTP timeout issues (long-lived connection)
- ✅ Can cancel mid-processing

**Cons**:
- ❌ Significant frontend changes
- ❌ Complex implementation (WebSocket infrastructure)
- ❌ Doesn't help existing API clients

**Estimated Effort**: 1-2 days (backend + frontend)

**Blast Radius**: High - new API endpoints, frontend rewrite

**Rollback Strategy**: Complex - requires feature flagging

## Solution Candidates

### Recommended: Layered Approach

**Phase 1 (Immediate - This Iteration)**:
1. ✅ Add HTTP-level timeout (120 seconds) to prevent indefinite hangs
2. ✅ Add detailed logging to track per-chunk processing time
3. ✅ Test with 86KB document to validate fix

**Phase 2 (Short-Term - Next 3 Iterations)**:
1. Auto-detect large documents (> 50KB) and recommend async mode
2. Add frontend support for task polling
3. Optimize per-chunk timeout (increase to 120s if needed)

**Phase 3 (Long-Term - Iterations 10+)**:
1. Implement streaming progress via Server-Sent Events
2. Investigate Ollama performance optimization
3. Add adaptive timeout based on document size

## Decision Criteria

**For Phase 1 (Immediate Fix)**:
- ✅ Prevents indefinite hangs (critical bug)
- ✅ < 30 minutes implementation time
- ✅ Low risk (single file change)
- ✅ Provides actionable error message
- ✅ Doesn't break existing functionality

**Success Metrics**:
- HTTP request fails with clear timeout error after 120 seconds
- Backend logs show detailed per-chunk timing
- 86KB document processes successfully (< 120s)

**Failure Criteria**:
- Timeout still doesn't trigger (code issue)
- Timeout too aggressive (small docs fail)
- No improvement in user experience

## Architectural Insights

### Current Pipeline Flow (Discovered)
```
upload_document() [NO TIMEOUT]
        ↓
   workspace_pipeline.process() [NO TIMEOUT]
        ↓
   chunker.chunk() [FAST - regex based]
        ↓
   extract_parallel() [SEMAPHORE: 16 concurrent]
        ↓
   ┌────────────────────────────┐
   │  For each chunk (parallel):│
   │  extractor.extract()       │ [PER-CHUNK: 60s timeout]
   │    ↓                       │
   │  LLM API call              │
   │    ↓                       │
   │  Parse entities/rels       │
   └────────────────────────────┘
        ↓
   embed() [BATCHED: 100 embeddings]
        ↓
   Return ProcessingResult
```

**Key Discovery**: The `extract_parallel()` function processes up to 16 chunks concurrently with a semaphore. This is GOOD for performance but means:
- 26 chunks with 16 concurrency = 2 batches
- If each chunk takes 60s, total time ≈ 120s (not 52 minutes!)
- **BUT**: Logs show only 1 "Ollama chat request" → may not be parallelizing correctly

### Timeout Architecture (Discovered)
```
HTTP Request [NO TIMEOUT]
     ↓
Handler [NO TIMEOUT]
     ↓
Pipeline Process [NO TIMEOUT]
     ↓
Extract Parallel [NO TIMEOUT]
     ↓
Extractor.extract() [60s TIMEOUT per chunk]
     ↓
LLM Provider [600s TIMEOUT per API call]
```

**Missing Layer**: HTTP/Handler level timeout is CRITICAL

## Questions Answered

1. ✅ **What is the actual timeout configured in Axum/Tower middleware?**
   - Answer: NONE - no timeout found in handler code

2. ✅ **Does the pipeline chunk the document before entity extraction?**
   - Answer: YES - ChunkerConfig with 1200 token chunks, 100 token overlap

3. ❓ **Why is there no "Ollama response received" log after the "chat request" log?**
   - Needs Investigation: Check Ollama provider code for response logging

4. ❓ **Is Ollama actually processing or is the connection hanging?**
   - Needs Testing: Monitor Ollama process CPU/memory during upload

5. ❓ **What is the maximum context window for gemma3:12b model?**
   - Needs Research: Check Ollama documentation

## New Questions for Next Iteration

1. ❓ Why does `extract_parallel()` only show 1 "Ollama chat request" log for a 26-chunk document?
2. ❓ Is the semaphore actually allowing 16 concurrent extractions?
3. ❓ What is the actual per-chunk processing time with Ollama?
4. ❓ Should we increase per-chunk timeout from 60s to 120s?

## Decision for Act Phase

**Chosen Solution**: Option A - Add HTTP-Level Timeout (Phase 1)

**Rationale**:
1. **Immediate Impact**: Prevents indefinite hangs (critical bug)
2. **Low Risk**: Single file change, easy to test and rollback
3. **Fast Implementation**: < 30 minutes
4. **Provides Data**: Timeout logs will show how long processing actually takes
5. **Foundation**: Necessary first step before other optimizations

**Implementation Plan**:
1. Add `tokio::time::timeout` wrapper around `workspace_pipeline.process()` call
2. Set timeout to 120 seconds (2 minutes) as conservative starting point
3. Return clear error message: "Document processing timeout - try async mode for large documents"
4. Add detailed logging for pipeline processing time

**Validation Criteria**:
- ✅ Request fails with timeout error after 120 seconds (not indefinite hang)
- ✅ 86KB document processes successfully (< 120s)
- ✅ Error message guides user to async mode
- ✅ Backend logs show per-chunk timing

**Rollback Plan**:
If timeout too aggressive (small docs fail):
1. Increase timeout to 300 seconds (5 minutes)
2. Or remove timeout and revert to original behavior
3. Single file change, < 5 minutes to rollback
