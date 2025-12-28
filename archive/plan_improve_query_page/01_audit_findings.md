# Phase 1: Audit Findings

**Document**: `01_audit_findings.md`  
**Created**: 2024-12-27  
**Status**: Complete

---

## 1. Executive Summary

The Query Page is EdgeQuake WebUI's flagship feature, achieving a **4.4/5 slickness score** in preliminary audits. However, critical technical debt threatens user experience:

| Problem                                    | Impact                          | Severity    |
| ------------------------------------------ | ------------------------------- | ----------- |
| Markdown rendering breaks during streaming | Users see garbled output        | 🔴 Critical |
| No cross-session persistence               | Context lost on logout          | 🔴 Critical |
| localStorage-only storage                  | Can't sync across devices       | 🟡 High     |
| No pagination/filtering                    | History panel unusable at scale | 🟡 High     |

**Recommendation**: Prioritize token-based markdown rendering and server-side persistence, adopting patterns from openwebui while avoiding their JSON-blob storage anti-pattern.

---

## 2. User Personas

### 2.1 Data Analyst "Dana"

- **Role**: Business analyst querying knowledge graphs daily
- **Goals**: Quick answers, save useful queries, share insights with team
- **Frustrations**:
  - Loses query history when clearing browser data
  - Can't filter history by topic or date
  - Markdown tables render incorrectly
- **Session pattern**: 5-15 queries/session, 2-3 sessions/day

### 2.2 Knowledge Manager "Kim"

- **Role**: Curates organizational knowledge bases
- **Goals**: Build comprehensive KGs, explore relationships, export insights
- **Frustrations**:
  - Chain-of-thought reasoning hard to read
  - Can't see query context after page refresh
  - Complex markdown (code blocks, mermaid) breaks mid-stream
- **Session pattern**: Long sessions (1-2 hours), deep exploration

### 2.3 Developer "Dev"

- **Role**: Engineers building on EdgeQuake APIs
- **Goals**: Debug RAG pipelines, test query modes, validate responses
- **Frustrations**:
  - Streaming output doesn't match API response
  - Can't replay exact queries for debugging
  - No way to export conversation for issue reports
- **Session pattern**: Burst usage during development cycles

---

## 3. User Journey Map

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        QUERY PAGE USER JOURNEY                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  STAGES:    Discover    │    Explore    │    Analyze    │    Return         │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ACTIONS:   • Open query page         • Review history      • Find old query│
│             • Type question           • Refine query        • Resume session │
│             • Select mode             • View reasoning      • Export results │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  THOUGHTS:  "What entities     "This response    "I need to     "Where did  │
│              are connected?"    is complex..."    dig deeper"     it go?!"   │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  EMOTIONS:  😊 Excited         😕 Confused       😤 Frustrated   😢 Lost    │
│             (starting)         (broken md)       (no filter)     (no persist)│
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  PAIN       ──────○──────      ────────●────     ────────●───    ────●────  │
│  INTENSITY:      Low                 HIGH             HIGH          HIGH     │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  OPPS:      [✓] Good empty     [!] Fix streaming [!] Add filter  [!] Persist│
│              state UI           markdown render   & pagination    to server  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Current Component Architecture

### 4.1 Component Hierarchy

```
query/page.tsx
└── QueryInterface (859 lines)
    ├── Header
    │   ├── QueryModeSelector
    │   ├── New Conversation Button
    │   └── Settings Sheet
    │       └── Stream/Temperature/TopK controls
    ├── Messages Area (ScrollArea)
    │   ├── EmptyState (when no messages)
    │   │   └── Suggestion Cards
    │   └── ChatMessage[] (per message)
    │       ├── UserMessage
    │       └── AssistantMessage
    │           ├── ThinkingSection (collapsible COT)
    │           ├── MarkdownRenderer
    │           │   ├── MermaidDiagram
    │           │   ├── CodeBlock
    │           │   └── KatexRenderer (disabled)
    │           └── SourceCitations
    ├── Input Form
    │   ├── Textarea (auto-resize)
    │   └── Send/Stop Button
    └── ConversationHistoryPanel
        ├── Search Input
        └── ConversationItem[]
```

### 4.2 State Management

| Store                  | Purpose                     | Persistence  |
| ---------------------- | --------------------------- | ------------ |
| `useConversationStore` | Conversations & messages    | localStorage |
| `useSettingsStore`     | Query settings (mode, temp) | localStorage |
| `useTenantStore`       | Tenant/workspace context    | Session      |

**Issue**: All state in localStorage means:

- Lost on clear browser data
- Can't sync across devices/browsers
- Can't implement server-side search

### 4.3 Dependencies

```json
{
  "react-markdown": "^9.x",
  "remark-gfm": "^4.x",
  "remark-math": "^6.x",
  "rehype-katex": "^7.x",
  "react-syntax-highlighter": "^15.x",
  "mermaid": "^10.x",
  "zustand": "^4.x",
  "@tanstack/react-query": "^5.x"
}
```

---

## 5. Markdown Rendering Deep Dive

### 5.1 Current Implementation Issues

#### Issue 1: Streaming Fallback to Plain Text

```tsx
// markdown-renderer.tsx (lines 720-730)
if (isStreaming) {
  const hasUnclosedBold = (safeContent.match(/\*\*/g) || []).length % 2 !== 0;
  // ... more checks
  if (safeContent.length < 50 || hasUnclosedBold || ...) {
    return fallback; // ❌ Renders as plain text!
  }
}
```

**Problem**: During streaming, if markdown is incomplete, entire content falls back to `<p>` tag, losing all formatting.

#### Issue 2: Disabled KaTeX

```tsx
// markdown-renderer.tsx (line 269)
enableMath = false,  // Temporarily disable math to debug
```

**Problem**: Math equations don't render, but this is a conscious debugging decision.

#### Issue 3: Mermaid Disabled During Streaming

```tsx
// markdown-renderer.tsx (lines 322-325)
if (
  enableMermaidRef.current &&
  language === "mermaid" &&
  !isStreamingRef.current
) {
  return <MermaidDiagram code={codeContent} />;
}
```

**Problem**: Mermaid diagrams only render after streaming completes.

#### Issue 4: Aggressive Token Normalization

```tsx
// markdown-renderer.tsx (lines 644-720)
const normalizeMarkdown = (text: string): string => {
  // 60+ regex replacements to fix spacing
  result = result.replace(/\s+\.\s+(?!\.)/g, ". ");
  // ...
};
```

**Problem**: Heavy post-processing indicates underlying tokenization issues from LLM.

### 5.2 openwebui Comparison

| Aspect            | EdgeQuake                    | openwebui                       |
| ----------------- | ---------------------------- | ------------------------------- |
| Parser            | react-markdown (render pass) | marked.lexer() (tokenize first) |
| Streaming         | Fallback to plain text       | Progressive token rendering     |
| Components        | Single MarkdownRenderer      | MarkdownTokens + InlineTokens   |
| Streaming prop    | `isStreaming` (boolean)      | `done` (inverted logic)         |
| Error handling    | ErrorBoundary + fallback     | Per-token error handling        |
| Custom extensions | None built-in                | KaTeX, citations, mentions      |

### 5.3 Recommended Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     NEW MARKDOWN PIPELINE                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   LLM Stream  ──▶  Buffer  ──▶  Lexer  ──▶  Token[]             │
│                    (chunk)      (marked)     (parsed)            │
│                                                                  │
│   Token[]  ──▶  TokenRenderer  ──▶  DOM                         │
│                 (per-type)         (progressive)                 │
│                                                                  │
│   Types: paragraph | heading | code | table | list | ...        │
│                                                                  │
│   Each token type has its own renderer component                 │
│   Incomplete tokens show skeleton/placeholder                    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. Persistence Architecture Analysis

### 6.1 Current State

```tsx
// use-conversation-store.ts
export const useConversationStore = create<ConversationStore>()(
  persist(
    (set, get) => ({
      /* ... */
    }),
    {
      name: "edgequake-conversations",
      partialize: (state) => ({
        conversations: state.conversations,
        activeConversationId: state.activeConversationId,
        historyPanelOpen: state.historyPanelOpen,
      }),
    }
  )
);
```

**Limitations**:

- 5MB localStorage limit (can't store many conversations)
- No server sync
- Can't share conversations
- Can't search across all conversations

### 6.2 openwebui Schema (Reference)

```sql
CREATE TABLE chat (
    id VARCHAR(255) PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    title TEXT,
    chat JSON,  -- Full history in JSON blob
    created_at BIGINT,
    updated_at BIGINT,
    share_id TEXT UNIQUE,
    archived BOOLEAN DEFAULT FALSE,
    pinned BOOLEAN DEFAULT FALSE,
    meta JSON DEFAULT '{}',
    folder_id TEXT REFERENCES folder(id)
);

CREATE INDEX idx_chat_user_updated ON chat(user_id, updated_at DESC);
CREATE INDEX idx_chat_folder ON chat(folder_id) WHERE folder_id IS NOT NULL;
```

**Their approach**: Store everything in JSON blob. Simple but limited.

### 6.3 Recommended EdgeQuake Schema

See [Technical Spec > 3.1](03_technical_spec.md#31-database-schema-design) for full schema with normalized messages table.

---

## 7. Competitive Matrix: EdgeQuake vs openwebui

| Feature               | EdgeQuake                           | openwebui          | Winner    |
| --------------------- | ----------------------------------- | ------------------ | --------- |
| **UI Framework**      | React/Next.js                       | SvelteKit          | Tie       |
| **Markdown Parser**   | react-markdown                      | marked.js          | openwebui |
| **Streaming UX**      | Fallback to text                    | Progressive tokens | openwebui |
| **Persistence**       | localStorage                        | Server-side SQL    | openwebui |
| **Multi-tenant**      | tenant + workspace                  | user_id only       | EdgeQuake |
| **Loading Animation** | Excellent shimmer                   | Basic spinner      | EdgeQuake |
| **Empty State**       | Beautiful + suggestions             | Plain              | EdgeQuake |
| **Mode Selection**    | 4 modes (Local/Global/Hybrid/Naive) | Model selector     | EdgeQuake |
| **COT Display**       | Collapsible thinking                | Inline             | EdgeQuake |
| **Message Tree**      | Flat array                          | Parent-child tree  | openwebui |
| **Search**            | Client-side filter                  | Server-side        | openwebui |
| **Sharing**           | None                                | share_id system    | openwebui |

**Summary**: EdgeQuake excels in UX polish and RAG-specific features (modes, COT). openwebui leads in infrastructure (persistence, streaming).

---

## 8. Key Recommendations

### 8.1 Critical (P0)

1. **Refactor Markdown Renderer**

   - Adopt token-based architecture from openwebui
   - Use marked.js lexer for tokenization
   - Create per-token renderers (ParagraphToken, CodeToken, etc.)
   - Implement progressive streaming without fallback

2. **Implement Server-Side Persistence**
   - Create normalized database schema (sessions, queries, messages)
   - Build REST API endpoints for CRUD operations
   - Migrate from localStorage to server sync
   - Maintain localStorage as offline cache

### 8.2 High Priority (P1)

3. **Add Pagination & Filtering**

   - Implement cursor-based pagination for history
   - Add date range, mode, and text search filters
   - Virtualize conversation list for performance

4. **Enable Math Rendering**
   - Re-enable KaTeX for math equations
   - Handle streaming edge cases for incomplete equations

### 8.3 Medium Priority (P2)

5. **Implement Conversation Sharing**

   - Add share_id generation
   - Create public view for shared conversations
   - Add export as PDF/Markdown

6. **Add Folder Organization**
   - Implement folder tree structure
   - Allow drag-and-drop organization

---

## 9. Technical Debt Summary

| Debt Item                   | Location                       | Effort | Risk if Ignored          |
| --------------------------- | ------------------------------ | ------ | ------------------------ |
| Streaming markdown fallback | markdown-renderer.tsx:720      | L      | High - broken UX         |
| Disabled KaTeX              | markdown-renderer.tsx:269      | S      | Medium - missing feature |
| 60+ regex normalizations    | markdown-renderer.tsx:644      | M      | Medium - fragile         |
| localStorage-only state     | use-conversation-store.ts      | L      | High - data loss         |
| No pagination               | conversation-history-panel.tsx | M      | High - perf issues       |
| Hardcoded suggestions       | query-interface.tsx:152        | S      | Low - inflexible         |

---

## 10. Next Steps

1. **Phase 2**: Define design principles and information architecture → [02_design_strategy.md](02_design_strategy.md)
2. **Phase 3**: Create technical specifications for schema, API, and architecture → [03_technical_spec.md](03_technical_spec.md)
3. **Phase 4**: Build prioritized implementation roadmap → [04_implementation_roadmap.md](04_implementation_roadmap.md)

---

_Last updated: 2024-12-27_
