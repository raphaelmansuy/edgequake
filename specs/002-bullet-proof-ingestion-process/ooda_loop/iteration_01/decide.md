# Iteration 01 - Decide

**Date**: 2026-01-28 15:20:00

**Mission Status**: ✅ Re-read mission file

## Chosen Solution

**Add HTTP-Level Timeout with Detailed Logging**

### Rationale

1. **Critical Bug Fix**: Indefinite hangs are unacceptable in production
2. **Fastest Path to Resolution**: < 30 minutes implementation
3. **Low Risk**: Single file change, isolated to synchronous upload path
4. **Provides Diagnostic Data**: Timeout logs will reveal actual processing time
5. **Foundation for Further Work**: Necessary before optimizing performance

### Alternative Solutions Rejected

#### Auto-Force Async Mode
- **Why Rejected**: Requires frontend changes, breaks API contract for sync mode
- **When to Revisit**: After implementing task polling UI (Iteration 5-8)

#### Optimize Ollama Speed
- **Why Rejected**: Unknown problem scope, no guarantee of success
- **When to Revisit**: After gathering per-chunk timing data (Iteration 3-5)

#### Streaming Progress
- **Why Rejected**: Too complex for immediate fix, requires infrastructure changes
- **When to Revisit**: Long-term improvement (Iteration 15+)

## Implementation Plan

### File Changes

#### File 1: `edgequake/crates/edgequake-api/src/handlers/documents.rs`
**Location**: Lines 645-660 (synchronous processing section)

**Before**:
```rust
// SPEC-032: Use workspace-specific pipeline with workspace LLM configuration
// This ensures the workspace's LLM model is used for entity extraction
let workspace_pipeline = state
    .create_workspace_pipeline(&workspace_id_for_storage)
    .await;
let result = workspace_pipeline
    .process(&document_id, &request.content)
    .await?;
```

**After**:
```rust
// SPEC-032: Use workspace-specific pipeline with workspace LLM configuration
// This ensures the workspace's LLM model is used for entity extraction
let workspace_pipeline = state
    .create_workspace_pipeline(&workspace_id_for_storage)
    .await;

// OODA-01: Add HTTP-level timeout to prevent indefinite hangs
// WHY: Large documents (100KB+) can take 5-10 minutes to process,
// but HTTP clients expect responses within 60-120 seconds.
// Without this timeout, requests hang indefinitely causing poor UX.
//
// Timeout Strategy:
// - 120 seconds (2 minutes): Conservative limit for synchronous mode
// - For larger documents, users should use async_processing: true
// - Timeout applies to ENTIRE pipeline, not just individual LLM calls
//
// See: specs/002-bullet-proof-ingestion-process.md
const SYNC_PROCESSING_TIMEOUT_SECS: u64 = 120;

let processing_start = std::time::Instant::now();
tracing::info!(
    document_id = %document_id,
    content_length = request.content.len(),
    timeout_secs = SYNC_PROCESSING_TIMEOUT_SECS,
    "Starting synchronous document processing"
);

let result = tokio::time::timeout(
    std::time::Duration::from_secs(SYNC_PROCESSING_TIMEOUT_SECS),
    workspace_pipeline.process(&document_id, &request.content)
)
.await
.map_err(|_elapsed| {
    let processing_time = processing_start.elapsed();
    tracing::error!(
        document_id = %document_id,
        timeout_secs = SYNC_PROCESSING_TIMEOUT_SECS,
        processing_time_secs = processing_time.as_secs(),
        content_length = request.content.len(),
        "Document processing timeout - consider using async mode for large documents"
    );
    ApiError::Timeout(format!(
        "Document processing exceeded {} seconds. For large documents (>50KB), \
         use async_processing: true to avoid timeouts. \
         Current document size: {} bytes",
        SYNC_PROCESSING_TIMEOUT_SECS,
        request.content.len()
    ))
})??;

let processing_time = processing_start.elapsed();
tracing::info!(
    document_id = %document_id,
    processing_time_secs = processing_time.as_secs(),
    processing_time_ms = processing_time.as_millis(),
    chunk_count = result.chunks.len(),
    entity_count = result.stats.entity_count,
    "Document processing completed successfully"
);
```

**Line Range**: 645-660 → 645-700 (approximately 55 lines added)

**Why This Approach**:
1. **Minimal Changes**: Only wraps existing call, no restructuring
2. **Clear Error Message**: Guides users to async mode
3. **Detailed Logging**: Start/end logs with timing
4. **Conservative Timeout**: 120s allows most small/medium docs

### New Files

None - all changes in existing file.

### Tests to Write/Modify

#### Test 1: Timeout Enforcement (New)
**File**: `edgequake/crates/edgequake-api/tests/document_upload_timeout.rs`

```rust
#[tokio::test]
async fn test_synchronous_upload_timeout_enforcement() {
    // Mock pipeline that sleeps for 150 seconds
    // Assert that request fails with ApiError::Timeout after 120 seconds
    // Assert error message contains "use async_processing: true"
}
```

**Priority**: High (validates fix)

#### Test 2: Small Document Success (Existing)
**File**: `edgequake/crates/edgequake-api/tests/integration_tests.rs`

**Modification**: Add assertion that small documents (< 10KB) complete in < 30 seconds

```rust
#[tokio::test]
async fn test_small_document_upload_fast() {
    let content = "Small document content (< 1KB)";
    let start = std::time::Instant::now();
    let response = upload_document(content).await;
    let duration = start.elapsed();
    
    assert!(duration.as_secs() < 30, "Small doc took too long");
    assert_eq!(response.status, "processed");
}
```

**Priority**: Medium (regression check)

#### Test 3: Manual Test - 86KB Document (Critical)
**Command**:
```bash
cd /Users/raphaelmansuy/Github/03-working/edgequake/zz-explore/test_docs
cat aws_2601.08734v1.extracted.md | \
  python3 -c 'import json, sys; print(json.dumps({"title": "AWS TerraFormer Paper", "content": sys.stdin.read()}))' > /tmp/test_aws.json

curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d @/tmp/test_aws.json \
  -w "\nHTTP Status: %{http_code}\nTime: %{time_total}s\n"
```

**Expected Outcome**:
- ✅ Completes successfully in < 120 seconds
- ✅ Returns 201 with document_id
- ✅ Backend logs show processing time

**Priority**: CRITICAL (validates fix with real data)

#### Test 4: Manual Test - 121KB Document (Expected Timeout)
**Command**:
```bash
cd /Users/raphaelmansuy/Github/03-working/edgequake/zz-explore/test_docs
cat scienti_2601.16282v1.extracted.md | \
  python3 -c 'import json, sys; print(json.dumps({"title": "Scientific Paper Test", "content": sys.stdin.read()}))' > /tmp/test_scienti.json

curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d @/tmp/test_scienti.json \
  -w "\nHTTP Status: %{http_code}\nTime: %{time_total}s\n"
```

**Expected Outcome**:
- ❌ Fails with timeout error after ~120 seconds
- ❌ Returns 408 or 500 with clear error message
- ✅ Error message suggests using async mode
- ✅ Backend logs show timeout with document size

**Priority**: CRITICAL (validates timeout works)

### Documentation Updates

#### File: `docs/api/document-upload.md`
**Section to Add**: "Synchronous vs Asynchronous Processing"

```markdown
## Processing Modes

### Synchronous Processing (Default)

For small to medium documents (< 50KB), the API processes documents synchronously and returns the result immediately.

**Timeout**: 120 seconds
**Best For**: Interactive uploads, documents < 50KB

Example:
```json
POST /api/v1/documents
{
  "title": "My Document",
  "content": "Document text here..."
}
```

### Asynchronous Processing (Recommended for Large Documents)

For large documents (> 50KB), use asynchronous mode to avoid timeout errors.

**Timeout**: None (background processing)
**Best For**: Batch uploads, documents > 50KB, PDFs

Example:
```json
POST /api/v1/documents
{
  "title": "Large Document",
  "content": "...",
  "async_processing": true
}
```

Response:
```json
{
  "document_id": "...",
  "status": "pending",
  "task_id": "abc-123"
}
```

Poll task status:
```
GET /api/v1/tasks/{task_id}
```

### Timeout Handling

If a synchronous upload exceeds 120 seconds, you'll receive:

**HTTP 408 Request Timeout**
```json
{
  "code": "TIMEOUT",
  "message": "Document processing exceeded 120 seconds. For large documents (>50KB), use async_processing: true to avoid timeouts. Current document size: 121000 bytes"
}
```

**Action**: Retry with `"async_processing": true`
```

**Priority**: High (user-facing documentation)

#### File: `specs/002-bullet-proof-ingestion-process/ooda_loop/iteration_01/act.md`
**Content**: Changes made, commit SHA, test results (to be created after implementation)

## Validation Criteria (Specific, Measurable)

### Success Criteria

| Criterion | Metric | Target | Validation Method |
|-----------|--------|--------|-------------------|
| **Timeout Enforcement** | Request fails after timeout | 120 seconds ± 5s | curl with `-w time_total` |
| **Small Doc Success** | 86KB doc processes | < 120 seconds | Manual test + logs |
| **Error Message Quality** | Message mentions async mode | 100% of timeouts | Read error response |
| **Logging Detail** | Start/end logs present | 100% of requests | grep backend logs |
| **No Regression** | Existing tests pass | 100% pass rate | `cargo test` |

### Failure Criteria (Rollback Triggers)

| Failure | Impact | Rollback Action |
|---------|--------|-----------------|
| **Timeout too aggressive** | Small docs fail | Increase to 300s |
| **Timeout not working** | Still hangs indefinitely | Revert changes |
| **Test suite breaks** | >10% tests fail | Revert changes |
| **Performance regression** | >20% slower processing | Revert changes |

### Metrics to Track

#### Pre-Implementation Baseline
- 875-byte test doc: 25.1 seconds (from previous test)
- 86KB doc: UNKNOWN (hangs)
- 121KB doc: UNKNOWN (hangs)

#### Post-Implementation Targets
- 875-byte test doc: < 30 seconds (no regression)
- 86KB doc: < 120 seconds (success)
- 121KB doc: ~120 seconds timeout (expected failure with clear error)

## Rollback Plan

### Scenario 1: Timeout Too Aggressive
**Symptoms**: Small documents (< 10KB) failing with timeout errors

**Action**:
```rust
// Change timeout from 120s to 300s
const SYNC_PROCESSING_TIMEOUT_SECS: u64 = 300;
```

**Test**: Retry failed document upload
**Time**: < 5 minutes

### Scenario 2: Timeout Not Working
**Symptoms**: Requests still hang indefinitely, no timeout error

**Action**:
```bash
cd /Users/raphaelmansuy/Github/03-working/edgequake/edgequake
git revert HEAD  # Revert timeout commit
cargo build
make backend-bg  # Restart backend
```

**Test**: Verify existing functionality not broken
**Time**: < 10 minutes

### Scenario 3: Breaking Changes
**Symptoms**: Test suite fails, compilation errors

**Action**:
```bash
git revert HEAD
cargo test --all
```

**Test**: All tests pass
**Time**: < 15 minutes

## Implementation Checklist

### Pre-Implementation
- [x] Re-read mission file
- [x] Understand current code structure
- [x] Identify exact line numbers for changes
- [x] Plan error handling strategy
- [x] Define success criteria

### Implementation
- [ ] Add timeout wrapper in documents.rs (lines 645-660)
- [ ] Add detailed logging (start/end/error)
- [ ] Add const for timeout value
- [ ] Add helpful error message
- [ ] Verify code compiles

### Testing
- [ ] Run `cargo build --package edgequake-api`
- [ ] Run `cargo test --package edgequake-api`
- [ ] Manual test: 86KB document (expect success)
- [ ] Manual test: 121KB document (expect timeout)
- [ ] Check backend logs for timing data

### Documentation
- [ ] Update API documentation
- [ ] Update act.md with results
- [ ] Commit with OODA-01 prefix
- [ ] Update success metrics table

### Validation
- [ ] Verify timeout enforced (curl test)
- [ ] Verify small docs still work (regression)
- [ ] Verify error message helpful
- [ ] Verify logs detailed enough
- [ ] No breaking changes to tests

## Next Iteration Focus

After completing this iteration, focus on:

1. **Analyze Timeout Results**: 
   - Did 86KB doc complete in time?
   - What was actual processing time?
   - Did 121KB doc timeout as expected?

2. **Investigate Per-Chunk Timing**:
   - Why only 1 "Ollama chat request" log?
   - Are chunks being processed in parallel?
   - What is actual per-chunk processing time?

3. **Optimize Timeout Value**:
   - Is 120s sufficient for medium docs (50-100KB)?
   - Should timeout scale with document size?
   - Should we differentiate by provider (Ollama vs OpenAI)?

4. **Plan Async Mode Improvements**:
   - Frontend UI for task polling
   - Auto-detection of large documents
   - Progress indicators

## Time Estimate

- **Implementation**: 15 minutes
- **Testing**: 15 minutes
- **Documentation**: 10 minutes
- **Total**: 40 minutes

## Dependencies

- None (standalone change)

## Breaking Changes

- None (only affects error handling, not API contract)
