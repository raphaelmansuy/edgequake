# Phase 3: Technical Specification - Query Page UX/UI Improvement

> **Date**: December 27, 2025  
> **Dependencies**: [Audit Findings](./01_audit_findings.md), [Design Strategy](./02_design_strategy.md)  
> **Objective**: Implementation blueprint for developers

---

## 1. Database Schema Design

### 1.1 Enhanced Schema Overview

The current schema is solid. We propose minimal additions for advanced features.

```sql
-- ============================================================================
-- SCHEMA ENHANCEMENT: Message Versioning
-- Purpose: Track message edits and regenerations
-- ============================================================================

CREATE TABLE IF NOT EXISTS message_versions (
    version_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES messages(message_id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL DEFAULT 1,
    content TEXT NOT NULL,
    reason VARCHAR(50) NOT NULL DEFAULT 'edit',  -- 'edit', 'regenerate', 'retry'
    created_by UUID REFERENCES users(user_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_message_version UNIQUE(message_id, version_number),
    CONSTRAINT valid_reason CHECK (reason IN ('edit', 'regenerate', 'retry', 'initial'))
);

CREATE INDEX IF NOT EXISTS idx_message_versions_message ON message_versions(message_id, version_number);

-- ============================================================================
-- SCHEMA ENHANCEMENT: Conversation Tags
-- Purpose: User-defined labels for organization
-- ============================================================================

CREATE TABLE IF NOT EXISTS conversation_tags (
    tag_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    name VARCHAR(50) NOT NULL,
    color VARCHAR(7) NOT NULL DEFAULT '#6366f1',  -- Hex color
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_tag_name UNIQUE(tenant_id, user_id, name)
);

CREATE TABLE IF NOT EXISTS conversation_tag_assignments (
    conversation_id UUID NOT NULL REFERENCES conversations(conversation_id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES conversation_tags(tag_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (conversation_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_tag_assignments_tag ON conversation_tag_assignments(tag_id);

-- ============================================================================
-- PERFORMANCE: Materialized Message Count
-- Purpose: Avoid COUNT(*) subqueries on conversation list
-- ============================================================================

-- Add column to conversations table
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS message_count INTEGER DEFAULT 0;
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS last_message_at TIMESTAMPTZ;
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS last_message_preview TEXT;

-- Trigger to maintain counts
CREATE OR REPLACE FUNCTION update_conversation_message_stats()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE conversations
        SET message_count = message_count + 1,
            last_message_at = NEW.created_at,
            last_message_preview = LEFT(NEW.content, 100)
        WHERE conversation_id = NEW.conversation_id;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE conversations
        SET message_count = GREATEST(0, message_count - 1)
        WHERE conversation_id = OLD.conversation_id;
        -- Recalculate last_message_preview on delete
        UPDATE conversations c
        SET last_message_preview = (
            SELECT LEFT(content, 100)
            FROM messages
            WHERE conversation_id = c.conversation_id
            ORDER BY created_at DESC LIMIT 1
        ),
        last_message_at = (
            SELECT created_at
            FROM messages
            WHERE conversation_id = c.conversation_id
            ORDER BY created_at DESC LIMIT 1
        )
        WHERE conversation_id = OLD.conversation_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_message_stats ON messages;
CREATE TRIGGER trigger_message_stats
    AFTER INSERT OR DELETE ON messages
    FOR EACH ROW
    EXECUTE FUNCTION update_conversation_message_stats();
```

### 1.2 Index Strategy

```sql
-- ============================================================================
-- EXPLAIN ANALYZE SCENARIOS
-- ============================================================================

-- Scenario 1: List conversations with filters
-- Query pattern: GET /conversations?filter[mode]=hybrid&sort=updated_at
EXPLAIN ANALYZE
SELECT c.*,
       c.message_count,
       c.last_message_preview
FROM conversations c
WHERE c.tenant_id = '...'
  AND c.user_id = '...'
  AND c.mode = 'hybrid'
  AND c.is_archived = false
ORDER BY c.updated_at DESC
LIMIT 20;

-- Recommended compound index:
CREATE INDEX IF NOT EXISTS idx_conversations_list_optimized
    ON conversations(tenant_id, user_id, is_archived, mode, updated_at DESC)
    INCLUDE (title, is_pinned, folder_id, message_count, last_message_preview);

-- Scenario 2: Full-text search in conversations
-- Query pattern: GET /conversations?filter[search]=entity+graph
EXPLAIN ANALYZE
SELECT c.*
FROM conversations c
WHERE c.tenant_id = '...'
  AND c.user_id = '...'
  AND to_tsvector('english', c.title) @@ plainto_tsquery('english', 'entity graph')
ORDER BY c.updated_at DESC
LIMIT 20;

-- Already have GIN index on title, add combined search:
CREATE INDEX IF NOT EXISTS idx_conversations_search_combined
    ON conversations USING GIN (
        (to_tsvector('english', title) || to_tsvector('english', COALESCE(last_message_preview, '')))
    );

-- Scenario 3: Load messages for a conversation (paginated)
-- Query pattern: GET /conversations/{id}/messages?cursor={id}&limit=50
EXPLAIN ANALYZE
SELECT m.*
FROM messages m
WHERE m.conversation_id = '...'
  AND m.created_at < '...'  -- cursor position
ORDER BY m.created_at DESC
LIMIT 50;

-- Already have idx_messages_conversation, which covers this pattern
```

### 1.3 Migration Plan

```sql
-- Migration: 010_enhance_conversations_for_ux.sql
-- This is a NON-BREAKING migration (all changes are additive)

-- Step 1: Add new columns (nullable first)
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS message_count INTEGER;
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS last_message_at TIMESTAMPTZ;
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS last_message_preview TEXT;

-- Step 2: Backfill existing data
UPDATE conversations c
SET
    message_count = (SELECT COUNT(*) FROM messages WHERE conversation_id = c.conversation_id),
    last_message_at = (SELECT MAX(created_at) FROM messages WHERE conversation_id = c.conversation_id),
    last_message_preview = (
        SELECT LEFT(content, 100)
        FROM messages
        WHERE conversation_id = c.conversation_id
        ORDER BY created_at DESC LIMIT 1
    );

-- Step 3: Set defaults and constraints
ALTER TABLE conversations ALTER COLUMN message_count SET DEFAULT 0;
ALTER TABLE conversations ALTER COLUMN message_count SET NOT NULL;

-- Step 4: Create triggers (from section 1.1)
-- Step 5: Create new indexes (from section 1.2)
-- Step 6: Create new tables (message_versions, tags) - optional for phase 2
```

---

## 2. API Specification

### 2.1 RESTful Endpoints

#### Conversations API

```yaml
openapi: 3.0.3
info:
  title: EdgeQuake Conversations API
  version: 2.0.0

paths:
  /conversations:
    get:
      summary: List conversations with pagination and filtering
      parameters:
        - name: cursor
          in: query
          schema:
            type: string
          description: Cursor for pagination (conversation ID)
        - name: limit
          in: query
          schema:
            type: integer
            default: 20
            maximum: 100
        - name: filter[mode]
          in: query
          schema:
            type: array
            items:
              type: string
              enum: [local, global, hybrid, naive]
          style: form
          explode: true
        - name: filter[archived]
          in: query
          schema:
            type: boolean
            default: false
        - name: filter[pinned]
          in: query
          schema:
            type: boolean
        - name: filter[folder_id]
          in: query
          schema:
            type: string
            format: uuid
        - name: filter[search]
          in: query
          schema:
            type: string
          description: Full-text search in title and last message
        - name: filter[date_from]
          in: query
          schema:
            type: string
            format: date
        - name: filter[date_to]
          in: query
          schema:
            type: string
            format: date
        - name: sort
          in: query
          schema:
            type: string
            enum: [updated_at, created_at, title]
            default: updated_at
        - name: order
          in: query
          schema:
            type: string
            enum: [asc, desc]
            default: desc
      responses:
        "200":
          description: Paginated list of conversations
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/PaginatedConversations"

  /conversations/{id}:
    get:
      summary: Get conversation with messages
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
            format: uuid
        - name: message_limit
          in: query
          schema:
            type: integer
            default: 50
            maximum: 200
          description: Number of recent messages to include
        - name: include_metadata
          in: query
          schema:
            type: boolean
            default: true
      responses:
        "200":
          description: Conversation with messages
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ConversationWithMessages"

  /conversations/{id}/messages:
    get:
      summary: List messages with cursor pagination
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
            format: uuid
        - name: cursor
          in: query
          schema:
            type: string
          description: Message ID for cursor-based pagination
        - name: limit
          in: query
          schema:
            type: integer
            default: 50
            maximum: 200
        - name: direction
          in: query
          schema:
            type: string
            enum: [before, after]
            default: before
          description: Load messages before or after cursor
      responses:
        "200":
          description: Paginated messages
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/PaginatedMessages"

components:
  schemas:
    PaginatedConversations:
      type: object
      properties:
        items:
          type: array
          items:
            $ref: "#/components/schemas/Conversation"
        pagination:
          $ref: "#/components/schemas/CursorPagination"

    CursorPagination:
      type: object
      properties:
        next_cursor:
          type: string
          nullable: true
        prev_cursor:
          type: string
          nullable: true
        has_more:
          type: boolean
        total:
          type: integer
          description: Optional total count (expensive, may be null)

    Conversation:
      type: object
      properties:
        id:
          type: string
          format: uuid
        title:
          type: string
        mode:
          type: string
          enum: [local, global, hybrid, naive]
        is_pinned:
          type: boolean
        is_archived:
          type: boolean
        folder_id:
          type: string
          format: uuid
          nullable: true
        message_count:
          type: integer
        last_message_preview:
          type: string
          nullable: true
        created_at:
          type: string
          format: date-time
        updated_at:
          type: string
          format: date-time

    ConversationWithMessages:
      allOf:
        - $ref: "#/components/schemas/Conversation"
        - type: object
          properties:
            messages:
              type: array
              items:
                $ref: "#/components/schemas/Message"
            has_more_messages:
              type: boolean

    Message:
      type: object
      properties:
        id:
          type: string
          format: uuid
        role:
          type: string
          enum: [user, assistant, system]
        content:
          type: string
        mode:
          type: string
          nullable: true
        tokens_used:
          type: integer
          nullable: true
        duration_ms:
          type: integer
          nullable: true
        thinking_time_ms:
          type: integer
          nullable: true
        context:
          $ref: "#/components/schemas/MessageContext"
        is_error:
          type: boolean
        created_at:
          type: string
          format: date-time

    MessageContext:
      type: object
      properties:
        sources:
          type: array
          items:
            $ref: "#/components/schemas/Source"
        entities:
          type: array
          items:
            type: string
        relationships:
          type: array
          items:
            type: string

    Source:
      type: object
      properties:
        id:
          type: string
        content:
          type: string
        score:
          type: number
        document_id:
          type: string
          nullable: true
```

### 2.2 Streaming API (SSE)

```typescript
/**
 * Chat Completion Stream
 * Endpoint: POST /chat/completions/stream
 * Content-Type: text/event-stream
 */

// Event Types (expanded from current implementation)
type StreamEvent =
  | { type: 'conversation'; conversation_id: string; user_message_id: string }
  | { type: 'context'; sources: Source[]; entities: string[]; relationships: string[] }
  | { type: 'thinking'; content: string }  // Chain-of-thought content
  | { type: 'token'; content: string }     // Response token
  | { type: 'done';
      assistant_message_id: string;
      tokens_used: number;
      duration_ms: number;
      thinking_time_ms?: number;
    }
  | { type: 'error'; code: string; message: string; retryable: boolean }
  | { type: 'heartbeat' }  // Keep-alive every 15s

// Example stream for a query
data: {"type":"conversation","conversation_id":"abc-123","user_message_id":"msg-001"}

data: {"type":"context","sources":[{"id":"doc-1","score":0.92}],"entities":["SARAH"],"relationships":["COLLABORATES_WITH"]}

data: {"type":"thinking","content":"Let me analyze the relationships..."}

data: {"type":"token","content":"##"}
data: {"type":"token","content":" Key"}
data: {"type":"token","content":" Rel"}
data: {"type":"token","content":"ationships"}
data: {"type":"token","content":"\n\n"}
data: {"type":"token","content":"Sarah"}
...

data: {"type":"done","assistant_message_id":"msg-002","tokens_used":234,"duration_ms":2345}
```

---

## 3. Markdown Rendering Pipeline

### 3.1 Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     STREAMING MARKDOWN PIPELINE                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐   ┌──────────────┐   ┌─────────────┐   ┌───────────────┐  │
│  │ SSE Stream  │──►│ Accumulator  │──►│ Normalizer  │──►│ marked.lexer  │  │
│  │  (tokens)   │   │   Buffer     │   │             │   │               │  │
│  └─────────────┘   └──────────────┘   └─────────────┘   └───────┬───────┘  │
│                                                                   │         │
│                                                                   ▼         │
│  ┌─────────────┐   ┌──────────────┐   ┌─────────────┐   ┌───────────────┐  │
│  │   React     │◄──│ Token Render │◄──│ Completion  │◄──│   Token[]     │  │
│  │   Tree      │   │  Components  │   │   Checker   │   │               │  │
│  └─────────────┘   └──────────────┘   └─────────────┘   └───────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

COMPONENTS:
─────────────
1. Accumulator Buffer: Concatenates incoming tokens
2. Normalizer: Fixes streaming artifacts (spaces around **)
3. marked.lexer: Tokenizes complete markdown
4. Completion Checker: Determines if tokens are renderable
5. Token Render Components: React components per token type
```

### 3.2 Marked Extensions to Add

```typescript
// src/lib/markdown/extensions/index.ts

import { marked } from "marked";

// 1. GitHub-style Alerts Extension
export const alertExtension = {
  name: "alert",
  level: "block",
  start(src: string) {
    return src.match(/^> \[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]/)?.index;
  },
  tokenizer(src: string) {
    const rule =
      /^> \[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\n((?:> .*(?:\n|$))*)/;
    const match = rule.exec(src);
    if (match) {
      return {
        type: "alert",
        raw: match[0],
        alertType: match[1] as
          | "NOTE"
          | "TIP"
          | "IMPORTANT"
          | "WARNING"
          | "CAUTION",
        text: match[2].replace(/^> ?/gm, ""),
        tokens: [],
      };
    }
  },
  renderer(token: any) {
    // Placeholder - actual rendering in React component
    return `<div data-alert="${token.alertType}">${token.text}</div>`;
  },
};

// 2. Footnotes Extension
export const footnoteExtension = {
  name: "footnote",
  level: "inline",
  start(src: string) {
    return src.match(/\[\^/)?.index;
  },
  tokenizer(src: string) {
    const rule = /^\[\^(\w+)\]/;
    const match = rule.exec(src);
    if (match) {
      return {
        type: "footnote",
        raw: match[0],
        id: match[1],
      };
    }
  },
  renderer(token: any) {
    return `<sup class="footnote-ref">[${token.id}]</sup>`;
  },
};

// 3. Citation Extension (enhanced)
export const citationExtension = {
  name: "citation",
  level: "inline",
  start(src: string) {
    return src.match(/\[source:/)?.index;
  },
  tokenizer(src: string) {
    const rule = /^\[source:(\d+)(?::([^\]]+))?\]/;
    const match = rule.exec(src);
    if (match) {
      return {
        type: "citation",
        raw: match[0],
        sourceId: match[1],
        label: match[2] || match[1],
      };
    }
  },
  renderer(token: any) {
    return `<cite data-source-id="${token.sourceId}">[${token.label}]</cite>`;
  },
};

// 4. Details/Collapsible Extension
export const detailsExtension = {
  name: "details",
  level: "block",
  start(src: string) {
    return src.match(/<details>/)?.index;
  },
  tokenizer(src: string) {
    const rule =
      /^<details>\s*\n<summary>(.*?)<\/summary>\s*\n([\s\S]*?)\n<\/details>/;
    const match = rule.exec(src);
    if (match) {
      return {
        type: "details",
        raw: match[0],
        summary: match[1],
        text: match[2],
        tokens: [],
      };
    }
  },
  renderer(token: any) {
    return `<details><summary>${token.summary}</summary>${token.text}</details>`;
  },
};

// Configure marked with all extensions
export function configureMarked() {
  const options = { breaks: true, gfm: true };

  marked.use({
    extensions: [
      alertExtension,
      footnoteExtension,
      citationExtension,
      detailsExtension,
      // Existing: katex block/inline from current implementation
    ],
    ...options,
  });
}
```

### 3.3 Token Completion Detection

````typescript
// src/lib/markdown/completion-checker.ts

import type { Token } from "marked";

interface CompletionResult {
  isComplete: boolean;
  pendingType?: string;
  requiresMore?: string;
}

/**
 * Checks if the last token in the stream is complete and renderable.
 * Incomplete tokens should be held in a buffer until complete.
 */
export function checkTokenCompletion(
  tokens: Token[],
  rawContent: string
): CompletionResult {
  if (tokens.length === 0) {
    return { isComplete: true };
  }

  const lastToken = tokens[tokens.length - 1];

  switch (lastToken.type) {
    case "code": {
      // Code blocks need closing ```
      const codeContent = (lastToken as any).raw || "";
      const hasClosing = codeContent.trim().endsWith("```");
      if (!hasClosing) {
        return {
          isComplete: false,
          pendingType: "code",
          requiresMore: "Waiting for closing ```",
        };
      }
      break;
    }

    case "table": {
      // Tables need complete row (ending with |)
      const lastLine = rawContent.split("\n").pop() || "";
      if (lastLine.includes("|") && !lastLine.trim().endsWith("|")) {
        return {
          isComplete: false,
          pendingType: "table",
          requiresMore: "Waiting for row completion",
        };
      }
      break;
    }

    case "blockKatex": {
      // Math blocks need closing $$
      if (!rawContent.trimEnd().endsWith("$$")) {
        return {
          isComplete: false,
          pendingType: "math_block",
          requiresMore: "Waiting for closing $$",
        };
      }
      break;
    }

    case "paragraph": {
      // Check for incomplete inline elements in paragraph
      const text = (lastToken as any).text || "";

      // Incomplete bold
      if ((text.match(/\*\*/g) || []).length % 2 !== 0) {
        return {
          isComplete: false,
          pendingType: "bold",
          requiresMore: "Waiting for closing **",
        };
      }

      // Incomplete inline code
      if ((text.match(/`/g) || []).length % 2 !== 0) {
        return {
          isComplete: false,
          pendingType: "inline_code",
          requiresMore: "Waiting for closing `",
        };
      }

      // Incomplete inline math
      if ((text.match(/\$/g) || []).length % 2 !== 0) {
        return {
          isComplete: false,
          pendingType: "inline_math",
          requiresMore: "Waiting for closing $",
        };
      }
      break;
    }
  }

  return { isComplete: true };
}

/**
 * Get a safe subset of tokens that can be rendered.
 * Holds back incomplete tokens.
 */
export function getSafeTokens(
  tokens: Token[],
  rawContent: string
): {
  renderableTokens: Token[];
  pendingContent: string;
} {
  const result = checkTokenCompletion(tokens, rawContent);

  if (result.isComplete) {
    return { renderableTokens: tokens, pendingContent: "" };
  }

  // Hold back last token if incomplete
  const renderableTokens = tokens.slice(0, -1);
  const lastToken = tokens[tokens.length - 1];
  const pendingContent = (lastToken as any).raw || "";

  return { renderableTokens, pendingContent };
}
````

### 3.4 HTML Sanitization

```typescript
// src/lib/markdown/sanitize.ts

import DOMPurify from "dompurify";

// Configure DOMPurify for our needs
const purifyConfig: DOMPurify.Config = {
  ALLOWED_TAGS: [
    // Structure
    "div",
    "span",
    "p",
    "br",
    "hr",
    // Headings
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    // Lists
    "ul",
    "ol",
    "li",
    // Tables
    "table",
    "thead",
    "tbody",
    "tr",
    "th",
    "td",
    // Text formatting
    "strong",
    "em",
    "del",
    "s",
    "code",
    "pre",
    // Quotes and citations
    "blockquote",
    "cite",
    "q",
    // Links and media
    "a",
    "img",
    // Details/summary
    "details",
    "summary",
    // Semantic
    "sup",
    "sub",
    "mark",
  ],
  ALLOWED_ATTR: [
    "class",
    "id",
    "href",
    "src",
    "alt",
    "title",
    "data-*", // Allow data attributes for our components
    "target",
    "rel", // For links
    "colspan",
    "rowspan", // For tables
    "open", // For details
  ],
  ALLOW_DATA_ATTR: true,
  ADD_ATTR: ["target"], // Add target="_blank" to links
  ADD_TAGS: ["iframe"], // Allow iframes for embeds (with strict sanitization)
};

// Hooks for link security
DOMPurify.addHook("afterSanitizeAttributes", (node) => {
  if (node.tagName === "A") {
    node.setAttribute("target", "_blank");
    node.setAttribute("rel", "noopener noreferrer");
  }
  if (node.tagName === "IFRAME") {
    node.setAttribute("sandbox", "allow-scripts allow-same-origin");
  }
});

export function sanitizeHtml(html: string): string {
  return DOMPurify.sanitize(html, purifyConfig);
}

export function sanitizeHtmlToken(html: string): string {
  // Stricter sanitization for inline HTML tokens
  return DOMPurify.sanitize(html, {
    ...purifyConfig,
    ALLOWED_TAGS: ["br", "span", "strong", "em", "code", "a", "sup", "sub"],
  });
}
```

---

## 4. Frontend Architecture

### 4.1 State Management

```typescript
// src/stores/use-query-store.ts

import { create } from "zustand";
import { persist, subscribeWithSelector } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";

// ============================================================================
// Types
// ============================================================================

export type StreamingPhase =
  | "idle"
  | "thinking"
  | "retrieving"
  | "generating"
  | "complete"
  | "error";

export interface StreamingState {
  phase: StreamingPhase;
  content: string;
  thinkingContent: string;
  tokensGenerated: number;
  startTime: number;
  thinkingDuration?: number;
  sources?: Source[];
  error?: { code: string; message: string; retryable: boolean };
}

export interface QueryFilters {
  modes: ConversationMode[];
  archived: boolean;
  pinned: boolean | null;
  folderId: string | null;
  search: string;
  dateRange: { from: string | null; to: string | null };
}

export interface QueryUIState {
  // Active conversation
  activeConversationId: string | null;

  // Panel state
  historyPanelOpen: boolean;
  historyPanelWidth: number;

  // Streaming
  streaming: StreamingState;
  abortController: AbortController | null;

  // Filters & sort
  filters: QueryFilters;
  sort: { field: "updated_at" | "created_at" | "title"; order: "asc" | "desc" };

  // Selection for batch operations
  selectedConversationIds: Set<string>;
  isSelectionMode: boolean;

  // UI preferences (persisted)
  showThinking: boolean;
  showSources: boolean;
  showMetadata: boolean;
}

// ============================================================================
// Store Implementation
// ============================================================================

export const useQueryStore = create<QueryUIState & QueryUIActions>()(
  subscribeWithSelector(
    persist(
      immer((set, get) => ({
        // Initial state
        activeConversationId: null,
        historyPanelOpen: true,
        historyPanelWidth: 280,
        streaming: {
          phase: "idle",
          content: "",
          thinkingContent: "",
          tokensGenerated: 0,
          startTime: 0,
        },
        abortController: null,
        filters: {
          modes: [],
          archived: false,
          pinned: null,
          folderId: null,
          search: "",
          dateRange: { from: null, to: null },
        },
        sort: { field: "updated_at", order: "desc" },
        selectedConversationIds: new Set(),
        isSelectionMode: false,
        showThinking: true,
        showSources: true,
        showMetadata: true,

        // Actions
        setActiveConversation: (id) => set({ activeConversationId: id }),

        toggleHistoryPanel: () =>
          set((state) => {
            state.historyPanelOpen = !state.historyPanelOpen;
          }),

        startStreaming: () => {
          const controller = new AbortController();
          set({
            abortController: controller,
            streaming: {
              phase: "thinking",
              content: "",
              thinkingContent: "",
              tokensGenerated: 0,
              startTime: Date.now(),
            },
          });
          return controller;
        },

        appendStreamContent: (token) =>
          set((state) => {
            if (state.streaming.phase === "thinking") {
              state.streaming.phase = "generating";
              state.streaming.thinkingDuration =
                Date.now() - state.streaming.startTime;
            }
            state.streaming.content += token;
            state.streaming.tokensGenerated += 1;
          }),

        setStreamPhase: (phase) =>
          set((state) => {
            state.streaming.phase = phase;
          }),

        setStreamError: (error) =>
          set((state) => {
            state.streaming.phase = "error";
            state.streaming.error = error;
          }),

        completeStreaming: () =>
          set((state) => {
            state.streaming.phase = "complete";
            state.abortController = null;
          }),

        abortStreaming: () => {
          const { abortController } = get();
          abortController?.abort();
          set((state) => {
            state.streaming.phase = "idle";
            state.abortController = null;
          });
        },

        setFilters: (filters) =>
          set((state) => {
            Object.assign(state.filters, filters);
          }),

        resetFilters: () =>
          set((state) => {
            state.filters = {
              modes: [],
              archived: false,
              pinned: null,
              folderId: null,
              search: "",
              dateRange: { from: null, to: null },
            };
          }),
      })),
      {
        name: "edgequake-query-ui",
        partialize: (state) => ({
          historyPanelOpen: state.historyPanelOpen,
          historyPanelWidth: state.historyPanelWidth,
          showThinking: state.showThinking,
          showSources: state.showSources,
          showMetadata: state.showMetadata,
        }),
      }
    )
  )
);
```

### 4.2 Component Structure

```
src/components/query/
├── index.ts                          # Public exports
├── QueryPage.tsx                     # Page layout container
├── QueryInterface.tsx                # Main interface logic
├── history/
│   ├── ConversationList.tsx          # Virtualized list
│   ├── ConversationItem.tsx          # Single item
│   ├── ConversationFilters.tsx       # Filter controls
│   ├── FolderTree.tsx               # Folder sidebar
│   └── HistoryPanel.tsx             # Panel container
├── chat/
│   ├── ChatArea.tsx                  # Scrollable message area
│   ├── ChatMessage.tsx               # Message container
│   ├── UserMessage.tsx               # User bubble
│   ├── AssistantMessage.tsx          # Assistant bubble
│   ├── MessageMetadata.tsx           # Tokens, time, mode
│   ├── ThinkingSection.tsx           # Collapsible COT
│   └── SourceCitations.tsx           # Source references
├── input/
│   ├── QueryInput.tsx                # Text input area
│   ├── ModeSelector.tsx              # Query mode picker
│   └── SendButton.tsx                # Submit button
├── markdown/
│   ├── StreamingMarkdownRenderer.tsx # Main entry
│   ├── MarkdownTokens.tsx            # Block renderer
│   ├── MarkdownInlineTokens.tsx      # Inline renderer
│   ├── tokens/
│   │   ├── HeadingToken.tsx
│   │   ├── ParagraphToken.tsx
│   │   ├── CodeBlockToken.tsx
│   │   ├── TableToken.tsx
│   │   ├── AlertToken.tsx            # NEW
│   │   ├── DetailsToken.tsx          # NEW
│   │   └── FootnoteToken.tsx         # NEW
│   ├── extensions/
│   │   ├── alert-extension.ts
│   │   ├── citation-extension.ts
│   │   ├── details-extension.ts
│   │   ├── footnote-extension.ts
│   │   └── katex-extension.ts
│   └── utils/
│       ├── configure-marked.ts
│       ├── completion-checker.ts
│       └── sanitize.ts
├── loading/
│   ├── ThinkingIndicator.tsx         # Brain animation
│   ├── StreamingSkeleton.tsx         # Content skeleton
│   ├── MessageSkeleton.tsx           # List item skeleton
│   └── StreamingCursor.tsx           # Blinking cursor
└── dialogs/
    ├── ExportDialog.tsx
    ├── ShareDialog.tsx
    └── DeleteConfirmDialog.tsx
```

### 4.3 React Query Configuration

```typescript
// src/lib/query-client.ts

import { QueryClient, QueryCache, MutationCache } from "@tanstack/react-query";
import { toast } from "sonner";

export const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error, query) => {
      // Only show toast for queries that have been retried
      if (query.state.fetchFailureCount > 0) {
        toast.error(`Failed to load data: ${error.message}`);
      }
    },
  }),
  mutationCache: new MutationCache({
    onError: (error) => {
      toast.error(`Operation failed: ${error.message}`);
    },
  }),
  defaultOptions: {
    queries: {
      staleTime: 30_000, // 30 seconds
      gcTime: 5 * 60_000, // 5 minutes
      retry: 2,
      retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: 1,
    },
  },
});

// Query keys factory
export const queryKeys = {
  conversations: {
    all: ["conversations"] as const,
    list: (filters: Record<string, unknown>) =>
      ["conversations", "list", filters] as const,
    detail: (id: string) => ["conversations", "detail", id] as const,
    messages: (id: string, cursor?: string) =>
      ["conversations", "messages", id, cursor] as const,
  },
  folders: {
    all: ["folders"] as const,
    list: () => ["folders", "list"] as const,
  },
};
```

---

## 5. Performance Optimizations

### 5.1 Virtualization Strategy

```typescript
// src/hooks/use-virtual-conversations.ts

import { useVirtualizer } from "@tanstack/react-virtual";
import { useRef, useMemo } from "react";

export function useVirtualConversations(
  conversations: Conversation[],
  parentRef: React.RefObject<HTMLElement>
) {
  const rowVirtualizer = useVirtualizer({
    count: conversations.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 64, // Estimated row height
    overscan: 5, // Render 5 extra items above/below viewport
    measureElement: (el) => el.getBoundingClientRect().height,
  });

  return {
    virtualRows: rowVirtualizer.getVirtualItems(),
    totalSize: rowVirtualizer.getTotalSize(),
    measureElement: rowVirtualizer.measureElement,
  };
}
```

### 5.2 Auto-scroll Optimization

```typescript
// src/hooks/use-auto-scroll.ts

import { useRef, useCallback, useEffect } from "react";
import { useThrottledCallback } from "use-debounce";

interface UseAutoScrollOptions {
  enabled: boolean;
  behavior?: ScrollBehavior;
  threshold?: number;
}

export function useAutoScroll(
  scrollRef: React.RefObject<HTMLElement>,
  options: UseAutoScrollOptions
) {
  const { enabled, behavior = "smooth", threshold = 100 } = options;
  const isUserScrollingRef = useRef(false);
  const wasAtBottomRef = useRef(true);

  const scrollToBottom = useThrottledCallback(
    () => {
      if (!scrollRef.current || isUserScrollingRef.current) return;

      scrollRef.current.scrollTo({
        top: scrollRef.current.scrollHeight,
        behavior,
      });
    },
    16, // ~60fps
    { leading: true, trailing: true }
  );

  const handleScroll = useCallback(() => {
    if (!scrollRef.current) return;

    const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
    const isAtBottom = scrollHeight - scrollTop - clientHeight < threshold;

    // User scrolled up - disable auto-scroll
    if (!isAtBottom && wasAtBottomRef.current) {
      isUserScrollingRef.current = true;
    }

    // User scrolled back to bottom - re-enable auto-scroll
    if (isAtBottom && !wasAtBottomRef.current) {
      isUserScrollingRef.current = false;
    }

    wasAtBottomRef.current = isAtBottom;
  }, [threshold]);

  useEffect(() => {
    const element = scrollRef.current;
    if (!element) return;

    element.addEventListener("scroll", handleScroll, { passive: true });
    return () => element.removeEventListener("scroll", handleScroll);
  }, [handleScroll]);

  return {
    scrollToBottom: enabled ? scrollToBottom : () => {},
    isAtBottom: wasAtBottomRef.current,
    isUserScrolling: isUserScrollingRef.current,
    resetScroll: () => {
      isUserScrollingRef.current = false;
      wasAtBottomRef.current = true;
    },
  };
}
```

---

## References

- [Audit Findings](./01_audit_findings.md)
- [Design Strategy](./02_design_strategy.md)
- [Implementation Roadmap](./04_implementation_roadmap.md)
- [OpenWebUI Source](https://github.com/open-webui/open-webui)
- [marked.js Documentation](https://marked.js.org/)
- [TanStack Query Docs](https://tanstack.com/query/latest)

---

_Document Version: 1.0 | Last Updated: December 27, 2025_
