# Client-Side: History Panel & Components

**Document**: `10_client_history_panel.md`  
**Created**: 2024-12-27  
**Status**: Draft

---

## 1. Overview

This document specifies the refactored Conversation History Panel with server-side pagination, filtering, virtualization, and batch operations.

### Current Issues

| Issue                      | Impact                           |
| -------------------------- | -------------------------------- |
| Client-side filtering only | Slow with many conversations     |
| No virtualization          | Poor performance with 100+ items |
| No batch operations        | Can't delete/archive multiple    |
| No folders                 | Limited organization             |

### Target Features

- Server-side pagination and filtering
- Virtualized list for smooth scrolling
- Folder organization with drag-drop
- Batch selection and operations
- Pin/archive functionality

---

## 2. Component Architecture

### 2.1 Component Hierarchy

```
ConversationHistoryPanel
├── PanelHeader
│   ├── NewConversationButton
│   └── CollapseButton
├── SearchBar
│   └── SearchInput
├── FilterBar
│   ├── DateRangeFilter
│   ├── ModeFilter
│   └── ClearFiltersButton
├── FolderTree (optional, P2)
│   └── FolderItem[]
├── ConversationList
│   ├── PinnedSection
│   │   └── ConversationItem[]
│   ├── RegularSection
│   │   └── VirtualizedList
│   │       └── ConversationItem[]
│   └── LoadMoreTrigger
└── SelectionToolbar (when items selected)
    ├── SelectCount
    ├── ArchiveButton
    ├── DeleteButton
    └── MoveToFolderButton
```

### 2.2 File Structure

```
src/components/query/history/
├── index.ts
├── ConversationHistoryPanel.tsx
├── PanelHeader.tsx
├── SearchBar.tsx
├── FilterBar.tsx
├── ConversationList.tsx
├── ConversationItem.tsx
├── SelectionToolbar.tsx
├── LoadingState.tsx
└── EmptyState.tsx
```

---

## 3. Core Components

### 3.1 ConversationHistoryPanel

Main container with responsive layout:

```typescript
// src/components/query/history/ConversationHistoryPanel.tsx
"use client";

import { memo } from "react";
import { cn } from "@/lib/utils";
import {
  useQueryUIStore,
  useHistoryPanelOpen,
} from "@/stores/use-query-ui-store";
import { PanelHeader } from "./PanelHeader";
import { SearchBar } from "./SearchBar";
import { FilterBar } from "./FilterBar";
import { ConversationList } from "./ConversationList";
import { SelectionToolbar } from "./SelectionToolbar";

interface ConversationHistoryPanelProps {
  className?: string;
}

export const ConversationHistoryPanel = memo(function ConversationHistoryPanel({
  className,
}: ConversationHistoryPanelProps) {
  const isOpen = useHistoryPanelOpen();
  const { isSelectionMode, selectedCount } = useQueryUIStore((s) => ({
    isSelectionMode: s.isSelectionMode,
    selectedCount: s.selectedIds.size,
  }));

  return (
    <aside
      className={cn(
        "flex flex-col h-full bg-background border-r border-border",
        "transition-all duration-300 ease-in-out",
        isOpen ? "w-80" : "w-0 overflow-hidden",
        className
      )}
    >
      <PanelHeader />

      <div className="flex-1 flex flex-col min-h-0 p-3 space-y-3">
        <SearchBar />
        <FilterBar />
        <ConversationList />
      </div>

      {isSelectionMode && selectedCount > 0 && <SelectionToolbar />}
    </aside>
  );
});

export default ConversationHistoryPanel;
```

### 3.2 PanelHeader

```typescript
// src/components/query/history/PanelHeader.tsx
"use client";

import { memo } from "react";
import { Button } from "@/components/ui/button";
import { Plus, PanelLeftClose, PanelLeft } from "lucide-react";
import { useQueryUIStore } from "@/stores/use-query-ui-store";
import { useCreateConversation } from "@/hooks/use-conversations";

export const PanelHeader = memo(function PanelHeader() {
  const { historyPanelOpen, toggleHistoryPanel, setActiveConversation } =
    useQueryUIStore();
  const createMutation = useCreateConversation();

  const handleNewChat = async () => {
    const conversation = await createMutation.mutateAsync({ mode: "hybrid" });
    setActiveConversation(conversation.id);
  };

  return (
    <div className="flex items-center justify-between p-3 border-b border-border">
      <Button
        onClick={handleNewChat}
        disabled={createMutation.isPending}
        className="flex-1 mr-2"
      >
        <Plus className="h-4 w-4 mr-2" />
        New Chat
      </Button>

      <Button
        variant="ghost"
        size="icon"
        onClick={toggleHistoryPanel}
        className="shrink-0"
        aria-label={historyPanelOpen ? "Collapse panel" : "Expand panel"}
      >
        {historyPanelOpen ? (
          <PanelLeftClose className="h-4 w-4" />
        ) : (
          <PanelLeft className="h-4 w-4" />
        )}
      </Button>
    </div>
  );
});
```

### 3.3 SearchBar

```typescript
// src/components/query/history/SearchBar.tsx
"use client";

import { memo, useState, useCallback, useEffect } from "react";
import { Input } from "@/components/ui/input";
import { Search, X } from "lucide-react";
import { useQueryUIStore } from "@/stores/use-query-ui-store";
import { useDebouncedCallback } from "use-debounce";

export const SearchBar = memo(function SearchBar() {
  const { filters, setFilters } = useQueryUIStore();
  const [localValue, setLocalValue] = useState(filters.search);

  // Debounce search to avoid too many API calls
  const debouncedSearch = useDebouncedCallback((value: string) => {
    setFilters({ search: value });
  }, 300);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const value = e.target.value;
      setLocalValue(value);
      debouncedSearch(value);
    },
    [debouncedSearch]
  );

  const handleClear = useCallback(() => {
    setLocalValue("");
    setFilters({ search: "" });
  }, [setFilters]);

  // Sync with external changes
  useEffect(() => {
    setLocalValue(filters.search);
  }, [filters.search]);

  return (
    <div className="relative">
      <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
      <Input
        value={localValue}
        onChange={handleChange}
        placeholder="Search conversations..."
        className="pl-9 pr-9"
      />
      {localValue && (
        <button
          onClick={handleClear}
          className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          aria-label="Clear search"
        >
          <X className="h-4 w-4" />
        </button>
      )}
    </div>
  );
});
```

### 3.4 FilterBar

```typescript
// src/components/query/history/FilterBar.tsx
"use client";

import { memo, useCallback } from "react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuCheckboxItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Calendar } from "@/components/ui/calendar";
import { Badge } from "@/components/ui/badge";
import { Calendar as CalendarIcon, Filter, X } from "lucide-react";
import { useQueryUIStore } from "@/stores/use-query-ui-store";
import { format } from "date-fns";
import type { ConversationMode } from "@/types";

const MODES: { value: ConversationMode; label: string; color: string }[] = [
  { value: "local", label: "Local", color: "bg-blue-500" },
  { value: "global", label: "Global", color: "bg-green-500" },
  { value: "hybrid", label: "Hybrid", color: "bg-purple-500" },
  { value: "naive", label: "Naive", color: "bg-gray-500" },
];

export const FilterBar = memo(function FilterBar() {
  const { filters, setFilters, resetFilters } = useQueryUIStore();

  const hasActiveFilters =
    filters.mode !== null ||
    filters.pinned !== null ||
    filters.dateFrom !== null ||
    filters.dateTo !== null;

  const handleModeToggle = useCallback(
    (mode: ConversationMode) => {
      const currentModes = filters.mode ?? [];
      const newModes = currentModes.includes(mode)
        ? currentModes.filter((m) => m !== mode)
        : [...currentModes, mode];

      setFilters({ mode: newModes.length > 0 ? newModes : null });
    },
    [filters.mode, setFilters]
  );

  const handleDateSelect = useCallback(
    (key: "dateFrom" | "dateTo", date: Date | undefined) => {
      setFilters({ [key]: date ? format(date, "yyyy-MM-dd") : null });
    },
    [setFilters]
  );

  return (
    <div className="flex items-center gap-2">
      {/* Mode Filter */}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm" className="h-8">
            <Filter className="h-3 w-3 mr-1" />
            Mode
            {filters.mode && filters.mode.length > 0 && (
              <Badge variant="secondary" className="ml-1 h-4 px-1 text-[10px]">
                {filters.mode.length}
              </Badge>
            )}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          {MODES.map(({ value, label, color }) => (
            <DropdownMenuCheckboxItem
              key={value}
              checked={filters.mode?.includes(value) ?? false}
              onCheckedChange={() => handleModeToggle(value)}
            >
              <div className="flex items-center gap-2">
                <div className={`w-2 h-2 rounded-full ${color}`} />
                {label}
              </div>
            </DropdownMenuCheckboxItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>

      {/* Date Range Filter */}
      <Popover>
        <PopoverTrigger asChild>
          <Button variant="outline" size="sm" className="h-8">
            <CalendarIcon className="h-3 w-3 mr-1" />
            Date
            {(filters.dateFrom || filters.dateTo) && (
              <Badge variant="secondary" className="ml-1 h-4 px-1 text-[10px]">
                •
              </Badge>
            )}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-auto p-0" align="start">
          <div className="flex">
            <div className="p-2 border-r">
              <div className="text-xs text-muted-foreground mb-2">From</div>
              <Calendar
                mode="single"
                selected={
                  filters.dateFrom ? new Date(filters.dateFrom) : undefined
                }
                onSelect={(d) => handleDateSelect("dateFrom", d)}
              />
            </div>
            <div className="p-2">
              <div className="text-xs text-muted-foreground mb-2">To</div>
              <Calendar
                mode="single"
                selected={filters.dateTo ? new Date(filters.dateTo) : undefined}
                onSelect={(d) => handleDateSelect("dateTo", d)}
              />
            </div>
          </div>
        </PopoverContent>
      </Popover>

      {/* Clear Filters */}
      {hasActiveFilters && (
        <Button
          variant="ghost"
          size="sm"
          onClick={resetFilters}
          className="h-8 text-muted-foreground"
        >
          <X className="h-3 w-3 mr-1" />
          Clear
        </Button>
      )}
    </div>
  );
});
```

### 3.5 ConversationList (Virtualized)

```typescript
// src/components/query/history/ConversationList.tsx
"use client";

import { memo, useRef, useCallback, useEffect } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useQueryPageState } from "@/hooks/use-query-page-state";
import { useQueryUIStore } from "@/stores/use-query-ui-store";
import { ConversationItem } from "./ConversationItem";
import { LoadingState } from "./LoadingState";
import { EmptyState } from "./EmptyState";

const ITEM_HEIGHT = 72; // Height of each conversation item

export const ConversationList = memo(function ConversationList() {
  const {
    conversations,
    isLoadingList,
    hasMoreConversations,
    loadMoreConversations,
  } = useQueryPageState();

  const { activeConversationId, isSelectionMode } = useQueryUIStore();
  const parentRef = useRef<HTMLDivElement>(null);
  const loadMoreRef = useRef<HTMLDivElement>(null);

  // Virtual list configuration
  const virtualizer = useVirtualizer({
    count: conversations.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ITEM_HEIGHT,
    overscan: 5,
  });

  // Intersection observer for infinite scroll
  useEffect(() => {
    if (!loadMoreRef.current || !hasMoreConversations) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          loadMoreConversations();
        }
      },
      { threshold: 0.1 }
    );

    observer.observe(loadMoreRef.current);
    return () => observer.disconnect();
  }, [hasMoreConversations, loadMoreConversations]);

  // Loading state
  if (isLoadingList && conversations.length === 0) {
    return <LoadingState count={5} />;
  }

  // Empty state
  if (conversations.length === 0) {
    return <EmptyState />;
  }

  return (
    <div
      ref={parentRef}
      className="flex-1 overflow-auto"
      style={{ contain: "strict" }}
    >
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        {virtualizer.getVirtualItems().map((virtualItem) => {
          const conversation = conversations[virtualItem.index];
          return (
            <div
              key={conversation.id}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: `${virtualItem.size}px`,
                transform: `translateY(${virtualItem.start}px)`,
              }}
            >
              <ConversationItem
                conversation={conversation}
                isActive={conversation.id === activeConversationId}
                isSelectionMode={isSelectionMode}
              />
            </div>
          );
        })}
      </div>

      {/* Load more trigger */}
      {hasMoreConversations && (
        <div
          ref={loadMoreRef}
          className="h-12 flex items-center justify-center"
        >
          <span className="text-sm text-muted-foreground">Loading more...</span>
        </div>
      )}
    </div>
  );
});
```

### 3.6 ConversationItem

```typescript
// src/components/query/history/ConversationItem.tsx
"use client";

import { memo, useCallback, useState } from "react";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  MoreHorizontal,
  Pin,
  Archive,
  Share2,
  Trash2,
  Pencil,
} from "lucide-react";
import { useQueryUIStore } from "@/stores/use-query-ui-store";
import {
  useDeleteConversation,
  useUpdateConversation,
  useShareConversation,
} from "@/hooks/use-conversations";
import { formatRelative } from "date-fns";
import type { Conversation } from "@/types";

interface ConversationItemProps {
  conversation: Conversation;
  isActive: boolean;
  isSelectionMode: boolean;
}

const MODE_COLORS: Record<string, string> = {
  local: "bg-blue-500",
  global: "bg-green-500",
  hybrid: "bg-purple-500",
  naive: "bg-gray-500",
};

export const ConversationItem = memo(function ConversationItem({
  conversation,
  isActive,
  isSelectionMode,
}: ConversationItemProps) {
  const { setActiveConversation, toggleSelection, selectedIds } =
    useQueryUIStore();
  const deleteMutation = useDeleteConversation();
  const updateMutation = useUpdateConversation();
  const shareMutation = useShareConversation();

  const [isRenaming, setIsRenaming] = useState(false);
  const [editTitle, setEditTitle] = useState(conversation.title);

  const isSelected = selectedIds.has(conversation.id);

  const handleClick = useCallback(() => {
    if (isSelectionMode) {
      toggleSelection(conversation.id);
    } else {
      setActiveConversation(conversation.id);
    }
  }, [
    conversation.id,
    isSelectionMode,
    setActiveConversation,
    toggleSelection,
  ]);

  const handlePin = useCallback(() => {
    updateMutation.mutate({
      id: conversation.id,
      data: { is_pinned: !conversation.is_pinned },
    });
  }, [conversation.id, conversation.is_pinned, updateMutation]);

  const handleArchive = useCallback(() => {
    updateMutation.mutate({
      id: conversation.id,
      data: { is_archived: !conversation.is_archived },
    });
  }, [conversation.id, conversation.is_archived, updateMutation]);

  const handleDelete = useCallback(() => {
    if (confirm("Delete this conversation?")) {
      deleteMutation.mutate(conversation.id);
    }
  }, [conversation.id, deleteMutation]);

  const handleShare = useCallback(() => {
    shareMutation.mutate(conversation.id);
  }, [conversation.id, shareMutation]);

  const handleRename = useCallback(() => {
    if (editTitle.trim() && editTitle !== conversation.title) {
      updateMutation.mutate({
        id: conversation.id,
        data: { title: editTitle.trim() },
      });
    }
    setIsRenaming(false);
  }, [conversation.id, conversation.title, editTitle, updateMutation]);

  return (
    <div
      onClick={handleClick}
      className={cn(
        "group flex items-start gap-3 p-3 rounded-lg cursor-pointer",
        "transition-colors duration-150",
        isActive && "bg-accent",
        !isActive && "hover:bg-muted/50",
        isSelected && "ring-2 ring-primary ring-inset"
      )}
    >
      {/* Selection checkbox */}
      {isSelectionMode && (
        <Checkbox
          checked={isSelected}
          onCheckedChange={() => toggleSelection(conversation.id)}
          onClick={(e) => e.stopPropagation()}
          className="mt-1"
        />
      )}

      {/* Main content */}
      <div className="flex-1 min-w-0">
        {/* Title */}
        {isRenaming ? (
          <input
            value={editTitle}
            onChange={(e) => setEditTitle(e.target.value)}
            onBlur={handleRename}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleRename();
              if (e.key === "Escape") setIsRenaming(false);
            }}
            autoFocus
            className="w-full bg-transparent border-b border-primary outline-none text-sm font-medium"
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <div className="flex items-center gap-2">
            {conversation.is_pinned && (
              <Pin className="h-3 w-3 text-primary shrink-0" />
            )}
            <span className="text-sm font-medium truncate">
              {conversation.title}
            </span>
          </div>
        )}

        {/* Preview */}
        {conversation.last_message_preview && (
          <p className="text-xs text-muted-foreground truncate mt-0.5">
            {conversation.last_message_preview}
          </p>
        )}

        {/* Metadata */}
        <div className="flex items-center gap-2 mt-1">
          <div
            className={cn(
              "w-2 h-2 rounded-full",
              MODE_COLORS[conversation.mode]
            )}
          />
          <span className="text-[10px] text-muted-foreground">
            {formatRelative(new Date(conversation.updated_at), new Date())}
          </span>
          <Badge variant="outline" className="text-[10px] h-4">
            {conversation.message_count} msgs
          </Badge>
        </div>
      </div>

      {/* Actions menu */}
      {!isSelectionMode && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 opacity-0 group-hover:opacity-100"
              onClick={(e) => e.stopPropagation()}
            >
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={() => setIsRenaming(true)}>
              <Pencil className="h-4 w-4 mr-2" />
              Rename
            </DropdownMenuItem>
            <DropdownMenuItem onClick={handlePin}>
              <Pin className="h-4 w-4 mr-2" />
              {conversation.is_pinned ? "Unpin" : "Pin"}
            </DropdownMenuItem>
            <DropdownMenuItem onClick={handleShare}>
              <Share2 className="h-4 w-4 mr-2" />
              Share
            </DropdownMenuItem>
            <DropdownMenuItem onClick={handleArchive}>
              <Archive className="h-4 w-4 mr-2" />
              {conversation.is_archived ? "Unarchive" : "Archive"}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              onClick={handleDelete}
              className="text-destructive"
            >
              <Trash2 className="h-4 w-4 mr-2" />
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  );
});
```

### 3.7 SelectionToolbar

```typescript
// src/components/query/history/SelectionToolbar.tsx
"use client";

import { memo, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Archive, Trash2, X, FolderInput } from "lucide-react";
import { useQueryUIStore } from "@/stores/use-query-ui-store";
import { useDeleteConversation } from "@/hooks/use-conversations";

export const SelectionToolbar = memo(function SelectionToolbar() {
  const { selectedIds, clearSelection } = useQueryUIStore();
  const deleteMutation = useDeleteConversation();

  const handleDeleteSelected = useCallback(async () => {
    if (!confirm(`Delete ${selectedIds.size} conversations?`)) return;

    // Delete all selected
    const promises = Array.from(selectedIds).map((id) =>
      deleteMutation.mutateAsync(id).catch(() => null)
    );
    await Promise.all(promises);
    clearSelection();
  }, [selectedIds, deleteMutation, clearSelection]);

  return (
    <div className="flex items-center gap-2 p-3 border-t border-border bg-muted/50">
      <span className="text-sm font-medium">{selectedIds.size} selected</span>

      <div className="flex-1" />

      <Button variant="ghost" size="sm" onClick={clearSelection}>
        <X className="h-4 w-4" />
      </Button>

      <Button variant="outline" size="sm">
        <Archive className="h-4 w-4 mr-1" />
        Archive
      </Button>

      <Button variant="outline" size="sm">
        <FolderInput className="h-4 w-4 mr-1" />
        Move
      </Button>

      <Button
        variant="destructive"
        size="sm"
        onClick={handleDeleteSelected}
        disabled={deleteMutation.isPending}
      >
        <Trash2 className="h-4 w-4 mr-1" />
        Delete
      </Button>
    </div>
  );
});
```

---

## 4. Loading & Empty States

### 4.1 LoadingState

```typescript
// src/components/query/history/LoadingState.tsx
"use client";

import { memo } from "react";
import { Skeleton } from "@/components/ui/skeleton";

interface LoadingStateProps {
  count?: number;
}

export const LoadingState = memo(function LoadingState({
  count = 5,
}: LoadingStateProps) {
  return (
    <div className="space-y-2 p-2">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="p-3 rounded-lg">
          <Skeleton className="h-4 w-3/4 mb-2" />
          <Skeleton className="h-3 w-1/2 mb-2" />
          <Skeleton className="h-3 w-1/4" />
        </div>
      ))}
    </div>
  );
});
```

### 4.2 EmptyState

```typescript
// src/components/query/history/EmptyState.tsx
"use client";

import { memo } from "react";
import { MessageSquare, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useQueryUIStore } from "@/stores/use-query-ui-store";
import { useCreateConversation } from "@/hooks/use-conversations";

export const EmptyState = memo(function EmptyState() {
  const { filters, resetFilters, setActiveConversation } = useQueryUIStore();
  const createMutation = useCreateConversation();

  const hasFilters =
    filters.search ||
    filters.mode !== null ||
    filters.dateFrom ||
    filters.dateTo;

  const handleNewChat = async () => {
    const conv = await createMutation.mutateAsync({ mode: "hybrid" });
    setActiveConversation(conv.id);
  };

  return (
    <div className="flex flex-col items-center justify-center h-full py-8 px-4 text-center">
      <MessageSquare className="h-12 w-12 text-muted-foreground/50 mb-4" />

      {hasFilters ? (
        <>
          <p className="text-sm text-muted-foreground mb-4">
            No conversations match your filters
          </p>
          <Button variant="outline" onClick={resetFilters}>
            Clear filters
          </Button>
        </>
      ) : (
        <>
          <p className="text-sm text-muted-foreground mb-4">
            No conversations yet
          </p>
          <Button onClick={handleNewChat} disabled={createMutation.isPending}>
            <Plus className="h-4 w-4 mr-2" />
            Start a conversation
          </Button>
        </>
      )}
    </div>
  );
});
```

---

## 5. Dependencies

Add to `package.json`:

```json
{
  "dependencies": {
    "@tanstack/react-virtual": "^3.0.0",
    "use-debounce": "^10.0.0",
    "date-fns": "^3.0.0"
  }
}
```

---

## 6. Testing Checklist

| Test                 | Expected Result                       |
| -------------------- | ------------------------------------- |
| Infinite scroll      | Loads more on scroll to bottom        |
| Search (debounced)   | Filters after 300ms pause             |
| Mode filter          | Filters by selected modes             |
| Date filter          | Filters by date range                 |
| Clear filters        | Resets all filters                    |
| Pin conversation     | Pins/unpins, shows icon               |
| Archive conversation | Removes from list (if viewing active) |
| Delete conversation  | Confirms and deletes                  |
| Rename inline        | Double-click opens editor             |
| Selection mode       | Checkboxes appear                     |
| Batch delete         | Deletes all selected                  |
| Virtualization       | Smooth scroll with 1000+ items        |
| Empty state          | Shows with and without filters        |

---

_Last updated: 2024-12-27_
