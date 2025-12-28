# Client-Side: Conversation API Client

**Document**: `08_client_api_client.md`  
**Created**: 2024-12-27  
**Status**: Draft

---

## 1. Overview

This document specifies the TypeScript API client for conversation persistence, enabling server-side storage, pagination, filtering, and sync.

### Goals

- Type-safe API client using existing patterns from `lib/api/client.ts`
- React Query integration for caching and optimistic updates
- Cursor-based pagination support
- Filter and search capabilities
- Import from localStorage migration

---

## 2. Type Definitions

### 2.1 Core Types

Add to `src/types/index.ts`:

```typescript
// ============================================================================
// Conversation Types
// ============================================================================

export type ConversationMode = "local" | "global" | "hybrid" | "naive" | "mix";

export interface Conversation {
  id: string;
  tenant_id: string;
  workspace_id?: string | null;
  user_id: string;
  title: string;
  mode: ConversationMode;
  is_pinned: boolean;
  is_archived: boolean;
  folder_id?: string | null;
  share_id?: string | null;
  message_count: number;
  last_message_preview?: string | null;
  meta: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface ConversationWithMessages extends Conversation {
  messages: Message[];
}

export interface Message {
  id: string;
  conversation_id: string;
  parent_id?: string | null;
  role: "user" | "assistant" | "system";
  content: string;
  mode?: ConversationMode | null;
  tokens_used?: number | null;
  duration_ms?: number | null;
  thinking_time_ms?: number | null;
  context?: MessageContext | null;
  is_error: boolean;
  created_at: string;
  updated_at: string;
}

export interface MessageContext {
  sources?: Source[];
  entities?: string[];
  relationships?: string[];
  thinking?: string;
}

export interface Source {
  id: string;
  title?: string;
  content: string;
  score: number;
}

export interface Folder {
  id: string;
  tenant_id: string;
  workspace_id?: string | null;
  user_id: string;
  name: string;
  parent_id?: string | null;
  position: number;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// Request/Response Types
// ============================================================================

export interface CreateConversationRequest {
  title?: string;
  mode?: ConversationMode;
  folder_id?: string | null;
}

export interface UpdateConversationRequest {
  title?: string;
  mode?: ConversationMode;
  is_pinned?: boolean;
  is_archived?: boolean;
  folder_id?: string | null;
}

export interface CreateMessageRequest {
  content: string;
  role: "user";
  parent_id?: string | null;
  stream?: boolean;
}

export interface UpdateMessageRequest {
  content?: string;
  tokens_used?: number;
  duration_ms?: number;
  thinking_time_ms?: number;
  context?: MessageContext;
  is_error?: boolean;
}

// ============================================================================
// Pagination Types
// ============================================================================

export interface CursorPaginationParams {
  cursor?: string;
  limit?: number;
}

export interface ConversationFilterParams {
  mode?: ConversationMode[];
  archived?: boolean;
  pinned?: boolean;
  folder_id?: string;
  search?: string;
  date_from?: string;
  date_to?: string;
  sort?: "updated_at" | "created_at" | "title";
  order?: "asc" | "desc";
}

export interface PaginatedConversations {
  items: Conversation[];
  pagination: PaginationMeta;
}

export interface PaginatedMessages {
  items: Message[];
  pagination: PaginationMeta;
}

export interface PaginationMeta {
  next_cursor?: string | null;
  prev_cursor?: string | null;
  total: number;
  has_more: boolean;
}

export interface ImportConversationsRequest {
  conversations: LocalStorageConversation[];
}

export interface LocalStorageConversation {
  id: string;
  title: string;
  messages: {
    id: string;
    role: "user" | "assistant";
    content: string;
    mode?: ConversationMode;
    tokensUsed?: number;
    durationMs?: number;
    thinkingTimeMs?: number;
    context?: MessageContext;
    isError?: boolean;
    timestamp?: number;
  }[];
  createdAt: number;
  updatedAt: number;
}

export interface ImportConversationsResponse {
  imported: number;
  failed: number;
  errors?: { id: string; error: string }[];
}

export interface ShareConversationResponse {
  share_id: string;
  share_url: string;
}
```

---

## 3. API Client Implementation

### 3.1 Conversations API

Create `src/lib/api/conversations.ts`:

```typescript
import type {
  Conversation,
  ConversationFilterParams,
  ConversationWithMessages,
  CreateConversationRequest,
  CreateMessageRequest,
  CursorPaginationParams,
  ImportConversationsRequest,
  ImportConversationsResponse,
  Message,
  PaginatedConversations,
  PaginatedMessages,
  ShareConversationResponse,
  UpdateConversationRequest,
  UpdateMessageRequest,
} from "@/types";
import { api } from "./client";

// ============================================================================
// Conversations
// ============================================================================

/**
 * List conversations with pagination and filtering.
 */
export async function listConversations(
  params?: CursorPaginationParams & ConversationFilterParams
): Promise<PaginatedConversations> {
  const searchParams = new URLSearchParams();

  if (params?.cursor) searchParams.set("cursor", params.cursor);
  if (params?.limit) searchParams.set("limit", String(params.limit));
  if (params?.mode?.length) {
    params.mode.forEach((m) => searchParams.append("filter[mode]", m));
  }
  if (params?.archived !== undefined) {
    searchParams.set("filter[archived]", String(params.archived));
  }
  if (params?.pinned !== undefined) {
    searchParams.set("filter[pinned]", String(params.pinned));
  }
  if (params?.folder_id)
    searchParams.set("filter[folder_id]", params.folder_id);
  if (params?.search) searchParams.set("filter[search]", params.search);
  if (params?.date_from)
    searchParams.set("filter[date_from]", params.date_from);
  if (params?.date_to) searchParams.set("filter[date_to]", params.date_to);
  if (params?.sort) searchParams.set("sort", params.sort);
  if (params?.order) searchParams.set("order", params.order);

  const query = searchParams.toString();
  return api.get<PaginatedConversations>(
    `/conversations${query ? `?${query}` : ""}`
  );
}

/**
 * Get a single conversation by ID (includes messages).
 */
export async function getConversation(
  conversationId: string
): Promise<ConversationWithMessages> {
  return api.get<ConversationWithMessages>(`/conversations/${conversationId}`);
}

/**
 * Create a new conversation.
 */
export async function createConversation(
  data: CreateConversationRequest
): Promise<Conversation> {
  return api.post<Conversation>("/conversations", data);
}

/**
 * Update a conversation.
 */
export async function updateConversation(
  conversationId: string,
  data: UpdateConversationRequest
): Promise<Conversation> {
  return api.patch<Conversation>(`/conversations/${conversationId}`, data);
}

/**
 * Delete a conversation.
 */
export async function deleteConversation(
  conversationId: string
): Promise<void> {
  return api.delete(`/conversations/${conversationId}`);
}

/**
 * Batch delete conversations.
 */
export async function deleteConversations(ids: string[]): Promise<void> {
  return api.post("/conversations/batch-delete", { ids });
}

// ============================================================================
// Messages
// ============================================================================

/**
 * List messages in a conversation.
 */
export async function listMessages(
  conversationId: string,
  params?: CursorPaginationParams
): Promise<PaginatedMessages> {
  const searchParams = new URLSearchParams();
  if (params?.cursor) searchParams.set("cursor", params.cursor);
  if (params?.limit) searchParams.set("limit", String(params.limit));

  const query = searchParams.toString();
  return api.get<PaginatedMessages>(
    `/conversations/${conversationId}/messages${query ? `?${query}` : ""}`
  );
}

/**
 * Add a message to a conversation.
 * Returns the user message immediately; AI response comes via streaming.
 */
export async function createMessage(
  conversationId: string,
  data: CreateMessageRequest
): Promise<Message> {
  return api.post<Message>(`/conversations/${conversationId}/messages`, data);
}

/**
 * Update a message (e.g., after streaming completes).
 */
export async function updateMessage(
  conversationId: string,
  messageId: string,
  data: UpdateMessageRequest
): Promise<Message> {
  return api.patch<Message>(
    `/conversations/${conversationId}/messages/${messageId}`,
    data
  );
}

// ============================================================================
// Sharing
// ============================================================================

/**
 * Generate a shareable link for a conversation.
 */
export async function shareConversation(
  conversationId: string
): Promise<ShareConversationResponse> {
  return api.post<ShareConversationResponse>(
    `/conversations/${conversationId}/share`
  );
}

/**
 * Remove the shareable link from a conversation.
 */
export async function unshareConversation(
  conversationId: string
): Promise<void> {
  return api.delete(`/conversations/${conversationId}/share`);
}

// ============================================================================
// Import
// ============================================================================

/**
 * Import conversations from localStorage.
 */
export async function importConversations(
  data: ImportConversationsRequest
): Promise<ImportConversationsResponse> {
  return api.post<ImportConversationsResponse>("/conversations/import", data);
}
```

### 3.2 Folders API

Create `src/lib/api/folders.ts`:

```typescript
import type { Folder } from "@/types";
import { api } from "./client";

export interface CreateFolderRequest {
  name: string;
  parent_id?: string | null;
}

export interface UpdateFolderRequest {
  name?: string;
  parent_id?: string | null;
  position?: number;
}

/**
 * List all folders for the current user.
 */
export async function listFolders(): Promise<Folder[]> {
  const response = await api.get<{ items: Folder[] }>("/folders");
  return response.items;
}

/**
 * Create a new folder.
 */
export async function createFolder(data: CreateFolderRequest): Promise<Folder> {
  return api.post<Folder>("/folders", data);
}

/**
 * Update a folder.
 */
export async function updateFolder(
  folderId: string,
  data: UpdateFolderRequest
): Promise<Folder> {
  return api.patch<Folder>(`/folders/${folderId}`, data);
}

/**
 * Delete a folder.
 */
export async function deleteFolder(folderId: string): Promise<void> {
  return api.delete(`/folders/${folderId}`);
}
```

---

## 4. React Query Hooks

### 4.1 Query Keys

Create `src/lib/api/query-keys.ts`:

```typescript
export const conversationKeys = {
  all: ["conversations"] as const,
  lists: () => [...conversationKeys.all, "list"] as const,
  list: (filters: Record<string, unknown>) =>
    [...conversationKeys.lists(), filters] as const,
  details: () => [...conversationKeys.all, "detail"] as const,
  detail: (id: string) => [...conversationKeys.details(), id] as const,
  messages: (id: string) =>
    [...conversationKeys.detail(id), "messages"] as const,
};

export const folderKeys = {
  all: ["folders"] as const,
  list: () => [...folderKeys.all, "list"] as const,
};
```

### 4.2 Conversation Hooks

Create `src/hooks/use-conversations.ts`:

```typescript
"use client";

import {
  useQuery,
  useMutation,
  useQueryClient,
  useInfiniteQuery,
  type InfiniteData,
} from "@tanstack/react-query";
import {
  listConversations,
  getConversation,
  createConversation,
  updateConversation,
  deleteConversation,
  createMessage,
  updateMessage,
  importConversations,
  shareConversation,
  unshareConversation,
} from "@/lib/api/conversations";
import { conversationKeys } from "@/lib/api/query-keys";
import type {
  Conversation,
  ConversationFilterParams,
  ConversationWithMessages,
  CreateConversationRequest,
  CreateMessageRequest,
  Message,
  PaginatedConversations,
  UpdateConversationRequest,
  UpdateMessageRequest,
} from "@/types";
import { toast } from "sonner";

// ============================================================================
// List Conversations (Infinite Query)
// ============================================================================

export function useConversations(filters?: ConversationFilterParams) {
  return useInfiniteQuery({
    queryKey: conversationKeys.list(filters ?? {}),
    queryFn: async ({ pageParam }) => {
      return listConversations({
        cursor: pageParam as string | undefined,
        limit: 20,
        ...filters,
      });
    },
    initialPageParam: undefined as string | undefined,
    getNextPageParam: (lastPage) =>
      lastPage.pagination.has_more
        ? lastPage.pagination.next_cursor
        : undefined,
    staleTime: 30_000, // 30 seconds
  });
}

// ============================================================================
// Single Conversation
// ============================================================================

export function useConversation(conversationId: string | null) {
  return useQuery({
    queryKey: conversationKeys.detail(conversationId ?? ""),
    queryFn: () => getConversation(conversationId!),
    enabled: !!conversationId,
    staleTime: 60_000, // 1 minute
  });
}

// ============================================================================
// Mutations
// ============================================================================

export function useCreateConversation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateConversationRequest) => createConversation(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.lists() });
    },
  });
}

export function useUpdateConversation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      id,
      data,
    }: {
      id: string;
      data: UpdateConversationRequest;
    }) => updateConversation(id, data),
    onMutate: async ({ id, data }) => {
      // Optimistic update
      await queryClient.cancelQueries({
        queryKey: conversationKeys.detail(id),
      });

      const previousConversation =
        queryClient.getQueryData<ConversationWithMessages>(
          conversationKeys.detail(id)
        );

      if (previousConversation) {
        queryClient.setQueryData(conversationKeys.detail(id), {
          ...previousConversation,
          ...data,
        });
      }

      return { previousConversation };
    },
    onError: (err, { id }, context) => {
      if (context?.previousConversation) {
        queryClient.setQueryData(
          conversationKeys.detail(id),
          context.previousConversation
        );
      }
      toast.error("Failed to update conversation");
    },
    onSettled: (_, __, { id }) => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: conversationKeys.lists() });
    },
  });
}

export function useDeleteConversation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => deleteConversation(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.lists() });
      toast.success("Conversation deleted");
    },
    onError: () => {
      toast.error("Failed to delete conversation");
    },
  });
}

// ============================================================================
// Message Mutations
// ============================================================================

export function useCreateMessage(conversationId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateMessageRequest) =>
      createMessage(conversationId, data),
    onMutate: async (data) => {
      // Optimistic: add user message immediately
      const optimisticMessage: Message = {
        id: `temp-${Date.now()}`,
        conversation_id: conversationId,
        role: "user",
        content: data.content,
        is_error: false,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      await queryClient.cancelQueries({
        queryKey: conversationKeys.detail(conversationId),
      });

      const previous = queryClient.getQueryData<ConversationWithMessages>(
        conversationKeys.detail(conversationId)
      );

      if (previous) {
        queryClient.setQueryData(conversationKeys.detail(conversationId), {
          ...previous,
          messages: [...previous.messages, optimisticMessage],
        });
      }

      return { previous, optimisticMessage };
    },
    onError: (err, _, context) => {
      if (context?.previous) {
        queryClient.setQueryData(
          conversationKeys.detail(conversationId),
          context.previous
        );
      }
      toast.error("Failed to send message");
    },
    onSettled: () => {
      queryClient.invalidateQueries({
        queryKey: conversationKeys.detail(conversationId),
      });
    },
  });
}

export function useUpdateMessage(conversationId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      messageId,
      data,
    }: {
      messageId: string;
      data: UpdateMessageRequest;
    }) => updateMessage(conversationId, messageId, data),
    onSettled: () => {
      queryClient.invalidateQueries({
        queryKey: conversationKeys.detail(conversationId),
      });
    },
  });
}

// ============================================================================
// Sharing
// ============================================================================

export function useShareConversation() {
  return useMutation({
    mutationFn: (id: string) => shareConversation(id),
    onSuccess: (data) => {
      navigator.clipboard.writeText(data.share_url);
      toast.success("Share link copied to clipboard");
    },
    onError: () => {
      toast.error("Failed to generate share link");
    },
  });
}

export function useUnshareConversation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => unshareConversation(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.detail(id) });
      toast.success("Share link removed");
    },
  });
}

// ============================================================================
// Import
// ============================================================================

export function useImportConversations() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: importConversations,
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.lists() });
      toast.success(
        `Imported ${result.imported} conversations${
          result.failed > 0 ? `, ${result.failed} failed` : ""
        }`
      );
    },
    onError: () => {
      toast.error("Failed to import conversations");
    },
  });
}
```

---

## 5. Export from lib/api

Update `src/lib/api/edgequake.ts` to re-export:

```typescript
// ... existing exports ...

// Conversations
export * from "./conversations";
export * from "./folders";
```

---

## 6. Usage Examples

### 6.1 List Conversations with Filters

```tsx
function ConversationList() {
  const { data, fetchNextPage, hasNextPage, isFetchingNextPage } =
    useConversations({
      archived: false,
      sort: "updated_at",
      order: "desc",
    });

  const allConversations = data?.pages.flatMap((page) => page.items) ?? [];

  return (
    <div>
      {allConversations.map((conv) => (
        <ConversationItem key={conv.id} conversation={conv} />
      ))}
      {hasNextPage && (
        <button onClick={() => fetchNextPage()} disabled={isFetchingNextPage}>
          {isFetchingNextPage ? "Loading..." : "Load more"}
        </button>
      )}
    </div>
  );
}
```

### 6.2 Create New Conversation

```tsx
function NewConversationButton() {
  const createMutation = useCreateConversation();

  const handleClick = async () => {
    const conversation = await createMutation.mutateAsync({
      title: "New Chat",
      mode: "hybrid",
    });
    // Navigate to new conversation
    router.push(`/query?conversation=${conversation.id}`);
  };

  return (
    <Button onClick={handleClick} disabled={createMutation.isPending}>
      <Plus className="h-4 w-4 mr-2" />
      New Chat
    </Button>
  );
}
```

### 6.3 Send Message with Streaming

```tsx
function ChatInput({ conversationId }: { conversationId: string }) {
  const createMessage = useCreateMessage(conversationId);
  const [input, setInput] = useState("");

  const handleSubmit = async () => {
    await createMessage.mutateAsync({
      content: input,
      role: "user",
      stream: true,
    });
    setInput("");
    // Streaming response handled via WebSocket/SSE separately
  };

  return (
    <form onSubmit={handleSubmit}>
      <Textarea value={input} onChange={(e) => setInput(e.target.value)} />
      <Button type="submit" disabled={createMessage.isPending}>
        Send
      </Button>
    </form>
  );
}
```

---

## 7. Testing Checklist

| Test                      | Expected Result                     |
| ------------------------- | ----------------------------------- |
| List conversations        | Returns paginated results           |
| Create conversation       | Creates and invalidates list        |
| Update title (optimistic) | UI updates immediately              |
| Delete conversation       | Removes from list                   |
| Send message              | User message appears optimistically |
| Import from localStorage  | Migrates conversations              |
| Share conversation        | Generates link, copies to clipboard |
| Infinite scroll           | Loads more pages on scroll          |

---

_Last updated: 2024-12-27_
