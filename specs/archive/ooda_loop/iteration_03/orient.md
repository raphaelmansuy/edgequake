# Iteration 03: Orient

**Mission Re-read**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

---

## Gap Analysis

### What's Already Working ✅

| Feature             | Implementation                | Evidence                                     |
| ------------------- | ----------------------------- | -------------------------------------------- |
| Rebuild Embeddings  | workspaces.rs:1343            | Full handler with workspace isolation        |
| Rebuild KG          | workspaces.rs:1729            | Full handler with optional embedding rebuild |
| Workspace Isolation | clear_workspace()             | Scoped by workspace_id                       |
| Dimension Handling  | Auto-detect from model config | OODA-225 implemented                         |
| Cache Eviction      | vector_registry.evict()       | Prevents stale dimension issues              |
| Frontend Buttons    | RebuildEmbeddingsButton       | Card UI with confirmation                    |
| Progress Dialog     | PipelineStatusDialog          | Polls for status                             |

### What Needs Improvement ⚠️

| Gap                                | Impact              | Effort | Priority |
| ---------------------------------- | ------------------- | ------ | -------- |
| E2E tests with Ollama              | Testing reliability | Medium | P1       |
| Status update during rebuild       | UX clarity          | Low    | P2       |
| Confirmation dialog impact preview | User confidence     | Low    | P2       |
| Error detail display               | Debug support       | Medium | P2       |

---

## First Principles Analysis

### Why is rebuild functionality critical?

1. **Model migration** - Users switch providers (OpenAI → Ollama)
2. **Quality improvement** - New models extract better entities
3. **Dimension compatibility** - Different models have different dimensions
4. **Recovery** - Fix corrupted data

### What's the user's mental model?

"I want to upgrade my extraction quality without losing data"

The system needs to:

1. Preserve source documents
2. Clear derived data (entities, relationships, embeddings)
3. Reprocess with new configuration
4. Show progress throughout

---

## Decision Points

### Decision 1: E2E Tests Priority

**Focus**: Create comprehensive E2E tests using Ollama models for:

- Rebuild embeddings with dimension change
- Rebuild KG with model change
- Error scenarios (provider unavailable)
- Workspace isolation verification

### Decision 2: Enhance Confirmation Dialog

Add impact preview to RebuildEmbeddingsButton:

```tsx
"This will reprocess {X} documents ({Y} chunks) using {newModel}.";
```

### Decision 3: Add Status Updates During Rebuild

The rebuild process should show stages:

1. "Clearing vectors..."
2. "Updating configuration..."
3. "Queueing documents..."
4. "Processing..." (then normal pipeline stages)

---

## Risk Assessment

| Risk                       | Probability | Mitigation                   |
| -------------------------- | ----------- | ---------------------------- |
| Ollama not available in CI | High        | Skip tests gracefully        |
| Long test runtime          | Medium      | Use small test documents     |
| Flaky tests from timing    | Low         | Add proper waits and retries |

---

## Solution Approach

### Phase 1: E2E Test Suite (This Iteration)

Create `edgequake_webui/e2e/rebuild-operations.spec.ts`:

1. Test rebuild embeddings flow
2. Test rebuild KG flow
3. Test dimension change scenario
4. Test workspace isolation
5. Ollama integration tests (conditional)

### Phase 2: UX Improvements (Next Iteration)

1. Enhance confirmation dialogs
2. Add status messages during rebuild
3. Improve error display

---

## Next Step

Proceed to **Decide** phase to finalize the E2E test plan.
