# Source Citations UX/UI Audit

**Date:** 2025-12-31  
**Status:** 🔴 CRITICAL ISSUES IDENTIFIED  
**Priority:** HIGH - User Trust Impact

---

## Executive Summary

The Source Citations component has **critical UX issues** that undermine user trust and provide meaningless information. Based on screenshot analysis and code audit, the following problems were identified:

| Issue                         | Severity    | User Impact                            |
| ----------------------------- | ----------- | -------------------------------------- |
| Scary red/orange score colors | 🔴 Critical | Users think responses are unreliable   |
| 0% relationship scores        | 🔴 Critical | Completely meaningless, destroys trust |
| UUID-based document titles    | 🟠 High     | Users can't identify documents         |
| No chunk line numbers         | 🟡 Medium   | Users can't navigate to source         |
| Arbitrary score thresholds    | 🟠 High     | 4% labeled "Low" when RAG normal       |

---

## Issue 1: Scary Score Colors 🔴

### Current State

**File:** [source-citations.tsx](../edgequake_webui/src/components/query/source-citations.tsx#L53-L57)

```typescript
const getConfidenceLabel = (
  score: number
): { label: string; color: string; bgColor: string } => {
  if (score >= 0.8)
    return {
      label: "High",
      color: "text-emerald-600...",
      bgColor: "bg-emerald-500",
    };
  if (score >= 0.6)
    return {
      label: "Good",
      color: "text-green-600...",
      bgColor: "bg-green-500",
    };
  if (score >= 0.4)
    return {
      label: "Medium",
      color: "text-amber-600...",
      bgColor: "bg-amber-500",
    }; // ⚠️ ORANGE
  return { label: "Low", color: "text-red-600...", bgColor: "bg-red-500" }; // 🔴 RED = SCARY!
};
```

### Problem Analysis

1. **RED = Danger in Universal UI Convention**

   - Users interpret red as error, failure, or warning
   - A "Low (4%)" confidence in red makes users distrust the answer
   - Even correct answers appear unreliable

2. **Thresholds Don't Match RAG Reality**

   - Vector similarity scores of 0.3-0.5 are NORMAL for RAG
   - A score of 0.35 (35%) is often a good match
   - Current thresholds treat 35% as "Low" with scary red

3. **SOTA Comparison**
   - Perplexity AI: No visible confidence scores
   - ChatGPT: No percentage scores on sources
   - Notion AI: Simple "Sources" with no percentages
   - **Best practice: Don't show raw scores to users**

### Solution

**Option A: Remove Percentage Display Entirely (Recommended)**

```typescript
// Just show "Sources" with count, no scary percentages
const getConfidenceLabel = (score: number) => {
  // Use only for internal logic, not display
  return score >= 0.5 ? "primary" : "secondary";
};
```

**Option B: Use Neutral Colors with Qualitative Labels**

```typescript
const getConfidenceLabel = (score: number) => {
  if (score >= 0.5)
    return { label: "Primary", color: "text-blue-600", bgColor: "bg-blue-500" };
  if (score >= 0.3)
    return {
      label: "Supporting",
      color: "text-slate-600",
      bgColor: "bg-slate-500",
    };
  return { label: "Related", color: "text-slate-400", bgColor: "bg-slate-400" };
};
```

---

## Issue 2: 0% Relationship Scores 🔴

### Current State

**File:** [source-citations.tsx](../edgequake_webui/src/components/query/source-citations.tsx#L326-L328)

```tsx
<span className="ml-auto text-[10px] text-muted-foreground">
  {Math.round(rel.relevance * 100)}%
</span>
```

### Screenshot Evidence

```
Agentless → comparison → RepoNavigator  0%
LocAgent → comparison → RepoNavigator   0%
CoSIL → related to → RepoSearcher       0%
CoSIL → created by jiang et → Jiang et al.  0%
```

**ALL relationships show 0%!** This is either:

1. Backend bug: Scores aren't being calculated
2. Design flaw: Relationships don't have similarity scores

### Root Cause Investigation

**File:** [sota_engine.rs](../edgequake/crates/edgequake-query/src/sota_engine.rs)

```rust
// RetrievedRelationship has score field, but...
// In query_local/global, relationships are fetched from graph
// Graph edges don't have vector similarity scores!
```

**Finding:** Relationships are from graph traversal, not vector search. They don't have meaningful similarity scores.

### Solution

**Option A: Hide Score for Relationships**

```tsx
{
  /* Only show score for chunks, not relationships */
}
{
  rel.relevance > 0 && (
    <span className="ml-auto text-[10px] text-muted-foreground">
      {Math.round(rel.relevance * 100)}%
    </span>
  );
}
```

**Option B: Remove Score Column Entirely for Relationships**

```tsx
<div className="flex items-center gap-1.5 text-xs p-2 rounded-md">
  <span className="font-medium">{rel.source}</span>
  <span className="text-primary/60">→</span>
  <Badge variant="outline" className="text-[10px]">
    {rel.type.toLowerCase().replace(/_/g, " ")}
  </Badge>
  <span className="text-primary/60">→</span>
  <span className="font-medium">{rel.target}</span>
  {/* NO SCORE - it's meaningless for graph relationships */}
</div>
```

---

## Issue 3: UUID-Based Document Titles 🟠

### Current State

**File:** [source-citations.tsx](../edgequake_webui/src/components/query/source-citations.tsx#L126-L131)

```tsx
{
  chunks[0]?.file_path
    ? chunks[0].file_path.split("/").pop()
    : `Document ${docId.slice(0, 8)}`;
}
```

### Screenshot Evidence

- Shows "Document f0291a69" instead of meaningful title
- UUID in subtitle: "f0291a69-8b63-46..." (redundant)
- Content preview starts with actual title: "# EdgeQuake Research Document"

### Solution

**Title Extraction Priority:**

1. Check `file_path` for filename
2. Extract first `#` heading from content
3. Fallback to "Untitled Document"

```typescript
const getDocumentTitle = (chunks: Chunk[]): string => {
  const chunk = chunks[0];
  if (!chunk) return "Untitled";

  // Priority 1: file_path filename
  if (chunk.file_path) {
    const filename = chunk.file_path.split("/").pop() || "";
    // Remove extension for cleaner display
    return filename.replace(/\.(md|txt|pdf|docx?)$/i, "") || filename;
  }

  // Priority 2: Extract markdown title from content
  const titleMatch = chunk.content.match(/^#\s+(.+)$/m);
  if (titleMatch) {
    return titleMatch[1].trim().slice(0, 60);
  }

  // Priority 3: First line truncated
  const firstLine = chunk.content.split("\n")[0]?.trim();
  if (firstLine && firstLine.length > 0) {
    return firstLine.slice(0, 60) + (firstLine.length > 60 ? "..." : "");
  }

  return "Untitled Document";
};
```

**Backend Enhancement (Optional):**

Add `title` field to `SourceReference`:

```rust
// query.rs
pub struct SourceReference {
    // ... existing fields ...

    /// Document title for display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}
```

---

## Issue 4: No Chunk Line Numbers 🟡

### Current State

- Chunks are grouped by document (good)
- Only first chunk content is shown
- No line number information
- Can't navigate to specific location

### Backend Data Available

**File:** [lineage.rs](../edgequake/crates/edgequake-pipeline/src/lineage.rs#L94-L105)

```rust
pub struct ChunkLineage {
    pub chunk_id: String,
    pub chunk_index: usize,
    pub start_line: usize,    // ✅ Available!
    pub end_line: usize,      // ✅ Available!
    pub start_offset: usize,
    pub end_offset: usize,
}
```

### Frontend Missing Fields

**File:** [types/index.ts](../edgequake_webui/src/types/index.ts#L246-L251)

```typescript
// Current
chunks: Array<{
  content: string;
  document_id: string;
  score: number;
  file_path?: string;
  // MISSING: start_line, end_line
}>;
```

### Solution

**Step 1: Add to SourceReference (Backend)**

```rust
pub struct SourceReference {
    // ... existing ...
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub chunk_index: Option<usize>,
}
```

**Step 2: Add to Frontend Types**

```typescript
chunks: Array<{
  content: string;
  document_id: string;
  score: number;
  file_path?: string;
  start_line?: number;
  end_line?: number;
  chunk_index?: number;
}>;
```

**Step 3: Display in UI**

```tsx
<div className="flex items-center gap-2">
  <FileText className="h-3.5 w-3.5" />
  <span className="font-medium">{title}</span>
  {chunk.start_line && (
    <Badge variant="outline" className="text-[9px]">
      Lines {chunk.start_line}-{chunk.end_line}
    </Badge>
  )}
</div>
```

**Step 4: Deep Link with Line Numbers**

```tsx
const handleChunkClick = (chunk: Chunk) => {
  const url = `/documents/${chunk.document_id}?start=${chunk.start_line}&end=${
    chunk.end_line
  }&highlight=${encodeURIComponent(chunk.content.slice(0, 50))}`;
  window.open(url, "_blank");
};
```

---

## Implementation Plan

### Phase 1: Quick Wins (1 hour)

| Task                                | File                 | Change                        |
| ----------------------------------- | -------------------- | ----------------------------- |
| Fix score colors                    | source-citations.tsx | Use neutral blue/gray palette |
| Hide 0% relationship scores         | source-citations.tsx | Conditional render            |
| Extract document title from content | source-citations.tsx | Add getDocumentTitle helper   |

### Phase 2: Backend Enhancement (1.5 hours)

| Task                         | File           | Change                 |
| ---------------------------- | -------------- | ---------------------- |
| Add title to SourceReference | query.rs       | New field + population |
| Add start_line/end_line      | query.rs       | New fields             |
| Populate from ChunkLineage   | sota_engine.rs | Flow lineage data      |

### Phase 3: Frontend Polish (1 hour)

| Task                      | File                 | Change           |
| ------------------------- | -------------------- | ---------------- |
| Update frontend types     | types/index.ts       | Add new fields   |
| Display line numbers      | source-citations.tsx | Badge with lines |
| Deep link with parameters | source-citations.tsx | Click handler    |
| Update source-mapper      | source-mapper.ts     | Map new fields   |

### Phase 4: Testing (30 mins)

| Task               | Tool       | Validation         |
| ------------------ | ---------- | ------------------ |
| Visual regression  | Playwright | Screenshots        |
| API response check | curl       | New fields present |
| Link navigation    | Browser    | Works end-to-end   |

---

## Success Metrics

1. **No scary colors** - All score indicators use neutral palette
2. **No 0% scores visible** - Hidden or replaced with qualitative labels
3. **Document titles readable** - Show actual filename or extracted title
4. **Line numbers present** - Each chunk shows line range
5. **Deep links work** - Clicking navigates to correct location

---

## Files to Modify

### Frontend

- `source-citations.tsx` - Main component overhaul
- `types/index.ts` - Add new type fields
- `source-mapper.ts` - Map new API fields

### Backend

- `query.rs` - SourceReference enhancements
- `chat.rs` - build_sources update
- `sota_engine.rs` - Lineage data flow

---

## Risk Assessment

| Risk                        | Likelihood | Mitigation                 |
| --------------------------- | ---------- | -------------------------- |
| Backend changes break API   | Low        | Add fields as optional     |
| Line numbers not in storage | Medium     | Check chunk storage schema |
| Title extraction fails      | Low        | Multiple fallback layers   |

---

**Signed:** Claude (UX Audit Mode)  
**Mode:** BEAST MODE 🔥
