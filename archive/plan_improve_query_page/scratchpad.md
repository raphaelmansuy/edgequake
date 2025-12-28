# Query Page Improvement - Research Scratchpad

## 2024-12-27 Initial Discovery

### EdgeQuake WebUI Current Implementation Analysis

**Technology Stack:**

- Next.js 14+ with App Router
- React 18+ with TypeScript
- TailwindCSS + shadcn/ui components
- Zustand for state management (with persist middleware)
- React Query (TanStack Query) for server state
- react-markdown + remark-gfm for markdown rendering
- Prism for syntax highlighting
- Mermaid for diagrams (dynamically loaded)

**Query Page Components (found at `src/components/query/`):**

1. `query-interface.tsx` - Main query orchestrator (859 lines)
2. `markdown-renderer.tsx` - Complex markdown rendering (814 lines)
3. `chat-message.tsx` - Individual message bubbles (489 lines)
4. `conversation-history-panel.tsx` - History sidebar (407 lines)
5. `code-block.tsx` - Code block component
6. `thinking-display.tsx` - Chain-of-thought reasoning display
7. `source-citations.tsx` - Citation display
8. `query-mode-selector.tsx` - Mode selection (Local/Global/Hybrid/Naive)

**Current Conversation Store (`use-conversation-store.ts`):**

- Uses Zustand with localStorage persistence
- Stores: `conversations[]`, `activeConversationId`, `historyPanelOpen`
- Message interface: id, role, content, mode, tokens, duration, context, isStreaming
- Conversation: id, title, messages[], createdAt, updatedAt, tenantId, workspaceId
- **Issue**: No server-side sync, all localStorage, loses data on clear

**Markdown Rendering Issues Identified:**

1. `enableMath = false` by default - temporarily disabled for debugging
2. During streaming: falls back to plain text for partial content
3. `normalizeMarkdown()` function tries to fix tokenized spacing issues
4. Mermaid diagrams are DISABLED during streaming mode
5. KaTeX loaded dynamically but disabled
6. Error boundary catches rendering failures but may hide issues

**Streaming Implementation:**

- Uses `queryStream()` API with async generator
- StreamingState: 'idle' | 'thinking' | 'generating' | 'complete' | 'error'
- Parses Chain-of-Thought content with `parseCOTContent()`
- Auto-scroll detection with threshold (100px from bottom)
- AbortController for cancellation

---

## 2024-12-27 Existing Audit Review

Found existing audit at `audit_ui/screens/query.md`:

- Overall Slickness Score: 4.4/5 (best screen in app)
- Minor issues identified:
  1. History panel search lacks clear indicator
  2. Mode selector buttons not grouped
  3. Suggested prompts lack hover animation
  4. History collapse button not obvious
  5. Input focus ring could be more prominent

Positive elements noted:

- Loading animation (brain icon with ping, shimmer)
- Message bubbles differentiation
- Collapsible thinking section
- Avatar design with gradient
- Empty state with upload CTA

---

## 2024-12-27 openwebui Competitive Analysis

### Key Findings from openwebui Repository

**Technology Stack:**

- SvelteKit framework (not React)
- marked.js library for markdown parsing (not react-markdown)
- Custom marked extensions for: KaTeX, citations, footnotes, mentions
- DOMPurify for sanitization
- Server-side SQLite/PostgreSQL with SQLAlchemy ORM

**Markdown Rendering Architecture:**

1. Uses `marked.lexer()` to tokenize markdown first
2. Custom `MarkdownTokens.svelte` component renders each token type
3. Separate `MarkdownInlineTokens.svelte` for inline content
4. `done` prop controls streaming behavior (false = still streaming)
5. Text token has fade animation during streaming
6. No fallback to plain text - renders partial tokens progressively

**Key Patterns to Adopt:**

1. **Token-based rendering**: Parse once, render tokens individually
2. **Streaming prop `done`**: Controls whether rendering is complete
3. **Custom extensions**: KaTeX, citations, mentions as marked extensions
4. **Progressive reveal**: Text fades in during streaming

**Database Schema (openwebui):**

```sql
-- Core Chat table
CREATE TABLE chat (
    id VARCHAR(255) PRIMARY KEY,
    user_id VARCHAR(255),
    title TEXT,
    chat JSON,  -- Contains history.messages as nested object
    created_at BIGINT,
    updated_at BIGINT,
    share_id TEXT UNIQUE,
    archived BOOLEAN DEFAULT FALSE,
    pinned BOOLEAN DEFAULT FALSE,
    meta JSON DEFAULT '{}',
    folder_id TEXT
);

-- Indexes for performance
CREATE INDEX folder_id_idx ON chat(folder_id);
CREATE INDEX user_id_pinned_idx ON chat(user_id, pinned);
CREATE INDEX user_id_archived_idx ON chat(user_id, archived);
CREATE INDEX updated_at_user_id_idx ON chat(updated_at, user_id);
```

**Chat JSON Structure:**

```json
{
  "history": {
    "currentId": "uuid-of-current-message",
    "messages": {
      "message-id-1": {
        "id": "message-id-1",
        "parentId": null,
        "childrenIds": ["message-id-2"],
        "role": "user",
        "content": "...",
        "model": "gpt-4",
        "timestamp": 1234567890
      }
    }
  },
  "models": ["gpt-4"],
  "timestamp": 1234567890,
  "title": "Chat Title"
}
```

**Key Differences from EdgeQuake:**
| Feature | openwebui | EdgeQuake |
|---------|-----------|-----------|
| Persistence | Server-side SQL | Client localStorage |
| Markdown | marked.js + custom extensions | react-markdown + remark-gfm |
| Streaming | Token-level with `done` prop | Fallback to plain text |
| Message tree | Parent-child relationships | Flat array |
| Multi-tenant | user_id column | tenantId + workspaceId |

**Patterns to AVOID from openwebui:**

1. Storing entire chat in JSON blob (limits queryability)
2. No separate messages table (can't index/search messages)
3. Limited pagination support in UI

**Patterns to ADOPT:**

1. Token-based markdown rendering
2. Server-side persistence with proper indexes
3. `done` prop for streaming state
4. Folder organization system
5. Chat tagging via meta JSON

---

## 2024-12-27 User Personas Draft

### Persona 1: Data Analyst "Dana"

- **Goal**: Quick answers from knowledge graph
- **Pain Points**: Loses query history, can't share insights
- **Workflow**: Multiple queries per session, needs filtering

### Persona 2: Knowledge Manager "Kim"

- **Goal**: Build and explore knowledge bases
- **Pain Points**: Markdown tables broken, can't see reasoning
- **Workflow**: Complex queries, exports results

### Persona 3: Developer "Dev"

- **Goal**: Test and debug RAG pipelines
- **Pain Points**: API responses not matching UI, streaming breaks
- **Workflow**: Technical queries, inspects full response

---

## 2024-12-27 Client-Side Implementation Specs Created

Created 4 new implementation specification documents:

### 07_client_markdown_pipeline.md (~350 lines)

- Token-based markdown architecture using marked.js
- Component hierarchy: StreamingMarkdownRenderer → MarkdownTokens → Individual tokens
- Per-token components: ParagraphToken, HeadingToken, CodeToken, TableToken, etc.
- Streaming support with `done` prop instead of fallback to plain text
- KaTeX and Mermaid extensions
- Migration steps from react-markdown

### 08_client_api_client.md (~400 lines)

- TypeScript type definitions for Conversation, Message, Folder
- API client functions using existing pattern from lib/api/client.ts
- React Query hooks: useConversations, useConversation, useCreateMessage
- Cursor-based pagination support
- Optimistic updates for messages
- localStorage import/migration endpoint

### 09_client_state_management.md (~380 lines)

- New `use-query-ui-store.ts` replacing `use-conversation-store.ts`
- Separation of concerns: UI state (Zustand) vs Server state (React Query)
- Streaming state machine: idle → thinking → generating → complete
- Filter and sort state persistence
- Selection state for batch operations
- Unified `useQueryPageState()` hook combining both stores
- Migration hook for localStorage → server sync

### 10_client_history_panel.md (~400 lines)

- Refactored component hierarchy with virtualization
- PanelHeader, SearchBar, FilterBar components
- Virtualized ConversationList using @tanstack/react-virtual
- ConversationItem with inline rename, pin, archive, delete
- SelectionToolbar for batch operations
- Loading and Empty states
- Infinite scroll with IntersectionObserver

---

## Implementation Priority

### Sprint 1 (Weeks 1-4): Markdown Pipeline

1. Install marked.js and configure extensions
2. Create StreamingMarkdownRenderer and token components
3. Migrate from react-markdown to token-based approach
4. Fix streaming behavior (no fallback to plain text)

### Sprint 2 (Weeks 5-8): Persistence

1. Run database migration (06_server_implementation.md)
2. Implement API handlers in Rust
3. Create API client and React Query hooks
4. Migrate from localStorage to server sync

### Sprint 3 (Weeks 9-12): UI Polish

1. Refactor history panel with virtualization
2. Add filtering and batch operations
3. Implement folders and organization
4. Add sharing and export features

---

## 2025-12-27 PostgreSQL Conversation Storage Implementation

### Implementation Summary

Completed the PostgreSQL backend for conversation persistence, enabling production-ready
storage with full multi-tenancy support.

### Files Created

#### 1. PostgresConversationStorage (edgequake-storage)

**File**: `crates/edgequake-storage/src/adapters/postgres/conversation.rs`

Low-level PostgreSQL storage implementation with:

- Full CRUD operations for conversations, messages, and folders
- RLS context management for multi-tenancy
- Bulk operations (delete, archive, move to folder)
- Share/unshare functionality
- Message count and preview helpers
- Proper error handling with StorageError

Data structures:

- `ConversationRow` - Database row representation
- `MessageRow` - Message database row
- `FolderRow` - Folder database row

#### 2. PostgresConversationService (edgequake-api)

**File**: `crates/edgequake-api/src/postgres_conversation_service.rs`

High-level service implementing `ConversationService` trait:

- Wraps `PostgresConversationStorage`
- Converts between storage rows and domain types
- Provides async trait implementation
- Available behind `postgres` feature flag

Usage:

```rust
#[cfg(feature = "postgres")]
use edgequake_api::PostgresConversationService;

let pool = PgPool::connect("postgres://...").await?;
let service: Arc<dyn ConversationService> = Arc::new(PostgresConversationService::new(pool));
```

### Integration Tests

**File**: `crates/edgequake-storage/tests/postgres_conversation_integration.rs`

Comprehensive test suite covering:

- Basic CRUD operations (create, get, update, delete)
- Non-existent resource handling
- Share/unshare workflow
- Message operations
- Folder operations
- Bulk operations (delete, archive, move)
- Full workflow tests with fixtures
- Filter and pagination tests

Run tests with:

```bash
POSTGRES_PASSWORD=your_password cargo test --package edgequake-storage \
  --test postgres_conversation_integration --features postgres
```

### Feature Flags

- `edgequake-storage/postgres` - Enables PostgreSQL storage adapters
- `edgequake-api/postgres` - Enables PostgresConversationService

### Cargo.toml Updates

**edgequake-storage:**

- Added `chrono` as optional dependency (with postgres feature)
- Added `uuid` to dependencies

**edgequake-api:**

- Added `postgres` feature flag
- Added `sqlx` as optional dependency

### Architecture Notes

The implementation follows the existing EdgeQuake patterns:

1. Low-level storage in `edgequake-storage` crate
2. High-level service in `edgequake-api` crate
3. Trait-based abstraction for swappable implementations
4. RLS context for multi-tenant isolation
5. Feature flags for optional PostgreSQL support

### Usage with AppState

To use PostgreSQL storage in production:

```rust
#[cfg(feature = "postgres")]
use edgequake_api::PostgresConversationService;

// In AppState construction:
let pool = PgPool::connect(&database_url).await?;
let conversation_service: SharedConversationService = Arc::new(
    PostgresConversationService::new(pool)
);
```

For development/testing, continue using `InMemoryConversationService`.

### Database Migration

The migration file `migrations/009_add_conversations_tables.sql` creates:

- `folders` table with hierarchical structure
- `conversations` table with RLS policies
- `messages` table with threading support
- Full-text search indexes
- Auto-update triggers for timestamps
- Proper foreign key constraints

---

## 2025-12-28 E2E Testing Session

### Test Environment

- **Backend**: Rust API server on port 8080 (edgequake release binary)
- **Frontend**: Next.js 16.1.0 with Turbopack on port 3000
- **Testing Tool**: Playwright MCP browser automation

### Issues Found and Fixed

#### Issue 1: 401 Unauthorized on Conversation APIs

**Symptom**: All conversation API calls returned 401 Unauthorized.

**Root Cause**: The Rust conversation handlers require `X-User-ID` header:

```rust
// handlers/conversations.rs
let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::Unauthorized)?;
```

**Fix Applied** (`src/lib/api/client.ts`):

```typescript
// Generate or retrieve persistent anonymous user ID
function getOrCreateUserId(): string {
  if (typeof window === "undefined") return "";
  const storageKey = "edgequake_user_id";
  let userId = localStorage.getItem(storageKey);
  if (!userId) {
    userId = crypto.randomUUID();
    localStorage.setItem(storageKey, userId);
  }
  return userId;
}

// In buildHeaders():
headers.set("X-User-ID", effectiveUserId);
```

#### Issue 2: Folders API Response Type Mismatch

**Symptom**: "Query data cannot be undefined" error in React Query.

**Root Cause**: Frontend expected `{ items: ConversationFolder[] }` but Rust API returns `Vec<FolderResponse>` directly.

**Fix Applied** (`src/lib/api/folders.ts`):

```typescript
// Before:
const response = await api.get<{ items: ConversationFolder[] }>("/folders");
return response.items;

// After:
const response = await api.get<ConversationFolder[]>("/folders");
return response ?? [];
```

### Verified Functionality

| Feature                 | Status  | Notes                                     |
| ----------------------- | ------- | ----------------------------------------- |
| Query page loads        | ✅ Pass | Empty state shows suggested prompts       |
| Conversation creation   | ✅ Pass | "New" button creates fresh conversation   |
| Message sending         | ✅ Pass | Submit sends message, shows user bubble   |
| Streaming response      | ✅ Pass | Assistant response appears with animation |
| History panel           | ✅ Pass | Lists all conversations, shows timestamps |
| Switching conversations | ✅ Pass | Clicking loads previous conversation      |
| Query mode selector     | ✅ Pass | Local/Global/Hybrid/Simple buttons work   |
| Markdown rendering      | ✅ Pass | Uses StreamingMarkdownRenderer            |
| Empty state             | ✅ Pass | Shows suggested prompts with icons        |

### Screenshots Captured

- `query-working.png` - Initial working state
- `new-conversation.png` - Empty state with prompts
- `message-sent.png` - After sending query
- `history-panel-test.png` - After switching conversations
- `markdown-test.png` - Message bubbles with formatting

### Token-Based Markdown Renderer

Verified that the markdown rendering uses the proper token-based approach:

```typescript
// StreamingMarkdownRenderer.tsx
const tokens = useMemo(() => {
  return tokenizeMarkdown(content); // Uses marked.lexer()
}, [content]);

// Renders tokens via MarkdownTokens component
return <MarkdownTokens tokens={tokens} isStreaming={isStreaming} />;
```

Components available:

- `CodeBlock.tsx` - Syntax-highlighted code blocks
- `KatexMath.tsx` - Math equation rendering (disabled by default)
- `MermaidBlock.tsx` - Diagram rendering
- `MarkdownTokens.tsx` - Token type dispatcher
- `MarkdownInlineTokens.tsx` - Inline token handling

### Recommendations for Future Testing

1. **Add E2E test suite**: Create Playwright tests in `e2e/` folder
2. **Test with real LLM**: Verify streaming with actual OpenAI responses
3. **Test markdown edge cases**: Code blocks, tables, math, nested lists
4. **Test conversation persistence**: Refresh page, verify data loads
5. **Test multi-user**: Different user IDs should see different conversations
