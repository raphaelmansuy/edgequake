# Iteration 03: Decide

**Mission Re-read**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

---

## Prioritized Action Plan

### Decision 1: Create E2E Test Suite for Rebuild Operations

**Action**: Create `e2e/rebuild-operations.spec.ts` with comprehensive tests

**Test Cases**:

1. `rebuild embeddings button is accessible from workspace page`
2. `rebuild KG button is accessible from workspace page`
3. `rebuild embeddings shows confirmation dialog`
4. `rebuild KG shows confirmation dialog`
5. `rebuild triggers progress dialog`
6. `workspace isolation - rebuild doesn't affect other workspaces`
7. `Ollama: rebuild embeddings with dimension change` (conditional)
8. `Ollama: rebuild KG with model change` (conditional)

### Decision 2: Enhance Existing Reprocess Tests

Add to `e2e/document-reprocess.spec.ts`:

- Workspace isolation verification
- Error recovery scenarios

### Decision 3: Verify Frontend Status Updates

Ensure the status-badge.tsx changes from iteration 01/02 work with rebuild:

- Documents show "extracting" during rebuild
- Documents show "embedding" during re-embedding

---

## Changes for This Iteration

| #   | File                           | Change                              |
| --- | ------------------------------ | ----------------------------------- |
| 1   | e2e/rebuild-operations.spec.ts | Create comprehensive E2E test suite |
| 2   | e2e/document-reprocess.spec.ts | Add workspace isolation tests       |

---

## Acceptance Criteria

- [ ] Rebuild operations E2E tests exist
- [ ] Tests can run without Ollama (graceful skip)
- [ ] Tests verify workspace isolation
- [ ] Tests verify progress dialog appears

---

## Next Step

Proceed to **Act** phase to implement the E2E tests.
