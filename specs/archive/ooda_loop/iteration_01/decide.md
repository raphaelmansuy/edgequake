# Iteration 01: Decide

**Mission Re-read**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

---

## Prioritized Action Plan

### Iteration 01 Focus: Critical Bug Fix + Foundation

Based on First Principles analysis, the highest signal-value changes are:

---

### Decision 1: Fix Loader2 Import ✅ DONE

**Status**: Already completed in this session
**Location**: document-manager.tsx:62
**Change**: Added `Loader2` to lucide-react imports

---

### Decision 2: Enhance Status Badge with Sub-States

**Why**: Users need to see WHAT is happening, not just "processing"

**Action**:

1. Add new processing sub-states to `status-badge.tsx`
2. Keep backward compatibility with existing 'processing' state
3. Add appropriate icons and colors for each stage

**Implementation**:

```typescript
// New states to add
'chunking'   → Scissors icon, blue
'extracting' → Brain/Sparkles icon, purple
'embedding'  → Cpu icon, cyan
'indexing'   → Database icon, green
```

---

### Decision 3: Display Error Messages in Document Row

**Why**: Users can't fix what they can't see

**Action**:

1. Add error message display when status is 'failed'
2. Show in document table row with expandable details
3. Add copy-to-clipboard for debugging

---

### Decision 4: Create Initial E2E Test Structure

**Why**: Tests must use Ollama to be realistic

**Action**:

1. Create `edgequake_webui/e2e/documents/reprocess.spec.ts`
2. Add Ollama model configuration
3. Test basic reprocess flow

---

## Changes for This Iteration

| #   | File                            | Change                    | Commit Message                     |
| --- | ------------------------------- | ------------------------- | ---------------------------------- |
| 1   | document-manager.tsx            | ✅ Add Loader2 import     | OODA-01: Fix Loader2 import        |
| 2   | status-badge.tsx                | Add processing sub-states | OODA-01: Add processing sub-states |
| 3   | document-manager.tsx            | Show error in row         | OODA-01: Display error messages    |
| 4   | e2e/documents/reprocess.spec.ts | Create test file          | OODA-01: Add reprocess E2E test    |

---

## Out of Scope for This Iteration

- Rebuild embeddings endpoint (needs backend work)
- WebSocket real-time updates (future enhancement)
- ETA calculation (needs baseline metrics)
- Ollama integration verification (need running server)

---

## Acceptance Criteria

- [ ] No Loader2 runtime error ✅
- [ ] Status badge shows 9 distinct states
- [ ] Failed documents show error message
- [ ] E2E test file exists with basic structure

---

## Next Step

Proceed to **Act** phase to implement changes.
