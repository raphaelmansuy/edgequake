# Phase 3: Technical Specification

**Document**: `03_technical_spec.md`  
**Created**: 2024-12-27  
**Status**: Complete

---

## 1. Overview

This document provides the implementation blueprint for the Query Page improvements, including database schema, API specifications, markdown rendering pipeline, and frontend architecture.

**Prerequisites**:

- [01_audit_findings.md](01_audit_findings.md) - Understanding of current issues
- [02_design_strategy.md](02_design_strategy.md) - Design principles and IA

---

## 2. System Architecture

### 2.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            EdgeQuake Query System                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                         Frontend (Next.js)                           │    │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────────┐  │    │
│  │  │ Query Page  │  │ State Stores │  │ Services                   │  │    │
│  │  │ Components  │◀▶│ (Zustand)    │◀▶│ (React Query + WebSocket)  │  │    │
│  │  └─────────────┘  └──────────────┘  └────────────────────────────┘  │    │
│  └────────────────────────────▲────────────────────────────────────────┘    │
│                               │ HTTP/WebSocket                               │
│  ┌────────────────────────────▼────────────────────────────────────────┐    │
│  │                         Backend (Axum)                               │    │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────────┐  │    │
│  │  │ API Router  │─▶│ Handlers     │─▶│ Services                   │  │    │
│  │  │ /api/v1/*   │  │ Query/Conv   │  │ Persistence + LLM          │  │    │
│  │  └─────────────┘  └──────────────┘  └────────────────────────────┘  │    │
│  └────────────────────────────▲────────────────────────────────────────┘    │
│                               │ SQL                                          │
│  ┌────────────────────────────▼────────────────────────────────────────┐    │
│  │                         Database (PostgreSQL)                        │    │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │    │
│  │  │ conversations   │  │ messages        │  │ tenants/workspaces  │  │    │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │    │
│  └──────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Data Flow for Query Submission

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           Query Submission Flow                               │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  1. User types query         2. Frontend creates        3. API receives       │
│  ┌─────────────────┐        message + sends          ┌─────────────────┐     │
│  │ QueryInterface  │────────────────────────────────▶│ POST /messages  │     │
│  │ handleSubmit()  │        to server               │ with conv_id    │     │
│  └─────────────────┘                                 └────────┬────────┘     │
│                                                               │              │
│  4. Server persists         5. LLM streaming         6. Tokens sent         │
│  ┌─────────────────┐        response generated       via SSE/WebSocket      │
│  │ INSERT INTO     │◀───────────────────────────────┌────────▼────────┐     │
│  │ messages        │        ┌────────────────┐      │ EventSource     │     │
│  └─────────────────┘        │ LLM Provider   │─────▶│ data: {token}   │     │
│                             └────────────────┘      └────────┬────────┘     │
│                                                               │              │
│  7. Frontend updates        8. Markdown rendered    9. Complete response    │
│  ┌─────────────────┐        progressively           saved to DB             │
│  │ updateMessage() │◀───────────────────────────────┌────────▼────────┐     │
│  │ in store        │        tokens ──▶ lexer       │ UPDATE messages │     │
│  └─────────────────┘        ──▶ render             │ SET content=... │     │
│                                                     └─────────────────┘     │
│                                                                               │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Database Schema Design

### 3.1 Entity Relationship Diagram

```
┌─────────────────────┐       ┌─────────────────────┐
│      tenants        │       │    workspaces       │
├─────────────────────┤       ├─────────────────────┤
│ id (PK)             │──┐    │ id (PK)             │
│ name                │  │    │ tenant_id (FK)      │──┐
│ created_at          │  │    │ name                │  │
│ updated_at          │  │    │ created_at          │  │
└─────────────────────┘  │    └─────────────────────┘  │
                         │                             │
                         │    ┌─────────────────────┐  │
                         └───▶│   conversations     │◀─┘
                              ├─────────────────────┤
                              │ id (PK)             │
                              │ tenant_id (FK)      │
                              │ workspace_id (FK)   │
                              │ user_id             │
                              │ title               │
                              │ mode                │
                              │ is_pinned           │
                              │ is_archived         │
                              │ folder_id (FK)      │──┐
                              │ meta (JSONB)        │  │
                              │ created_at          │  │
                              │ updated_at          │  │
                              └──────────┬──────────┘  │
                                         │             │
                         ┌───────────────┘             │
                         │                             │
                         ▼                             │
            ┌─────────────────────┐     ┌──────────────▼────────┐
            │      messages       │     │       folders         │
            ├─────────────────────┤     ├───────────────────────┤
            │ id (PK)             │     │ id (PK)               │
            │ conversation_id (FK)│     │ tenant_id (FK)        │
            │ parent_id (FK)      │──┐  │ workspace_id (FK)     │
            │ role                │  │  │ name                  │
            │ content             │  │  │ parent_id (FK)        │──┐
            │ mode                │  │  │ created_at            │  │
            │ tokens_used         │  │  │ updated_at            │  │
            │ duration_ms         │  │  └───────────────────────┘  │
            │ thinking_time_ms    │  │                             │
            │ context (JSONB)     │  └─────────────────────────────┘
            │ is_error            │
            │ created_at          │
            │ updated_at          │
            └─────────────────────┘
```

### 3.2 DDL Statements

```sql
-- ===========================================================================
-- CONVERSATIONS TABLE
-- Stores conversation metadata, one row per chat session
-- ===========================================================================
CREATE TABLE conversations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    workspace_id    UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    user_id         VARCHAR(255) NOT NULL,  -- Auth user ID
    title           VARCHAR(500) NOT NULL DEFAULT 'New Conversation',
    mode            VARCHAR(50) NOT NULL DEFAULT 'hybrid',  -- local|global|hybrid|naive
    is_pinned       BOOLEAN NOT NULL DEFAULT FALSE,
    is_archived     BOOLEAN NOT NULL DEFAULT FALSE,
    folder_id       UUID REFERENCES folders(id) ON DELETE SET NULL,
    share_id        VARCHAR(64) UNIQUE,  -- For public sharing
    meta            JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_conversations_tenant_user
    ON conversations(tenant_id, user_id, updated_at DESC);
CREATE INDEX idx_conversations_workspace
    ON conversations(workspace_id, updated_at DESC)
    WHERE workspace_id IS NOT NULL;
CREATE INDEX idx_conversations_folder
    ON conversations(folder_id)
    WHERE folder_id IS NOT NULL;
CREATE INDEX idx_conversations_archived
    ON conversations(tenant_id, user_id, is_archived, updated_at DESC);
CREATE INDEX idx_conversations_pinned
    ON conversations(tenant_id, user_id, is_pinned)
    WHERE is_pinned = TRUE;
CREATE INDEX idx_conversations_share
    ON conversations(share_id)
    WHERE share_id IS NOT NULL;

-- Full-text search index
CREATE INDEX idx_conversations_title_search
    ON conversations USING gin(to_tsvector('english', title));

-- ===========================================================================
-- MESSAGES TABLE
-- Stores individual messages within conversations
-- ===========================================================================
CREATE TABLE messages (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id     UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    parent_id           UUID REFERENCES messages(id) ON DELETE SET NULL,
    role                VARCHAR(20) NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content             TEXT NOT NULL,
    mode                VARCHAR(50),  -- Query mode used for this specific message
    tokens_used         INTEGER,
    duration_ms         INTEGER,
    thinking_time_ms    INTEGER,
    context             JSONB,  -- Source citations, entities, etc.
    is_error            BOOLEAN NOT NULL DEFAULT FALSE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for message retrieval
CREATE INDEX idx_messages_conversation
    ON messages(conversation_id, created_at ASC);
CREATE INDEX idx_messages_parent
    ON messages(parent_id)
    WHERE parent_id IS NOT NULL;

-- Full-text search on message content
CREATE INDEX idx_messages_content_search
    ON messages USING gin(to_tsvector('english', content));

-- ===========================================================================
-- FOLDERS TABLE
-- Hierarchical folder structure for organizing conversations
-- ===========================================================================
CREATE TABLE folders (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    workspace_id    UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    user_id         VARCHAR(255) NOT NULL,
    name            VARCHAR(255) NOT NULL,
    parent_id       UUID REFERENCES folders(id) ON DELETE CASCADE,
    position        INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(tenant_id, user_id, parent_id, name)
);

CREATE INDEX idx_folders_parent
    ON folders(parent_id);
CREATE INDEX idx_folders_user
    ON folders(tenant_id, user_id, position);

-- ===========================================================================
-- ROW-LEVEL SECURITY POLICIES
-- ===========================================================================
ALTER TABLE conversations ENABLE ROW LEVEL SECURITY;
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE folders ENABLE ROW LEVEL SECURITY;

-- Conversations: Users can only see their own or shared
CREATE POLICY conversations_tenant_isolation ON conversations
    FOR ALL
    USING (
        tenant_id = current_setting('app.tenant_id')::UUID
        AND (
            user_id = current_setting('app.user_id')
            OR share_id IS NOT NULL
        )
    );

-- Messages: Inherit from conversation access
CREATE POLICY messages_conversation_access ON messages
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM conversations c
            WHERE c.id = messages.conversation_id
            AND c.tenant_id = current_setting('app.tenant_id')::UUID
            AND (
                c.user_id = current_setting('app.user_id')
                OR c.share_id IS NOT NULL
            )
        )
    );

-- Folders: Users can only see their own
CREATE POLICY folders_user_access ON folders
    FOR ALL
    USING (
        tenant_id = current_setting('app.tenant_id')::UUID
        AND user_id = current_setting('app.user_id')
    );
```

### 3.3 Migration Plan

```sql
-- Migration: 001_create_conversation_tables
-- Run after existing schema is in place

-- Step 1: Create new tables (non-destructive)
-- DDL from above...

-- Step 2: Migrate existing localStorage data via API
-- This happens client-side, calling POST /api/v1/conversations/import

-- Step 3: Add trigger for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_conversations_updated_at
    BEFORE UPDATE ON conversations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_messages_updated_at
    BEFORE UPDATE ON messages
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

---

## 4. API Specification

### 4.1 OpenAPI 3.0 Specification

```yaml
openapi: 3.0.3
info:
  title: EdgeQuake Query API
  description: API for managing query conversations and messages
  version: 1.0.0

servers:
  - url: /api/v1

paths:
  /conversations:
    get:
      summary: List conversations
      description: Retrieve paginated list of conversations for the authenticated user
      operationId: listConversations
      tags:
        - Conversations
      parameters:
        - $ref: "#/components/parameters/TenantHeader"
        - $ref: "#/components/parameters/WorkspaceHeader"
        - $ref: "#/components/parameters/Cursor"
        - $ref: "#/components/parameters/Limit"
        - name: filter[mode]
          in: query
          schema:
            type: array
            items:
              type: string
              enum: [local, global, hybrid, naive]
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
        - name: filter[date_from]
          in: query
          schema:
            type: string
            format: date-time
        - name: filter[date_to]
          in: query
          schema:
            type: string
            format: date-time
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

    post:
      summary: Create conversation
      operationId: createConversation
      tags:
        - Conversations
      parameters:
        - $ref: "#/components/parameters/TenantHeader"
        - $ref: "#/components/parameters/WorkspaceHeader"
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/CreateConversationRequest"
      responses:
        "201":
          description: Conversation created
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Conversation"

  /conversations/{id}:
    get:
      summary: Get conversation by ID
      operationId: getConversation
      tags:
        - Conversations
      parameters:
        - $ref: "#/components/parameters/ConversationId"
        - $ref: "#/components/parameters/TenantHeader"
      responses:
        "200":
          description: Conversation details with messages
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ConversationWithMessages"

    patch:
      summary: Update conversation
      operationId: updateConversation
      tags:
        - Conversations
      parameters:
        - $ref: "#/components/parameters/ConversationId"
        - $ref: "#/components/parameters/TenantHeader"
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/UpdateConversationRequest"
      responses:
        "200":
          description: Conversation updated
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Conversation"

    delete:
      summary: Delete conversation
      operationId: deleteConversation
      tags:
        - Conversations
      parameters:
        - $ref: "#/components/parameters/ConversationId"
        - $ref: "#/components/parameters/TenantHeader"
      responses:
        "204":
          description: Conversation deleted

  /conversations/{id}/messages:
    get:
      summary: Get messages in conversation
      operationId: listMessages
      tags:
        - Messages
      parameters:
        - $ref: "#/components/parameters/ConversationId"
        - $ref: "#/components/parameters/TenantHeader"
        - $ref: "#/components/parameters/Cursor"
        - $ref: "#/components/parameters/Limit"
      responses:
        "200":
          description: Paginated list of messages
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/PaginatedMessages"

    post:
      summary: Add message to conversation
      operationId: createMessage
      tags:
        - Messages
      parameters:
        - $ref: "#/components/parameters/ConversationId"
        - $ref: "#/components/parameters/TenantHeader"
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/CreateMessageRequest"
      responses:
        "201":
          description: Message created and AI response started
          headers:
            X-Stream-URL:
              description: WebSocket URL for streaming response
              schema:
                type: string
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Message"

  /conversations/{id}/messages/{messageId}:
    patch:
      summary: Update message
      operationId: updateMessage
      tags:
        - Messages
      parameters:
        - $ref: "#/components/parameters/ConversationId"
        - $ref: "#/components/parameters/MessageId"
        - $ref: "#/components/parameters/TenantHeader"
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/UpdateMessageRequest"
      responses:
        "200":
          description: Message updated
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Message"

  /conversations/import:
    post:
      summary: Import conversations from client
      description: Migrate localStorage conversations to server
      operationId: importConversations
      tags:
        - Conversations
      parameters:
        - $ref: "#/components/parameters/TenantHeader"
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/ImportConversationsRequest"
      responses:
        "200":
          description: Import results
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ImportConversationsResponse"

  /conversations/{id}/share:
    post:
      summary: Generate share link
      operationId: shareConversation
      tags:
        - Conversations
      parameters:
        - $ref: "#/components/parameters/ConversationId"
        - $ref: "#/components/parameters/TenantHeader"
      responses:
        "200":
          description: Share link generated
          content:
            application/json:
              schema:
                type: object
                properties:
                  share_id:
                    type: string
                  share_url:
                    type: string

    delete:
      summary: Remove share link
      operationId: unshareConversation
      tags:
        - Conversations
      parameters:
        - $ref: "#/components/parameters/ConversationId"
        - $ref: "#/components/parameters/TenantHeader"
      responses:
        "204":
          description: Share link removed

components:
  parameters:
    TenantHeader:
      name: X-Tenant-ID
      in: header
      required: true
      schema:
        type: string
        format: uuid

    WorkspaceHeader:
      name: X-Workspace-ID
      in: header
      schema:
        type: string
        format: uuid

    ConversationId:
      name: id
      in: path
      required: true
      schema:
        type: string
        format: uuid

    MessageId:
      name: messageId
      in: path
      required: true
      schema:
        type: string
        format: uuid

    Cursor:
      name: cursor
      in: query
      description: Opaque cursor for pagination
      schema:
        type: string

    Limit:
      name: limit
      in: query
      schema:
        type: integer
        minimum: 1
        maximum: 100
        default: 20

  schemas:
    Conversation:
      type: object
      properties:
        id:
          type: string
          format: uuid
        tenant_id:
          type: string
          format: uuid
        workspace_id:
          type: string
          format: uuid
          nullable: true
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
        share_id:
          type: string
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

    Message:
      type: object
      properties:
        id:
          type: string
          format: uuid
        conversation_id:
          type: string
          format: uuid
        parent_id:
          type: string
          format: uuid
          nullable: true
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
          $ref: "#/components/schemas/QueryContext"
        is_error:
          type: boolean
        created_at:
          type: string
          format: date-time
        updated_at:
          type: string
          format: date-time

    QueryContext:
      type: object
      nullable: true
      properties:
        sources:
          type: array
          items:
            type: object
            properties:
              id:
                type: string
              title:
                type: string
              content:
                type: string
              score:
                type: number
        entities:
          type: array
          items:
            type: string
        relationships:
          type: array
          items:
            type: string

    PaginatedConversations:
      type: object
      properties:
        items:
          type: array
          items:
            $ref: "#/components/schemas/Conversation"
        pagination:
          $ref: "#/components/schemas/PaginationMeta"

    PaginatedMessages:
      type: object
      properties:
        items:
          type: array
          items:
            $ref: "#/components/schemas/Message"
        pagination:
          $ref: "#/components/schemas/PaginationMeta"

    PaginationMeta:
      type: object
      properties:
        next_cursor:
          type: string
          nullable: true
        prev_cursor:
          type: string
          nullable: true
        total:
          type: integer
        has_more:
          type: boolean

    CreateConversationRequest:
      type: object
      properties:
        title:
          type: string
          default: "New Conversation"
        mode:
          type: string
          enum: [local, global, hybrid, naive]
          default: hybrid
        folder_id:
          type: string
          format: uuid
          nullable: true

    UpdateConversationRequest:
      type: object
      properties:
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

    CreateMessageRequest:
      type: object
      required:
        - content
        - role
      properties:
        content:
          type: string
        role:
          type: string
          enum: [user]
        parent_id:
          type: string
          format: uuid
          nullable: true
        stream:
          type: boolean
          default: true

    UpdateMessageRequest:
      type: object
      properties:
        content:
          type: string
        tokens_used:
          type: integer
        duration_ms:
          type: integer
        thinking_time_ms:
          type: integer
        context:
          $ref: "#/components/schemas/QueryContext"
        is_error:
          type: boolean

    ImportConversationsRequest:
      type: object
      required:
        - conversations
      properties:
        conversations:
          type: array
          items:
            type: object
            properties:
              id:
                type: string
              title:
                type: string
              messages:
                type: array
                items:
                  type: object
              created_at:
                type: integer
              updated_at:
                type: integer

    ImportConversationsResponse:
      type: object
      properties:
        imported:
          type: integer
        failed:
          type: integer
        errors:
          type: array
          items:
            type: object
            properties:
              id:
                type: string
              error:
                type: string
```

### 4.2 Cursor-Based Pagination

```typescript
// Server-side cursor encoding
interface CursorPayload {
  updated_at: number; // Unix timestamp
  id: string; // UUID for tie-breaking
}

function encodeCursor(payload: CursorPayload): string {
  return Buffer.from(JSON.stringify(payload)).toString("base64url");
}

function decodeCursor(cursor: string): CursorPayload {
  return JSON.parse(Buffer.from(cursor, "base64url").toString());
}

// SQL query with cursor
const query = `
  SELECT * FROM conversations
  WHERE tenant_id = $1
    AND user_id = $2
    AND (updated_at, id) < ($3, $4)  -- Cursor comparison
  ORDER BY updated_at DESC, id DESC
  LIMIT $5
`;
```

---

## 5. Markdown Rendering Pipeline

### 5.1 Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Token-Based Markdown Pipeline                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────┐    ┌──────────────┐    ┌─────────────────────────────────┐ │
│  │ Raw Content │───▶│ Lexer        │───▶│ Token[]                         │ │
│  │ (string)    │    │ (marked.js)  │    │ [{type: 'paragraph', text:...}] │ │
│  └─────────────┘    └──────────────┘    └───────────────┬─────────────────┘ │
│                                                          │                   │
│                     ┌────────────────────────────────────┘                   │
│                     ▼                                                        │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                         TokenRenderer                                    ││
│  │  ┌─────────────────────────────────────────────────────────────────┐    ││
│  │  │ switch(token.type) {                                            │    ││
│  │  │   case 'paragraph': return <ParagraphToken {...} />             │    ││
│  │  │   case 'heading':   return <HeadingToken {...} />               │    ││
│  │  │   case 'code':      return <CodeToken {...} />                  │    ││
│  │  │   case 'table':     return <TableToken {...} />                 │    ││
│  │  │   case 'list':      return <ListToken {...} />                  │    ││
│  │  │   case 'blockquote': return <BlockquoteToken {...} />           │    ││
│  │  │   case 'hr':        return <HorizontalRule />                   │    ││
│  │  │   case 'html':      return <HtmlToken {...} />                  │    ││
│  │  │   default:          return <FallbackToken {...} />              │    ││
│  │  │ }                                                               │    ││
│  │  └─────────────────────────────────────────────────────────────────┘    ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Component Structure

```typescript
// src/components/query/markdown/types.ts
import type { Token } from "marked";

export interface TokenRendererProps {
  token: Token;
  isStreaming: boolean;
  onComplete?: () => void;
}

export interface InlineTokenRendererProps {
  tokens: Token[];
  isStreaming: boolean;
}
```

```typescript
// src/components/query/markdown/TokenRenderer.tsx
import { memo } from "react";
import type { Token } from "marked";
import { ParagraphToken } from "./tokens/ParagraphToken";
import { HeadingToken } from "./tokens/HeadingToken";
import { CodeToken } from "./tokens/CodeToken";
import { TableToken } from "./tokens/TableToken";
import { ListToken } from "./tokens/ListToken";
import { BlockquoteToken } from "./tokens/BlockquoteToken";

interface Props {
  tokens: Token[];
  isStreaming: boolean;
}

export const TokenRenderer = memo(function TokenRenderer({
  tokens,
  isStreaming,
}: Props) {
  return (
    <>
      {tokens.map((token, idx) => {
        const key = `${token.type}-${idx}`;
        const isLast = idx === tokens.length - 1;

        switch (token.type) {
          case "paragraph":
            return (
              <ParagraphToken
                key={key}
                token={token}
                isStreaming={isStreaming && isLast}
              />
            );
          case "heading":
            return <HeadingToken key={key} token={token} />;
          case "code":
            return (
              <CodeToken
                key={key}
                token={token}
                isStreaming={isStreaming && isLast}
              />
            );
          case "table":
            return <TableToken key={key} token={token} />;
          case "list":
            return (
              <ListToken key={key} token={token} isStreaming={isStreaming} />
            );
          case "blockquote":
            return <BlockquoteToken key={key} token={token} />;
          case "hr":
            return <hr key={key} className="my-6 border-t border-border" />;
          case "space":
            return <div key={key} className="my-2" />;
          default:
            return <FallbackToken key={key} token={token} />;
        }
      })}
    </>
  );
});
```

### 5.3 Streaming Buffer Strategy

````typescript
// src/lib/markdown/streaming-parser.ts
import { marked } from "marked";

export class StreamingMarkdownParser {
  private buffer = "";
  private completeTokens: marked.Token[] = [];

  constructor() {
    // Configure marked for streaming
    marked.use({
      breaks: true,
      gfm: true,
    });
  }

  /**
   * Process incoming chunk and return renderable tokens
   */
  processChunk(chunk: string): {
    tokens: marked.Token[];
    partialText: string;
    isPartial: boolean;
  } {
    this.buffer += chunk;

    // Find safe split point (complete blocks/paragraphs)
    const { complete, remainder } = this.splitAtSafeBoundary(this.buffer);

    if (complete) {
      const newTokens = marked.lexer(complete);
      this.completeTokens.push(...newTokens);
      this.buffer = remainder;
    }

    return {
      tokens: [...this.completeTokens],
      partialText: this.buffer,
      isPartial: this.buffer.length > 0,
    };
  }

  /**
   * Finalize parsing - flush remaining buffer
   */
  finalize(): marked.Token[] {
    if (this.buffer) {
      const finalTokens = marked.lexer(this.buffer);
      this.completeTokens.push(...finalTokens);
      this.buffer = "";
    }
    return this.completeTokens;
  }

  /**
   * Find a safe boundary to split content
   * Safe boundaries: double newlines, end of code blocks, etc.
   */
  private splitAtSafeBoundary(text: string): {
    complete: string;
    remainder: string;
  } {
    // Check for unclosed code blocks
    const codeBlockCount = (text.match(/```/g) || []).length;
    if (codeBlockCount % 2 !== 0) {
      // Code block is open, don't split
      return { complete: "", remainder: text };
    }

    // Check for unclosed inline elements
    const boldCount = (text.match(/\*\*/g) || []).length;
    const italicCount = (text.match(/(?<!\*)\*(?!\*)/g) || []).length;

    if (boldCount % 2 !== 0 || italicCount % 2 !== 0) {
      // Find last complete paragraph
      const lastDoubleNewline = text.lastIndexOf("\n\n");
      if (lastDoubleNewline > 0) {
        return {
          complete: text.slice(0, lastDoubleNewline + 2),
          remainder: text.slice(lastDoubleNewline + 2),
        };
      }
      return { complete: "", remainder: text };
    }

    // Find last paragraph boundary
    const lastDoubleNewline = text.lastIndexOf("\n\n");
    if (lastDoubleNewline > 0) {
      return {
        complete: text.slice(0, lastDoubleNewline + 2),
        remainder: text.slice(lastDoubleNewline + 2),
      };
    }

    return { complete: "", remainder: text };
  }

  reset(): void {
    this.buffer = "";
    this.completeTokens = [];
  }
}
````

### 5.4 Library Recommendations

| Feature             | Library                            | Rationale                                          |
| ------------------- | ---------------------------------- | -------------------------------------------------- |
| Markdown parsing    | `marked` (v12+)                    | Fastest lexer, extensible, used by openwebui       |
| Syntax highlighting | `shiki` or `prism-react-renderer`  | Better theme support than react-syntax-highlighter |
| KaTeX               | `katex` + `marked-katex-extension` | Direct integration with marked                     |
| Mermaid             | `mermaid` (dynamic import)         | Standard, lazy-load for performance                |
| Sanitization        | `dompurify`                        | For HTML token handling                            |

---

## 6. Frontend Architecture

### 6.1 State Management

```typescript
// src/stores/use-conversation-store.ts (refactored)
import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";

interface ConversationState {
  // Server-synced state
  conversations: Map<string, Conversation>;
  activeConversationId: string | null;

  // UI state (not persisted to server)
  historyPanelOpen: boolean;
  filterState: FilterState;

  // Sync status
  syncStatus: "idle" | "syncing" | "error";
  lastSyncedAt: number | null;
  pendingChanges: PendingChange[];
}

interface ConversationActions {
  // Remote operations (API calls)
  fetchConversations: (filters?: FilterParams) => Promise<void>;
  fetchConversation: (id: string) => Promise<void>;
  createConversation: () => Promise<string>;
  updateConversation: (
    id: string,
    updates: Partial<Conversation>
  ) => Promise<void>;
  deleteConversation: (id: string) => Promise<void>;

  // Message operations
  addMessage: (
    conversationId: string,
    message: Omit<Message, "id">
  ) => Promise<string>;
  updateMessage: (
    conversationId: string,
    messageId: string,
    updates: Partial<Message>
  ) => void;

  // Optimistic updates
  optimisticAddMessage: (conversationId: string, message: Message) => void;
  rollbackMessage: (conversationId: string, messageId: string) => void;

  // UI actions
  setActiveConversation: (id: string | null) => void;
  toggleHistoryPanel: () => void;
  setFilters: (filters: Partial<FilterState>) => void;

  // Sync
  syncPendingChanges: () => Promise<void>;
  importFromLocalStorage: () => Promise<void>;
}

export const useConversationStore = create<
  ConversationState & ConversationActions
>()(
  subscribeWithSelector(
    immer((set, get) => ({
      // Initial state
      conversations: new Map(),
      activeConversationId: null,
      historyPanelOpen: true,
      filterState: { mode: [], archived: false, search: "" },
      syncStatus: "idle",
      lastSyncedAt: null,
      pendingChanges: [],

      // Implementation of actions...
    }))
  )
);
```

### 6.2 React Query Integration

```typescript
// src/hooks/use-conversations.ts
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { conversationsApi } from "@/lib/api/conversations";

export function useConversations(filters: FilterParams) {
  return useQuery({
    queryKey: ["conversations", filters],
    queryFn: () => conversationsApi.list(filters),
    staleTime: 30_000, // 30 seconds
    refetchOnWindowFocus: true,
  });
}

export function useConversation(id: string) {
  return useQuery({
    queryKey: ["conversation", id],
    queryFn: () => conversationsApi.get(id),
    staleTime: 60_000, // 1 minute
  });
}

export function useCreateConversation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: conversationsApi.create,
    onSuccess: (newConversation) => {
      queryClient.invalidateQueries({ queryKey: ["conversations"] });
      return newConversation;
    },
  });
}

export function useSendMessage(conversationId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (content: string) =>
      conversationsApi.sendMessage(conversationId, content),
    onMutate: async (content) => {
      // Optimistic update
      await queryClient.cancelQueries({
        queryKey: ["conversation", conversationId],
      });

      const previousConversation = queryClient.getQueryData([
        "conversation",
        conversationId,
      ]);

      queryClient.setQueryData(
        ["conversation", conversationId],
        (old: any) => ({
          ...old,
          messages: [
            ...old.messages,
            {
              id: `temp-${Date.now()}`,
              role: "user",
              content,
              created_at: new Date().toISOString(),
            },
          ],
        })
      );

      return { previousConversation };
    },
    onError: (err, content, context) => {
      // Rollback on error
      if (context?.previousConversation) {
        queryClient.setQueryData(
          ["conversation", conversationId],
          context.previousConversation
        );
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({
        queryKey: ["conversation", conversationId],
      });
    },
  });
}
```

### 6.3 Component Structure

```
src/components/query/
├── QueryPage.tsx                 # Page component
├── QueryInterface.tsx            # Main interface (refactored)
├── ConversationHeader.tsx        # Title, mode, actions
├── MessageThread.tsx             # Message list
├── MessageInput.tsx              # Query input
├── ConversationHistoryPanel/
│   ├── index.tsx                 # Panel container
│   ├── ConversationList.tsx      # Virtualized list
│   ├── ConversationItem.tsx      # Single item
│   ├── FilterBar.tsx             # Search + filters
│   └── Pagination.tsx            # Load more trigger
├── Message/
│   ├── index.tsx                 # Message wrapper
│   ├── UserMessage.tsx           # User bubble
│   ├── AssistantMessage.tsx      # AI bubble
│   ├── ThinkingSection.tsx       # COT display
│   └── SourceCitations.tsx       # Expandable sources
├── markdown/
│   ├── MarkdownRenderer.tsx      # Main renderer (refactored)
│   ├── TokenRenderer.tsx         # Token dispatcher
│   ├── StreamingParser.ts        # Buffer logic
│   └── tokens/
│       ├── ParagraphToken.tsx
│       ├── HeadingToken.tsx
│       ├── CodeToken.tsx
│       ├── TableToken.tsx
│       ├── ListToken.tsx
│       ├── BlockquoteToken.tsx
│       ├── InlineTokens.tsx
│       └── MermaidToken.tsx
└── shared/
    ├── Skeleton.tsx              # Loading states
    ├── ErrorBoundary.tsx         # Error handling
    └── SyncIndicator.tsx         # Save status
```

---

## 7. Performance Optimizations

### 7.1 Virtualized Conversation List

```typescript
// Using @tanstack/react-virtual
import { useVirtualizer } from "@tanstack/react-virtual";

function VirtualizedConversationList({ conversations }: Props) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: conversations.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 72, // Estimated item height
    overscan: 5, // Render 5 extra items above/below viewport
  });

  return (
    <div ref={parentRef} className="h-full overflow-auto">
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          position: "relative",
        }}
      >
        {virtualizer.getVirtualItems().map((virtualItem) => (
          <div
            key={virtualItem.key}
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              width: "100%",
              transform: `translateY(${virtualItem.start}px)`,
            }}
          >
            <ConversationItem conversation={conversations[virtualItem.index]} />
          </div>
        ))}
      </div>
    </div>
  );
}
```

### 7.2 Message Memoization

```typescript
// Memoize messages to prevent re-renders during streaming
const MemoizedMessage = memo(
  function Message({ message, isLast }: Props) {
    // ...
  },
  (prevProps, nextProps) => {
    // Only re-render if content changed or streaming state changed
    return (
      prevProps.message.id === nextProps.message.id &&
      prevProps.message.content === nextProps.message.content &&
      prevProps.message.isStreaming === nextProps.message.isStreaming &&
      prevProps.isLast === nextProps.isLast
    );
  }
);
```

### 7.3 Lazy Loading

```typescript
// Lazy load heavy components
const MermaidDiagram = lazy(() => import("./tokens/MermaidToken"));
const KatexRenderer = lazy(() => import("./tokens/KatexRenderer"));

// Usage with Suspense
<Suspense fallback={<Skeleton className="h-32" />}>
  <MermaidDiagram code={token.text} />
</Suspense>;
```

---

## 8. Next Steps

1. **Phase 4**: Create implementation roadmap → [04_implementation_roadmap.md](04_implementation_roadmap.md)
2. **Phase 5**: Design mockups → [05_design_mockups.md](05_design_mockups.md)

---

_Last updated: 2024-12-27_
