# Client-Side: State Management Architecture

**Document**: `09_client_state_management.md`  
**Created**: 2024-12-27  
**Status**: Draft

---

## 1. Overview

This document defines the state management architecture for conversation persistence, combining Zustand for local UI state with React Query for server state.

### Current State

| Store                       | Type              | Issues                     |
| --------------------------- | ----------------- | -------------------------- |
| `use-conversation-store.ts` | Zustand + persist | localStorage only, no sync |
| `use-settings-store.ts`     | Zustand + persist | OK, keep as-is             |
| `use-tenant-store.ts`       | Zustand           | OK, keep as-is             |

### Target Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    STATE ARCHITECTURE                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  SERVER STATE (React Query)          UI STATE (Zustand)          │
│  ┌─────────────────────────┐        ┌─────────────────────────┐ │
│  │ Conversations List      │        │ Active Conversation ID  │ │
│  │ (paginated, cached)     │        │ History Panel Open      │ │
│  │                         │        │ Streaming State         │ │
│  │ Single Conversation     │        │ Pending Message         │ │
│  │ (with messages)         │        │ Filter State            │ │
│  │                         │        │ Sort State              │ │
│  │ Folders                 │        │ Selected Items          │ │
│  └─────────────────────────┘        └─────────────────────────┘ │
│            ▲                                    ▲                │
│            │                                    │                │
│            └──────────── Combined ──────────────┘                │
│                              │                                   │
│                              ▼                                   │
│                    useQueryPageState()                           │
│                    (unified hook)                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Zustand Store Refactor

### 2.1 New Query UI Store

Replace `use-conversation-store.ts` with a lightweight UI-only store:

```typescript
// src/stores/use-query-ui-store.ts
"use client";

import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { ConversationMode } from "@/types";

// ============================================================================
// Types
// ============================================================================

export type StreamingState =
  | "idle"
  | "thinking"
  | "generating"
  | "complete"
  | "error";

export interface PendingMessage {
  id: string;
  content: string;
  thinkingContent?: string;
  tokensGenerated: number;
  startTime: number;
  thinkingDuration?: number;
}

export interface ConversationFilters {
  mode: ConversationMode[] | null;
  archived: boolean;
  pinned: boolean | null;
  folderId: string | null;
  search: string;
  dateFrom: string | null;
  dateTo: string | null;
}

export interface ConversationSort {
  field: "updated_at" | "created_at" | "title";
  order: "asc" | "desc";
}

// ============================================================================
// Store State
// ============================================================================

interface QueryUIState {
  // Active conversation
  activeConversationId: string | null;

  // Panel state
  historyPanelOpen: boolean;

  // Streaming state
  streamingState: StreamingState;
  pendingMessage: PendingMessage | null;
  abortController: AbortController | null;

  // Filters & sort
  filters: ConversationFilters;
  sort: ConversationSort;

  // Selection (for batch operations)
  selectedIds: Set<string>;
  isSelectionMode: boolean;
}

interface QueryUIActions {
  // Active conversation
  setActiveConversation: (id: string | null) => void;

  // Panel
  setHistoryPanelOpen: (open: boolean) => void;
  toggleHistoryPanel: () => void;

  // Streaming
  startStreaming: (conversationId: string) => AbortController;
  updateStreamingState: (state: StreamingState) => void;
  setPendingMessage: (message: PendingMessage | null) => void;
  appendToPendingMessage: (content: string) => void;
  setThinkingContent: (content: string) => void;
  completeStreaming: () => void;
  abortStreaming: () => void;

  // Filters
  setFilters: (filters: Partial<ConversationFilters>) => void;
  resetFilters: () => void;
  setSort: (sort: ConversationSort) => void;

  // Selection
  toggleSelection: (id: string) => void;
  selectAll: (ids: string[]) => void;
  clearSelection: () => void;
  setSelectionMode: (mode: boolean) => void;

  // Reset
  reset: () => void;
}

type QueryUIStore = QueryUIState & QueryUIActions;

// ============================================================================
// Default Values
// ============================================================================

const defaultFilters: ConversationFilters = {
  mode: null,
  archived: false,
  pinned: null,
  folderId: null,
  search: "",
  dateFrom: null,
  dateTo: null,
};

const defaultSort: ConversationSort = {
  field: "updated_at",
  order: "desc",
};

const defaultState: QueryUIState = {
  activeConversationId: null,
  historyPanelOpen: true,
  streamingState: "idle",
  pendingMessage: null,
  abortController: null,
  filters: defaultFilters,
  sort: defaultSort,
  selectedIds: new Set(),
  isSelectionMode: false,
};

// ============================================================================
// Store Implementation
// ============================================================================

export const useQueryUIStore = create<QueryUIStore>()(
  persist(
    (set, get) => ({
      ...defaultState,

      // Active conversation
      setActiveConversation: (id) => {
        set({ activeConversationId: id });
      },

      // Panel
      setHistoryPanelOpen: (open) => set({ historyPanelOpen: open }),
      toggleHistoryPanel: () =>
        set((state) => ({ historyPanelOpen: !state.historyPanelOpen })),

      // Streaming
      startStreaming: (conversationId) => {
        const controller = new AbortController();
        set({
          activeConversationId: conversationId,
          streamingState: "thinking",
          abortController: controller,
          pendingMessage: {
            id: `pending-${Date.now()}`,
            content: "",
            tokensGenerated: 0,
            startTime: Date.now(),
          },
        });
        return controller;
      },

      updateStreamingState: (state) => set({ streamingState: state }),

      setPendingMessage: (message) => set({ pendingMessage: message }),

      appendToPendingMessage: (content) => {
        set((state) => {
          if (!state.pendingMessage) return state;
          return {
            pendingMessage: {
              ...state.pendingMessage,
              content: state.pendingMessage.content + content,
              tokensGenerated: state.pendingMessage.tokensGenerated + 1,
            },
            streamingState: "generating",
          };
        });
      },

      setThinkingContent: (content) => {
        set((state) => {
          if (!state.pendingMessage) return state;
          return {
            pendingMessage: {
              ...state.pendingMessage,
              thinkingContent: content,
              thinkingDuration: Date.now() - state.pendingMessage.startTime,
            },
          };
        });
      },

      completeStreaming: () => {
        set({
          streamingState: "complete",
          pendingMessage: null,
          abortController: null,
        });
      },

      abortStreaming: () => {
        const { abortController } = get();
        abortController?.abort();
        set({
          streamingState: "idle",
          pendingMessage: null,
          abortController: null,
        });
      },

      // Filters
      setFilters: (filters) =>
        set((state) => ({
          filters: { ...state.filters, ...filters },
        })),

      resetFilters: () => set({ filters: defaultFilters }),

      setSort: (sort) => set({ sort }),

      // Selection
      toggleSelection: (id) => {
        set((state) => {
          const newSet = new Set(state.selectedIds);
          if (newSet.has(id)) {
            newSet.delete(id);
          } else {
            newSet.add(id);
          }
          return { selectedIds: newSet };
        });
      },

      selectAll: (ids) => set({ selectedIds: new Set(ids) }),

      clearSelection: () =>
        set({ selectedIds: new Set(), isSelectionMode: false }),

      setSelectionMode: (mode) => set({ isSelectionMode: mode }),

      // Reset
      reset: () => set(defaultState),
    }),
    {
      name: "edgequake-query-ui",
      partialize: (state) => ({
        // Only persist these fields
        historyPanelOpen: state.historyPanelOpen,
        activeConversationId: state.activeConversationId,
        filters: state.filters,
        sort: state.sort,
      }),
    }
  )
);
```

### 2.2 Derived Selectors

```typescript
// src/stores/use-query-ui-store.ts (continued)

// ============================================================================
// Derived Selectors
// ============================================================================

export const useActiveConversationId = () =>
  useQueryUIStore((state) => state.activeConversationId);

export const useHistoryPanelOpen = () =>
  useQueryUIStore((state) => state.historyPanelOpen);

export const useStreamingState = () =>
  useQueryUIStore((state) => ({
    state: state.streamingState,
    pendingMessage: state.pendingMessage,
    isStreaming:
      state.streamingState !== "idle" && state.streamingState !== "complete",
  }));

export const useConversationFilters = () =>
  useQueryUIStore((state) => ({
    filters: state.filters,
    sort: state.sort,
    setFilters: state.setFilters,
    resetFilters: state.resetFilters,
    setSort: state.setSort,
  }));

export const useConversationSelection = () =>
  useQueryUIStore((state) => ({
    selectedIds: state.selectedIds,
    isSelectionMode: state.isSelectionMode,
    toggleSelection: state.toggleSelection,
    selectAll: state.selectAll,
    clearSelection: state.clearSelection,
    setSelectionMode: state.setSelectionMode,
    selectedCount: state.selectedIds.size,
  }));
```

---

## 3. Unified Query Page Hook

### 3.1 useQueryPageState

Combines UI state with React Query server state:

```typescript
// src/hooks/use-query-page-state.ts
"use client";

import { useMemo, useCallback } from "react";
import {
  useQueryUIStore,
  useActiveConversationId,
  useStreamingState,
} from "@/stores/use-query-ui-store";
import {
  useConversation,
  useConversations,
  useCreateConversation,
  useCreateMessage,
} from "./use-conversations";
import type { Conversation, ConversationWithMessages, Message } from "@/types";

interface QueryPageState {
  // Current conversation
  conversation: ConversationWithMessages | null;
  isLoadingConversation: boolean;

  // Messages (including pending)
  messages: Message[];

  // Streaming
  isStreaming: boolean;
  streamingState: ReturnType<typeof useStreamingState>;

  // Conversation list
  conversations: Conversation[];
  isLoadingList: boolean;
  hasMoreConversations: boolean;
  loadMoreConversations: () => void;

  // Actions
  createNewConversation: () => Promise<string>;
  sendMessage: (content: string) => Promise<void>;
  switchConversation: (id: string) => void;

  // Panel state
  historyPanelOpen: boolean;
  toggleHistoryPanel: () => void;
}

export function useQueryPageState(): QueryPageState {
  const store = useQueryUIStore();
  const activeId = useActiveConversationId();
  const streamingState = useStreamingState();

  // Server state
  const { data: conversationData, isLoading: isLoadingConversation } =
    useConversation(activeId);

  const {
    data: conversationsData,
    isLoading: isLoadingList,
    fetchNextPage,
    hasNextPage,
  } = useConversations({
    archived: store.filters.archived,
    mode: store.filters.mode ?? undefined,
    pinned: store.filters.pinned ?? undefined,
    folder_id: store.filters.folderId ?? undefined,
    search: store.filters.search || undefined,
    date_from: store.filters.dateFrom ?? undefined,
    date_to: store.filters.dateTo ?? undefined,
    sort: store.sort.field,
    order: store.sort.order,
  });

  // Mutations
  const createConversationMutation = useCreateConversation();
  const createMessageMutation = useCreateMessage(activeId ?? "");

  // Flatten paginated conversations
  const conversations = useMemo(() => {
    return conversationsData?.pages.flatMap((page) => page.items) ?? [];
  }, [conversationsData]);

  // Combine real messages with pending message
  const messages = useMemo(() => {
    const realMessages = conversationData?.messages ?? [];

    if (streamingState.pendingMessage && streamingState.isStreaming) {
      // Add pending assistant message
      const pendingAssistantMessage: Message = {
        id: streamingState.pendingMessage.id,
        conversation_id: activeId ?? "",
        role: "assistant",
        content: streamingState.pendingMessage.content,
        is_error: false,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        context: streamingState.pendingMessage.thinkingContent
          ? { thinking: streamingState.pendingMessage.thinkingContent }
          : undefined,
      };
      return [...realMessages, pendingAssistantMessage];
    }

    return realMessages;
  }, [conversationData?.messages, streamingState, activeId]);

  // Actions
  const createNewConversation = useCallback(async () => {
    const conversation = await createConversationMutation.mutateAsync({
      mode: "hybrid",
    });
    store.setActiveConversation(conversation.id);
    return conversation.id;
  }, [createConversationMutation, store]);

  const sendMessage = useCallback(
    async (content: string) => {
      if (!activeId) {
        // Create conversation first
        const newId = await createNewConversation();
        store.setActiveConversation(newId);
      }

      await createMessageMutation.mutateAsync({
        content,
        role: "user",
        stream: true,
      });
    },
    [activeId, createNewConversation, createMessageMutation, store]
  );

  const switchConversation = useCallback(
    (id: string) => {
      if (streamingState.isStreaming) {
        store.abortStreaming();
      }
      store.setActiveConversation(id);
    },
    [store, streamingState.isStreaming]
  );

  return {
    conversation: conversationData ?? null,
    isLoadingConversation,
    messages,
    isStreaming: streamingState.isStreaming,
    streamingState,
    conversations,
    isLoadingList,
    hasMoreConversations: hasNextPage ?? false,
    loadMoreConversations: fetchNextPage,
    createNewConversation,
    sendMessage,
    switchConversation,
    historyPanelOpen: store.historyPanelOpen,
    toggleHistoryPanel: store.toggleHistoryPanel,
  };
}
```

---

## 4. Migration from localStorage

### 4.1 Migration Hook

```typescript
// src/hooks/use-migrate-conversations.ts
"use client";

import { useEffect, useState } from "react";
import { useImportConversations } from "./use-conversations";
import type { LocalStorageConversation } from "@/types";

const MIGRATION_KEY = "edgequake-conversations-migrated";

interface MigrationState {
  status: "pending" | "checking" | "migrating" | "complete" | "error";
  progress: number;
  total: number;
  error?: string;
}

export function useMigrateConversations() {
  const [state, setState] = useState<MigrationState>({
    status: "pending",
    progress: 0,
    total: 0,
  });

  const importMutation = useImportConversations();

  useEffect(() => {
    const checkAndMigrate = async () => {
      // Check if already migrated
      if (typeof window === "undefined") return;
      if (localStorage.getItem(MIGRATION_KEY)) {
        setState({ status: "complete", progress: 0, total: 0 });
        return;
      }

      setState({ status: "checking", progress: 0, total: 0 });

      // Check for old conversations
      const oldData = localStorage.getItem("edgequake-conversations");
      if (!oldData) {
        localStorage.setItem(MIGRATION_KEY, "true");
        setState({ status: "complete", progress: 0, total: 0 });
        return;
      }

      try {
        const parsed = JSON.parse(oldData);
        const conversations: LocalStorageConversation[] =
          parsed.state?.conversations ?? [];

        if (conversations.length === 0) {
          localStorage.setItem(MIGRATION_KEY, "true");
          setState({ status: "complete", progress: 0, total: 0 });
          return;
        }

        setState({
          status: "migrating",
          progress: 0,
          total: conversations.length,
        });

        // Import in batches of 10
        const batchSize = 10;
        for (let i = 0; i < conversations.length; i += batchSize) {
          const batch = conversations.slice(i, i + batchSize);
          await importMutation.mutateAsync({ conversations: batch });
          setState((prev) => ({
            ...prev,
            progress: Math.min(i + batchSize, conversations.length),
          }));
        }

        // Mark as migrated
        localStorage.setItem(MIGRATION_KEY, "true");
        // Optionally clear old data
        // localStorage.removeItem('edgequake-conversations');

        setState({
          status: "complete",
          progress: conversations.length,
          total: conversations.length,
        });
      } catch (error) {
        setState({
          status: "error",
          progress: 0,
          total: 0,
          error: error instanceof Error ? error.message : "Unknown error",
        });
      }
    };

    checkAndMigrate();
  }, [importMutation]);

  return state;
}
```

### 4.2 Migration UI Component

```typescript
// src/components/query/MigrationBanner.tsx
"use client";

import { useMigrateConversations } from "@/hooks/use-migrate-conversations";
import { Progress } from "@/components/ui/progress";
import { AlertCircle, CheckCircle, Loader2 } from "lucide-react";

export function MigrationBanner() {
  const migration = useMigrateConversations();

  if (migration.status === "complete" || migration.status === "pending") {
    return null;
  }

  if (migration.status === "checking") {
    return (
      <div className="bg-muted/50 border-b border-border px-4 py-2 flex items-center gap-2">
        <Loader2 className="h-4 w-4 animate-spin" />
        <span className="text-sm">
          Checking for conversations to migrate...
        </span>
      </div>
    );
  }

  if (migration.status === "migrating") {
    const percent = Math.round((migration.progress / migration.total) * 100);
    return (
      <div className="bg-primary/10 border-b border-primary/20 px-4 py-3">
        <div className="flex items-center gap-2 mb-2">
          <Loader2 className="h-4 w-4 animate-spin text-primary" />
          <span className="text-sm font-medium">
            Migrating conversations... ({migration.progress}/{migration.total})
          </span>
        </div>
        <Progress value={percent} className="h-2" />
      </div>
    );
  }

  if (migration.status === "error") {
    return (
      <div className="bg-destructive/10 border-b border-destructive/20 px-4 py-2 flex items-center gap-2">
        <AlertCircle className="h-4 w-4 text-destructive" />
        <span className="text-sm text-destructive">
          Migration failed: {migration.error}
        </span>
      </div>
    );
  }

  return null;
}
```

---

## 5. Provider Setup

### 5.1 QueryProvider

```typescript
// src/providers/QueryProvider.tsx
"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { useState, type ReactNode } from "react";

interface QueryProviderProps {
  children: ReactNode;
}

export function QueryProvider({ children }: QueryProviderProps) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 30_000, // 30 seconds
            refetchOnWindowFocus: false,
          },
        },
      })
  );

  return (
    <QueryClientProvider client={queryClient}>
      {children}
      <ReactQueryDevtools initialIsOpen={false} />
    </QueryClientProvider>
  );
}
```

---

## 6. File Structure

```
src/
├── stores/
│   ├── use-query-ui-store.ts      # UI state (replaces use-conversation-store)
│   ├── use-settings-store.ts      # Keep existing
│   └── use-tenant-store.ts        # Keep existing
├── hooks/
│   ├── use-conversations.ts       # React Query hooks
│   ├── use-query-page-state.ts    # Unified hook
│   └── use-migrate-conversations.ts
├── lib/api/
│   ├── conversations.ts           # API functions
│   ├── folders.ts
│   └── query-keys.ts
└── providers/
    └── QueryProvider.tsx
```

---

## 7. Migration Checklist

| Step | Action                                        |
| ---- | --------------------------------------------- |
| 1    | Install `@tanstack/react-query`               |
| 2    | Create `QueryProvider` and add to layout      |
| 3    | Create new API client files                   |
| 4    | Create `use-query-ui-store.ts`                |
| 5    | Create React Query hooks                      |
| 6    | Create `useQueryPageState`                    |
| 7    | Update `query-interface.tsx` to use new hooks |
| 8    | Add migration banner component                |
| 9    | Test migration from localStorage              |
| 10   | Remove old `use-conversation-store.ts`        |

---

_Last updated: 2024-12-27_
