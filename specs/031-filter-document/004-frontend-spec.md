# SPEC-031 / 004 — Frontend Specification

> **Lens**: Full Stack Engineer  
> **Cross-refs**: [002-ux-ui-design.md](002-ux-ui-design.md), [003-api-backend-spec.md](003-api-backend-spec.md)  
> **Constraints**: DRY · SOLID · First-class TypeScript types · No new dependencies

---

## 1. Type Changes

### 1.1 Extended `DocumentFilter` (types/query.ts)

```typescript
// edgequake_webui/src/types/query.ts

export interface DocumentFilter {
  /** Start date (inclusive), ISO 8601. @implements SPEC-005 */
  date_from?: string;
  /** End date (inclusive), ISO 8601. @implements SPEC-005 */
  date_to?: string;
  /** Case-insensitive title substring, comma-separated OR. @implements SPEC-005 */
  document_pattern?: string;
  /**
   * Explicit document IDs to restrict query scope.
   * When set, only these documents contribute RAG context.
   * Empty array treated as null (no filtering).
   * @implements SPEC-031
   */
  document_ids?: string[];
}

/** Returns true when the filter has no active criteria. */
export function isEmptyDocumentFilter(f?: DocumentFilter): boolean {
  if (!f) return true;
  return (
    !f.date_from &&
    !f.date_to &&
    !f.document_pattern &&
    (!f.document_ids || f.document_ids.length === 0)
  );
}
```

### 1.2 New `DocumentSearchItem` Type (types/documents.ts or types/index.ts)

```typescript
/**
 * Minimal document projection for the scope picker.
 * Returned by GET /api/v1/documents/search.
 * @implements SPEC-031
 */
export interface DocumentSearchItem {
  id: string;
  title: string;
  status: string;
  created_at?: string;
}

export interface DocumentSearchResponse {
  items: DocumentSearchItem[];
  total: number;
  has_more: boolean;
}
```

---

## 2. API Client

### 2.1 New `searchDocuments` function (lib/api/edgequake/documents.ts)

```typescript
import type { DocumentSearchResponse } from '@/types';

/**
 * Search documents by title for the scope picker.
 * Lightweight — returns only id/title/status projections.
 * @implements SPEC-031
 */
export async function searchDocuments(params: {
  q?: string;
  page_size?: number;
  status?: string;
}): Promise<DocumentSearchResponse> {
  const query = buildQueryString({
    q: params.q,
    page_size: params.page_size ?? 20,
    status: params.status ?? 'completed',
  });
  return api.get<DocumentSearchResponse>(withQuery('/documents/search', query));
}
```

---

## 3. React Query Hook — `useDocumentSearch`

### 3.1 Hook (hooks/use-document-search.ts)

```typescript
'use client';
/**
 * @module useDocumentSearch
 * @description Type-ahead document search for the scope picker.
 * Debounces query, caches results, returns loading/error state.
 * @implements SPEC-031
 */

import { useQuery } from '@tanstack/react-query';
import { useDebounce } from '@/hooks/use-debounce';
import { searchDocuments } from '@/lib/api/edgequake/documents';
import type { DocumentSearchItem } from '@/types';

const SEARCH_DEBOUNCE_MS = 300;
const SEARCH_STALE_TIME_MS = 30_000; // 30s — documents don't change often
const MIN_QUERY_LENGTH = 0; // Show recent docs when query is empty

export function useDocumentSearch(query: string, enabled = true) {
  const debouncedQuery = useDebounce(query, SEARCH_DEBOUNCE_MS);

  return useQuery<DocumentSearchItem[]>({
    queryKey: ['documents', 'search', debouncedQuery],
    queryFn: async () => {
      const result = await searchDocuments({
        q: debouncedQuery || undefined,
        page_size: 20,
        status: 'completed',
      });
      return result.items;
    },
    enabled: enabled && debouncedQuery.length >= MIN_QUERY_LENGTH,
    staleTime: SEARCH_STALE_TIME_MS,
    gcTime: 60_000,
    // On network error, show stale data rather than breaking the UI
    placeholderData: (prev) => prev,
  });
}
```

### 3.2 `useDebounce` (reuse existing or create)

```typescript
// hooks/use-debounce.ts — create if not exists
import { useEffect, useState } from 'react';

export function useDebounce<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);
  return debounced;
}
```

---

## 4. New Component: `DocumentPickerPopover`

### 4.1 Component Interface

```typescript
// components/query/document-picker-popover.tsx

export interface DocumentPickerPopoverProps {
  /** Currently selected document IDs */
  selectedIds: string[];
  /** Callback when selection changes */
  onSelectionChange: (ids: string[]) => void;
  /** Whether the picker is disabled (e.g., during query execution) */
  disabled?: boolean;
  /** Custom trigger element */
  trigger?: React.ReactNode;
}
```

### 4.2 Full Component Implementation

```typescript
'use client';
/**
 * @module DocumentPickerPopover
 * @description Popover for selecting specific documents to scope a query.
 * Features type-ahead search, checkbox selection, and selected item summary.
 * @implements SPEC-031: Document scope selection
 */

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { useDocumentSearch } from '@/hooks/use-document-search';
import { cn } from '@/lib/utils';
import type { DocumentSearchItem } from '@/types';
import { FileText, Loader2, Plus, Search, X } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface DocumentPickerPopoverProps {
  selectedIds: string[];
  onSelectionChange: (ids: string[]) => void;
  disabled?: boolean;
  trigger?: React.ReactNode;
}

export function DocumentPickerPopover({
  selectedIds,
  onSelectionChange,
  disabled = false,
  trigger,
}: DocumentPickerPopoverProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');

  const { data: searchResults = [], isLoading } = useDocumentSearch(search, open);

  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);

  const toggle = useCallback(
    (item: DocumentSearchItem) => {
      if (selectedSet.has(item.id)) {
        onSelectionChange(selectedIds.filter((id) => id !== item.id));
      } else {
        onSelectionChange([...selectedIds, item.id]);
      }
    },
    [selectedIds, selectedSet, onSelectionChange],
  );

  const clearAll = useCallback(() => onSelectionChange([]), [onSelectionChange]);

  // Sort results: selected items first, then by title
  const sortedResults = useMemo(() => {
    return [...searchResults].sort((a, b) => {
      const aSelected = selectedSet.has(a.id) ? 0 : 1;
      const bSelected = selectedSet.has(b.id) ? 0 : 1;
      if (aSelected !== bSelected) return aSelected - bSelected;
      return a.title.localeCompare(b.title);
    });
  }, [searchResults, selectedSet]);

  // Selected items not in current search results (for summary section)
  // We still show them so the user can uncheck them
  const selectedNotInResults = useMemo(() => {
    const inResultIds = new Set(searchResults.map((r) => r.id));
    return selectedIds
      .filter((id) => !inResultIds.has(id))
      .map((id) => ({ id, title: id, status: 'unknown', created_at: undefined }));
  }, [selectedIds, searchResults]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        {trigger ?? (
          <Button
            variant="ghost"
            size="sm"
            disabled={disabled}
            className="gap-1.5 h-7 text-xs"
            aria-label={t('query.scope.addDocuments', 'Add documents to scope')}
          >
            <Plus className="h-3.5 w-3.5" />
            {t('query.scope.add', 'Add')}
          </Button>
        )}
      </PopoverTrigger>

      <PopoverContent
        align="start"
        className="w-80 p-0"
        aria-label={t('query.scope.popover', 'Document scope selector')}
      >
        {/* Header */}
        <div className="px-3 pt-3 pb-2">
          <p className="text-sm font-semibold mb-2">
            {t('query.scope.title', 'Scope documents')}
          </p>
          {/* Search input */}
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 h-3.5 w-3.5 text-muted-foreground" />
            <Input
              placeholder={t('query.scope.searchPlaceholder', 'Search by title...')}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-8 pr-8 h-8 text-sm"
              aria-label={t('query.scope.searchLabel', 'Search documents by title')}
              aria-autocomplete="list"
            />
            {search && (
              <button
                onClick={() => setSearch('')}
                className="absolute right-2 top-2 text-muted-foreground hover:text-foreground"
                aria-label={t('common.clear', 'Clear')}
              >
                <X className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        </div>

        <Separator />

        {/* Results list */}
        <ScrollArea className="max-h-60">
          {isLoading && (
            <div className="flex items-center gap-2 px-3 py-3 text-xs text-muted-foreground">
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              {t('common.loading', 'Loading...')}
            </div>
          )}

          {!isLoading && sortedResults.length === 0 && !search && (
            <div className="px-3 py-4 text-xs text-muted-foreground text-center">
              {t('query.scope.noDocuments', 'No documents in this workspace yet.')}
            </div>
          )}

          {!isLoading && sortedResults.length === 0 && search && (
            <div className="px-3 py-4 text-xs text-muted-foreground text-center">
              {t('query.scope.noResults', 'No documents match "{query}".', { query: search })}
            </div>
          )}

          <div
            role="listbox"
            aria-label={t('query.scope.resultsList', 'Document search results')}
            aria-multiselectable="true"
          >
            {sortedResults.map((item) => {
              const checked = selectedSet.has(item.id);
              return (
                <button
                  key={item.id}
                  role="option"
                  aria-selected={checked}
                  onClick={() => toggle(item)}
                  className={cn(
                    'w-full flex items-center gap-2.5 px-3 py-2 text-sm',
                    'hover:bg-accent hover:text-accent-foreground',
                    'focus-visible:outline-none focus-visible:bg-accent',
                    checked && 'bg-accent/50',
                  )}
                >
                  <Checkbox
                    checked={checked}
                    readOnly
                    className="pointer-events-none h-3.5 w-3.5 shrink-0"
                    aria-hidden="true"
                  />
                  <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                  <span className="truncate text-left flex-1" title={item.title}>
                    {item.title}
                  </span>
                </button>
              );
            })}

            {/* Selected items not in current search results */}
            {selectedNotInResults.map((item) => (
              <button
                key={`selected-${item.id}`}
                role="option"
                aria-selected={true}
                onClick={() => toggle(item)}
                className={cn(
                  'w-full flex items-center gap-2.5 px-3 py-2 text-sm bg-accent/50',
                  'hover:bg-accent hover:text-accent-foreground',
                )}
              >
                <Checkbox checked readOnly className="pointer-events-none h-3.5 w-3.5 shrink-0" />
                <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                <span className="truncate text-left flex-1 opacity-60" title={item.id}>
                  {item.id}
                </span>
              </button>
            ))}
          </div>
        </ScrollArea>

        {/* Footer */}
        {selectedIds.length > 0 && (
          <>
            <Separator />
            <div className="px-3 py-2 flex items-center justify-between">
              <span className="text-xs text-muted-foreground">
                {t('query.scope.selectedCount', '{count} selected', {
                  count: selectedIds.length,
                })}
              </span>
              <Button
                variant="ghost"
                size="sm"
                onClick={clearAll}
                className="h-6 text-xs gap-1 text-muted-foreground"
              >
                <X className="h-3 w-3" />
                {t('query.scope.clearAll', 'Clear all')}
              </Button>
            </div>
          </>
        )}
      </PopoverContent>
    </Popover>
  );
}
```

---

## 5. New Component: `QueryScopeBar`

### 5.1 Component Interface

```typescript
// components/query/query-scope-bar.tsx

export interface QueryScopePillsProps {
  /** Selected document IDs */
  selectedIds: string[];
  /** Callback when selection changes (e.g., pill removed) */
  onSelectionChange: (ids: string[]) => void;
  /** Whether the scope bar is disabled (e.g., during query execution) */
  disabled?: boolean;
}
```

> **Always renders.** Empty state shows "All docs ▾" ghost button — the primary
> discoverability affordance. Active state shows pills. No null-return guard.
> This is intentional (see SPEC-031 INV-04 update).

### 5.2 Full Component

```typescript
'use client';
/**
 * @module QueryScopeBar
 * @description Horizontal pill bar showing active document scope.
 * Rendered ONLY when selectedIds is non-empty (zero footprint otherwise).
 * @implements SPEC-031: Document scope visualization
 */

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { DocumentPickerPopover } from './document-picker-popover';
import { useDocumentTitles } from '@/hooks/use-document-titles';

const MAX_VISIBLE_PILLS = 3;
const MAX_PILL_CHARS = 22;

interface QueryScopeBarProps {
  selectedIds: string[];
  onSelectionChange: (ids: string[]) => void;
  disabled?: boolean;
}

export function QueryScopeBar({
  selectedIds,
  onSelectionChange,
  disabled = false,
}: QueryScopeBarProps) {
  const { t } = useTranslation();

  // Null render when no scope selected — zero DOM footprint
  if (selectedIds.length === 0) return null;

  const visibleIds = selectedIds.slice(0, MAX_VISIBLE_PILLS);
  const hiddenCount = selectedIds.length - visibleIds.length;

  const removeId = (id: string) => {
    onSelectionChange(selectedIds.filter((sid) => sid !== id));
  };

  const clearAll = () => onSelectionChange([]);

  return (
    <div
      role="region"
      aria-label={t('query.scope.activeScope', 'Active query scope')}
      className={cn(
        'flex items-center gap-1.5 px-3 py-1.5',
        'border-b bg-muted/30 overflow-x-auto',
        'scrollbar-hide', // hide scrollbar on mobile
        disabled && 'opacity-60 pointer-events-none',
      )}
    >
      {/* Label */}
      <span className="text-xs text-muted-foreground shrink-0 mr-0.5">
        {t('query.scope.label', 'Scope:')}
      </span>

      {/* Pills */}
      <ul role="list" className="flex items-center gap-1.5 flex-wrap">
        {visibleIds.map((id) => (
          <ScopePill
            key={id}
            documentId={id}
            onRemove={() => removeId(id)}
            disabled={disabled}
          />
        ))}

        {/* +N more */}
        {hiddenCount > 0 && (
          <li>
            <DocumentPickerPopover
              selectedIds={selectedIds}
              onSelectionChange={onSelectionChange}
              disabled={disabled}
              trigger={
                <button
                  className={cn(
                    'inline-flex items-center rounded-full px-2 py-0.5 text-xs',
                    'bg-muted text-muted-foreground hover:bg-muted/80',
                    'focus-visible:outline-none focus-visible:ring-2',
                  )}
                  aria-label={t('query.scope.moreCount', '{count} more documents in scope', {
                    count: hiddenCount,
                  })}
                >
                  +{hiddenCount}
                </button>
              }
            />
          </li>
        )}
      </ul>

      {/* Add more button */}
      <DocumentPickerPopover
        selectedIds={selectedIds}
        onSelectionChange={onSelectionChange}
        disabled={disabled}
      />

      {/* Clear all */}
      <button
        onClick={clearAll}
        disabled={disabled}
        className={cn(
          'ml-auto shrink-0 text-xs text-muted-foreground',
          'hover:text-destructive focus-visible:outline-none',
        )}
        aria-label={t('query.scope.clearAllScope', 'Clear all document scope')}
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

/**
 * Individual scope pill for a single document.
 * Resolves document title from query cache if available.
 */
function ScopePill({
  documentId,
  onRemove,
  disabled,
}: {
  documentId: string;
  onRemove: () => void;
  disabled: boolean;
}) {
  const { t } = useTranslation();
  // Attempt to get the title from the documents list cache
  const title = useDocumentTitle(documentId);
  const displayTitle = title
    ? title.length > MAX_PILL_CHARS
      ? title.slice(0, MAX_PILL_CHARS) + '…'
      : title
    : documentId.slice(0, 8) + '…';

  return (
    <li role="listitem">
      <span
        className={cn(
          'inline-flex items-center gap-1 rounded-full pl-2 pr-1 py-0.5',
          'bg-secondary text-secondary-foreground text-xs',
          'max-w-[200px]',
        )}
        title={title ?? documentId}
      >
        <span className="truncate">{displayTitle}</span>
        <button
          onClick={onRemove}
          disabled={disabled}
          className={cn(
            'ml-0.5 shrink-0 rounded-full p-0.5',
            'text-muted-foreground hover:text-destructive',
            'focus-visible:outline-none focus-visible:ring-1',
          )}
          aria-label={t('query.scope.removeDoc', 'Remove {title} from scope', {
            title: title ?? documentId,
          })}
        >
          <X className="h-3 w-3" />
        </button>
      </span>
    </li>
  );
}
```

### 5.3 `useDocumentTitle` Helper Hook

```typescript
// hooks/use-document-title.ts
/**
 * Looks up a document title from the React Query cache.
 * Falls back to undefined if not cached (caller shows ID fragment).
 * Does NOT trigger a fetch — purely reads from cache.
 * @implements SPEC-031
 */
import { useQueryClient } from '@tanstack/react-query';
import type { DocumentSearchItem } from '@/types';

export function useDocumentTitle(documentId: string): string | undefined {
  const qc = useQueryClient();

  // Try search results cache (most likely to have recent docs)
  const allSearchCaches = qc.getQueriesData<DocumentSearchItem[]>({
    queryKey: ['documents', 'search'],
  });

  for (const [, data] of allSearchCaches) {
    if (data) {
      const found = data.find((item) => item.id === documentId);
      if (found) return found.title;
    }
  }

  // Try full documents list cache
  const listData = qc.getQueryData<{ items: Array<{ id: string; title: string }> }>(['documents']);
  if (listData?.items) {
    const found = listData.items.find((item) => item.id === documentId);
    if (found) return found.title;
  }

  return undefined;
}
```

---

## 6. State Management — `useQuerySettings` Extension

### 6.1 Current Settings Shape

```typescript
interface QuerySettings {
  mode: QueryMode;
  stream: boolean;
  topK: number;
  temperature: number;
  maxTokens: number;
  systemPrompt?: string;
  provider?: string;
  model?: string;
  documentFilter?: DocumentFilter;  // SPEC-005
}
```

### 6.2 Extended Settings Shape

```typescript
interface QuerySettings {
  mode: QueryMode;
  stream: boolean;
  topK: number;
  temperature: number;
  maxTokens: number;
  systemPrompt?: string;
  provider?: string;
  model?: string;
  documentFilter?: DocumentFilter;  // SPEC-005: date/pattern filter

  /**
   * Explicitly selected document IDs for scope restriction.
   * This is the SPEC-031 addition — separate from documentFilter for clarity.
   * Merged into documentFilter.document_ids before API call.
   * @implements SPEC-031
   */
  scopedDocumentIds?: string[];
}
```

**Why separate from `documentFilter`?** The `documentFilter` in settings sheet handles date/pattern (more "advanced" filters). The `scopedDocumentIds` is the primary explicit selection — keeping them separate avoids coupling two different interaction paradigms into one state atom.

Before sending the query, they are merged:

```typescript
// In useQueryInterface or the submit handler:
function buildDocumentFilter(settings: QuerySettings): DocumentFilter | undefined {
  const hasDateOrPattern =
    settings.documentFilter?.date_from ||
    settings.documentFilter?.date_to ||
    settings.documentFilter?.document_pattern;
  const hasScope = settings.scopedDocumentIds && settings.scopedDocumentIds.length > 0;

  if (!hasDateOrPattern && !hasScope) return undefined;

  return {
    ...settings.documentFilter,
    document_ids: hasScope ? settings.scopedDocumentIds : undefined,
  };
}
```

### 6.3 Persistence

```typescript
// In useQuerySettings, extend localStorage key handling:
const SETTINGS_STORAGE_KEY = 'edgequake:query-settings-v2';  // bump version

// scopedDocumentIds is persisted — user's scope survives page reload.
// This is intentional: if you scoped to 3 docs, you should keep that scope.
// Users can clear explicitly via [× All].
```

---

## 7. Integration into `QueryInterface`

### 7.1 Header Changes

No changes to the existing header bar structure. The scope bar is inserted **between the messages area and the text input**, not in the header.

### 7.2 Integration Point

```typescript
// In query-interface.tsx, inside the content area, above the text input:

<div className="shrink-0">
  <QueryScopeBar
    selectedIds={querySettings.scopedDocumentIds ?? []}
    onSelectionChange={(ids) => setQuerySettings({ scopedDocumentIds: ids })}
    disabled={isLoading}
  />
</div>

{/* Existing text input area */}
<div className="shrink-0 border-t ...">
  ...
</div>
```

### 7.3 Submit Handler Changes

```typescript
// In useQueryInterface hook, in the handleSubmit function:
const documentFilter = buildDocumentFilter(querySettings);

// Pass to chat API:
await sendMessage({
  query: input,
  mode: querySettings.mode,
  // ...other settings...
  document_filter: documentFilter,  // now includes document_ids if scoped
});
```

---

## 8. Settings Sheet Integration

In `QuerySettingsSheet`, add a "Document Scope" section below the existing `QueryDocumentFilter`:

```typescript
{/* In QuerySettingsSheet — new section */}
<div className="space-y-2">
  <div className="flex items-center gap-2">
    <FileText className="h-4 w-4 text-muted-foreground" />
    <Label className="text-sm font-medium">
      {t('query.scope.sectionTitle', 'Document Scope')}
    </Label>
    {scopedDocumentIds && scopedDocumentIds.length > 0 && (
      <Badge variant="secondary" className="text-xs">
        {scopedDocumentIds.length}
      </Badge>
    )}
  </div>
  <p className="text-xs text-muted-foreground">
    {t('query.scope.description',
      'Restrict this query to specific documents. Default is all workspace documents.'
    )}
  </p>
  <DocumentPickerPopover
    selectedIds={scopedDocumentIds ?? []}
    onSelectionChange={onScopedDocumentIdsChange}
    disabled={disabled}
    trigger={
      <Button variant="outline" size="sm" className="w-full gap-2 justify-start">
        <Plus className="h-4 w-4" />
        {scopedDocumentIds && scopedDocumentIds.length > 0
          ? t('query.scope.editSelection', '{count} documents selected', {
              count: scopedDocumentIds.length,
            })
          : t('query.scope.addDocuments', 'Add documents to scope')
        }
      </Button>
    }
  />
</div>
```

The `QuerySettingsSheet` interface is extended:

```typescript
interface QuerySettingsSheetProps {
  // ... existing props ...
  scopedDocumentIds?: string[];
  onScopedDocumentIdsChange?: (ids: string[]) => void;
}
```

---

## 9. Component Dependency Graph

```
QueryInterface (query-interface.tsx)
  |
  +-- QueryScopeBar (query-scope-bar.tsx)          [NEW]
  |     +-- ScopePill [internal]
  |     |     +-- useDocumentTitle (hook)          [NEW]
  |     +-- DocumentPickerPopover (picker)         [NEW]
  |           +-- useDocumentSearch (hook)         [NEW]
  |                 +-- useDebounce (hook)         [NEW or reuse]
  |                 +-- searchDocuments (api fn)   [NEW]
  |
  +-- QuerySettingsSheet (query-settings-sheet.tsx)
        |
        +-- QueryDocumentFilter (unchanged)        [SPEC-005]
        +-- DocumentPickerPopover (reuse)          [NEW - shared instance]
```

**DRY principle**: `DocumentPickerPopover` is shared between `QueryScopeBar` and `QuerySettingsSheet` — no duplication.

---

## 10. Internationalization Strings

```typescript
// New keys to add to translation files:
{
  "query.scope.addDocuments": "Add documents to scope",
  "query.scope.add": "Add",
  "query.scope.popover": "Document scope selector",
  "query.scope.title": "Scope documents",
  "query.scope.searchPlaceholder": "Search by title…",
  "query.scope.searchLabel": "Search documents by title",
  "query.scope.noDocuments": "No completed documents in this workspace.",
  "query.scope.noResults": "No documents match \"{{query}}\".",
  "query.scope.selectedCount": "{{count}} selected",
  "query.scope.clearAll": "Clear all",
  "query.scope.activeScope": "Query scope",
  "query.scope.label": "Scope:",
  "query.scope.moreCount": "+{{count}} more",
  "query.scope.clearAllScope": "Clear document scope",
  "query.scope.removeDoc": "Remove {{title}} from scope",
  "query.scope.sectionTitle": "Document Scope",
  "query.scope.description": "Restrict queries to specific documents. Default is all workspace docs.",
  "query.scope.editSelection": "Edit scope ({{count}} docs)",
  // Discoverability keys (empty-state "All docs" affordance)
  "query.scope.allDocs": "All docs",
  "query.scope.allDocsLabel": "Query scope: all workspace documents. Click to restrict.",
  "query.scope.allDocsTitle": "Restrict query to specific documents"
}
```

---

## 11. File List (New/Modified)

### New Files

| File                                               | Purpose                                   |
| -------------------------------------------------- | ----------------------------------------- |
| `src/components/query/document-picker-popover.tsx` | Picker popover component                  |
| `src/components/query/query-scope-bar.tsx`         | Scope bar with pills                      |
| `src/hooks/use-document-search.ts`                 | Type-ahead search hook                    |
| `src/hooks/use-document-title.ts`                  | Cache-based title resolver                |
| `src/hooks/use-debounce.ts`                        | Debounce utility (if not already present) |

### Modified Files

| File                                             | Change                                             |
| ------------------------------------------------ | -------------------------------------------------- |
| `src/types/query.ts`                             | Add `document_ids?: string[]` to `DocumentFilter`  |
| `src/types/index.ts` or `src/types/documents.ts` | Add `DocumentSearchItem`, `DocumentSearchResponse` |
| `src/lib/api/edgequake/documents.ts`             | Add `searchDocuments()`                            |
| `src/components/query/query-interface.tsx`       | Insert `<QueryScopeBar>` above input               |
| `src/components/query/query-settings-sheet.tsx`  | Add scope section, extend props                    |
| `src/hooks/use-query-settings.ts`                | Add `scopedDocumentIds` to state                   |
