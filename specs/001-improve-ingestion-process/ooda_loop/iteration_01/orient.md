# Iteration 01: Orient

**Mission Re-read**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

---

## Gap Analysis

### 1. Reprocess Document Functionality

| Current State | Desired State | Gap |
|---------------|---------------|-----|
| `reprocessDocument()` sends track_id | Works but no progress feedback | Need processing stage events |
| Backend cleans graph data before retry | Correct behavior | ✓ Good |
| Status updates to "pending" | Need "reprocessing" state | Missing intermediate state |
| No per-document error details | Show error reason to user | Need error message storage/display |

### 2. Rebuild Embeddings

| Current State | Desired State | Gap |
|---------------|---------------|-----|
| No dedicated "rebuild embeddings" endpoint | Isolated embedding regeneration | Missing endpoint |
| Workspace dimension stored | Use during rebuild | ✓ Available |
| No dimension change handling | Graceful migration | Missing migration logic |
| No provider switching support | OpenAI↔Ollama without corruption | Needs testing/validation |

### 3. Rebuild Knowledge Graph

| Current State | Desired State | Gap |
|---------------|---------------|-----|
| `rebuild_knowledge_graph()` clears all | Works but destructive | ✓ Intentional |
| Queues docs for reprocess | Need progress tracking | Missing batch progress |
| UX unclear about rebuild impact | Explicit warnings | Need confirmation dialogs |
| No partial rebuild option | Consider future enhancement | Out of scope for now |

### 4. UX/UI for Document Processing

| Current State | Desired State | Gap |
|---------------|---------------|-----|
| Single "processing" state | Multi-stage visibility | Need 4-5 sub-states |
| Aggregate pipeline stats | Per-document details | Need document-level view |
| No ETA calculation | Time estimates | Need processing time tracking |
| No stage transitions shown | Real-time updates | Need SSE/WebSocket |

### 5. Error Handling

| Current State | Desired State | Gap |
|---------------|---------------|-----|
| `status: failed` in metadata | Need `error_message` field | Partially exists, need display |
| Generic "Failed" badge | Show error category + reason | Need error parsing |
| No actionable suggestions | Guide user to fix | Need error→action mapping |
| Errors not easily copyable | Debug-friendly display | Need copy button + details |

---

## First Principles Analysis

### Why do users need better reprocessing UX?

1. **Uncertainty causes stress** - User doesn't know if system is working
2. **Failed operations need context** - Without reason, user can't fix issue
3. **Time perception** - Progress bars reduce perceived wait time
4. **Trust building** - Transparent operations build confidence

### What are the atomic units of work?

```
Document Processing = Sum of:
1. Content Storage       (~100ms)
2. Chunking             (~500ms per 10KB)
3. Entity Extraction    (~2-30s per chunk, LLM dependent)
4. Embedding Generation (~500ms per chunk)
5. Graph Upsert         (~100ms per entity)
6. Vector Index         (~50ms per embedding)
```

### Minimum viable improvement?

1. **Add processing sub-stages** - 4 states visible in UI
2. **Store error messages** - Already in metadata, just display
3. **Real-time progress** - Use existing SSE infrastructure

---

## Risk Assessment

| Approach | Benefit | Risk | Mitigation |
|----------|---------|------|------------|
| Add processing sub-states | Better UX | Breaking change for filters | Backward-compatible mapping |
| Store error details | Debug support | Privacy (leak internal errors) | Sanitize error messages |
| SSE for progress | Real-time | Connection overhead | Reuse existing stream infra |
| Ollama E2E tests | Realistic testing | Slow CI | Run in separate job |

---

## Solution Options

### Option A: Incremental Enhancement (RECOMMENDED)
1. Enhance status badge with sub-states
2. Display error messages from metadata
3. Add document-level progress in pipeline dialog
4. Create Ollama-based E2E tests

**Effort**: ~4-6 hours
**Risk**: Low
**Value**: High

### Option B: Full Rewrite with WebSocket
1. Replace polling with WebSocket
2. Real-time per-stage updates
3. Complete error framework

**Effort**: ~16-20 hours
**Risk**: High (breaking changes)
**Value**: Very High

### Option C: Minimal Fix
1. Just fix Loader2 import ✅
2. Display existing error field
3. No new features

**Effort**: ~1 hour
**Risk**: None
**Value**: Low

---

## Architecture Decision

**Selected: Option A (Incremental Enhancement)**

Rationale:
1. Uses existing infrastructure (minimal code changes)
2. Backward compatible (old clients still work)
3. Delivers immediate value (users see better progress)
4. Foundation for future WebSocket upgrade

---

## Proposed Processing States

```typescript
// Enhanced status with processing sub-states
type DocumentStatus = 
  | 'pending'      // Waiting in queue
  | 'chunking'     // Splitting document
  | 'extracting'   // LLM entity extraction
  | 'embedding'    // Generating vectors
  | 'indexing'     // Storing in databases
  | 'completed'    // Successfully processed
  | 'indexed'      // Alias for completed (backward compat)
  | 'failed'       // Error occurred
  | 'cancelled';   // User cancelled
```

---

## Priority Matrix

| Task | Impact | Effort | Priority |
|------|--------|--------|----------|
| Display error messages | High | Low | P1 |
| Add processing sub-states | High | Medium | P1 |
| Ollama E2E tests | Medium | Medium | P2 |
| Per-doc progress in dialog | Medium | Low | P2 |
| ETA calculation | Low | Medium | P3 |
| Error→action mapping | Medium | High | P3 |

---

## Next Step

Proceed to **Decide** phase to prioritize specific changes.
