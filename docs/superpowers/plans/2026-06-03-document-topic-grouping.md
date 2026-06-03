# Document Topic Grouping — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a toggle in the document list toolbar that groups documents into collapsible accordion sections by `enrichment_topic`, with all groups collapsed by default.

**Architecture:** Pure client-side grouping — no backend changes. `useDocumentGrouping` groups the already-fetched `Document[]` by `enrichment_topic`. Toggle state lives in `useDocumentPreferences` (localStorage). `DocumentTableSection` conditionally renders flat table or `DocumentGroupAccordion` list. `DocumentTableRow` is reused inside each accordion group unchanged.

**Tech Stack:** React 19, TypeScript, Tailwind CSS, shadcn/ui (Collapsible), Lucide icons, Vitest.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `edgequake_webui/src/types/index.ts` | Add enrichment fields to `Document` interface |
| Create | `edgequake_webui/src/hooks/use-document-grouping.ts` | Pure grouping logic — groups `Document[]` by topic |
| Create | `edgequake_webui/src/hooks/__tests__/use-document-grouping.test.ts` | Tests for grouping logic |
| Modify | `edgequake_webui/src/hooks/use-document-preferences.ts` | Add `groupedView` + `setGroupedView` |
| Create | `edgequake_webui/src/components/documents/document-group-accordion.tsx` | Accordion for one topic group |
| Modify | `edgequake_webui/src/components/documents/document-toolbar-section.tsx` | Add List/Grouped toggle |
| Modify | `edgequake_webui/src/components/documents/document-table-section.tsx` | Conditional flat vs grouped render |
| Modify | `edgequake_webui/src/components/documents/document-manager.tsx` | Pass `groupedView`/`setGroupedView` through |

---

## Task 1: Add enrichment fields to `Document` type

**Files:**
- Modify: `edgequake_webui/src/types/index.ts`

- [ ] **Step 1: Find the end of the `Document` interface**

```bash
grep -n "document_type\|page_count\|file_size_bytes" edgequake_webui/src/types/index.ts | tail -5
```

The `Document` interface ends around line 145–155. Find the last field before the closing `}`.

- [ ] **Step 2: Add enrichment fields**

After the last existing field (e.g. `file_size_bytes`) and before the closing `}` of the `Document` interface, add:

```ts
  // ========================================================================
  // Metadata Enrichment Fields (PDF Metadata Enrichment Pipeline)
  // ========================================================================

  /** Enrichment pipeline status. */
  enrichment_status?: "pending" | "processing" | "completed" | "failed" | "skipped";
  /** Short topic phrase extracted from first 5 pages. */
  enrichment_topic?: string;
  /** 2-3 paragraph summary. */
  enrichment_summary?: string;
  /** ISO 639-1 language code detected from content. */
  enrichment_language?: string;
  /** Up to 10 keywords extracted from content. */
  enrichment_keywords?: string[];
  /** When enrichment completed. */
  enrichment_completed_at?: string;
  /** Error message if enrichment failed. */
  enrichment_error?: string;
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd edgequake_webui && pnpm exec tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add edgequake_webui/src/types/index.ts
git commit -m "feat(ui): add enrichment fields to Document type"
```

---

## Task 2: Create `useDocumentGrouping` hook

**Files:**
- Create: `edgequake_webui/src/hooks/use-document-grouping.ts`
- Create: `edgequake_webui/src/hooks/__tests__/use-document-grouping.test.ts`

- [ ] **Step 1: Write the failing tests first**

Create `edgequake_webui/src/hooks/__tests__/use-document-grouping.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { groupDocumentsByTopic } from "../use-document-grouping";
import type { Document } from "@/types";

function makeDoc(id: string, topic?: string): Document {
  return { id, enrichment_topic: topic } as Document;
}

describe("groupDocumentsByTopic", () => {
  it("groups documents by topic", () => {
    const docs = [
      makeDoc("1", "Psikologi"),
      makeDoc("2", "Politik"),
      makeDoc("3", "Psikologi"),
    ];
    const result = groupDocumentsByTopic(docs);
    expect(result.get("Psikologi")).toHaveLength(2);
    expect(result.get("Politik")).toHaveLength(1);
  });

  it("puts docs without topic into 'Tanpa Topic'", () => {
    const docs = [makeDoc("1", undefined), makeDoc("2", ""), makeDoc("3", "Politik")];
    const result = groupDocumentsByTopic(docs);
    expect(result.get("Tanpa Topic")).toHaveLength(2);
    expect(result.get("Politik")).toHaveLength(1);
  });

  it("'Tanpa Topic' is always last key in the map", () => {
    const docs = [
      makeDoc("1", undefined),
      makeDoc("2", "Zzz Topic"),
      makeDoc("3", "Aaa Topic"),
    ];
    const result = groupDocumentsByTopic(docs);
    const keys = Array.from(result.keys());
    expect(keys[keys.length - 1]).toBe("Tanpa Topic");
  });

  it("sorts other topics alphabetically", () => {
    const docs = [
      makeDoc("1", "Zebra"),
      makeDoc("2", "Apple"),
      makeDoc("3", "Mango"),
    ];
    const result = groupDocumentsByTopic(docs);
    const keys = Array.from(result.keys());
    expect(keys).toEqual(["Apple", "Mango", "Zebra"]);
  });

  it("returns empty map for empty input", () => {
    expect(groupDocumentsByTopic([]).size).toBe(0);
  });

  it("returns single group when all docs have same topic", () => {
    const docs = [makeDoc("1", "ML"), makeDoc("2", "ML")];
    const result = groupDocumentsByTopic(docs);
    expect(result.size).toBe(1);
    expect(result.get("ML")).toHaveLength(2);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd edgequake_webui && pnpm exec vitest run src/hooks/__tests__/use-document-grouping.test.ts 2>&1 | tail -15
```

Expected: FAIL — `groupDocumentsByTopic` not found.

- [ ] **Step 3: Implement `useDocumentGrouping`**

Create `edgequake_webui/src/hooks/use-document-grouping.ts`:

```ts
import type { Document } from "@/types";
import { useMemo } from "react";

const NO_TOPIC_KEY = "Tanpa Topic";

/**
 * Groups an array of documents by their `enrichment_topic`.
 * Documents without a topic are placed under "Tanpa Topic", always last.
 * Other groups are sorted alphabetically.
 *
 * Exported as a named function so it can be tested independently of React.
 */
export function groupDocumentsByTopic(documents: Document[]): Map<string, Document[]> {
  const groups = new Map<string, Document[]>();

  for (const doc of documents) {
    const topic =
      doc.enrichment_topic && doc.enrichment_topic.trim() !== ""
        ? doc.enrichment_topic.trim()
        : NO_TOPIC_KEY;

    const existing = groups.get(topic);
    if (existing) {
      existing.push(doc);
    } else {
      groups.set(topic, [doc]);
    }
  }

  // Sort: alphabetical first, "Tanpa Topic" last
  const sorted = new Map<string, Document[]>();
  const keys = Array.from(groups.keys())
    .filter((k) => k !== NO_TOPIC_KEY)
    .sort((a, b) => a.localeCompare(b));

  for (const key of keys) {
    sorted.set(key, groups.get(key)!);
  }
  if (groups.has(NO_TOPIC_KEY)) {
    sorted.set(NO_TOPIC_KEY, groups.get(NO_TOPIC_KEY)!);
  }

  return sorted;
}

/**
 * React hook wrapping groupDocumentsByTopic with memoization.
 * Re-groups only when the documents array reference changes.
 */
export function useDocumentGrouping(documents: Document[]): Map<string, Document[]> {
  return useMemo(() => groupDocumentsByTopic(documents), [documents]);
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd edgequake_webui && pnpm exec vitest run src/hooks/__tests__/use-document-grouping.test.ts 2>&1 | tail -15
```

Expected: 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add edgequake_webui/src/hooks/use-document-grouping.ts \
        edgequake_webui/src/hooks/__tests__/use-document-grouping.test.ts
git commit -m "feat(ui): add useDocumentGrouping hook with tests"
```

---

## Task 3: Add `groupedView` to `useDocumentPreferences`

**Files:**
- Modify: `edgequake_webui/src/hooks/use-document-preferences.ts`

The existing hook already persists to localStorage using `STORAGE_KEY = "edgequake:documents:prefs"`. We add one boolean field.

- [ ] **Step 1: Add `groupedView` to the return interface**

In `UseDocumentPreferencesReturn`, add after `setSortDirection`:

```ts
  /** Whether documents are grouped by topic */
  groupedView: boolean;
  setGroupedView: (value: boolean) => void;
```

- [ ] **Step 2: Add default and state**

In the `DEFAULTS` object, add:

```ts
  groupedView: false,
```

Add state initialization after the existing `sortDirection` state:

```ts
  const [groupedView, setGroupedView] = useState<boolean>(() => {
    const prefs = readPreferences();
    return prefs.groupedView ?? DEFAULTS.groupedView;
  });
```

- [ ] **Step 3: Persist `groupedView` in the useEffect**

In the existing `useEffect` that calls `localStorage.setItem`, add `groupedView` to the JSON:

```ts
  useEffect(() => {
    try {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          pageSize,
          statusFilter,
          sortField,
          sortDirection,
          groupedView,
        }),
      );
    } catch {
      // Ignore localStorage errors (e.g., in incognito mode)
    }
  }, [pageSize, statusFilter, sortField, sortDirection, groupedView]);
```

- [ ] **Step 4: Return `groupedView` and `setGroupedView`**

In the `return` statement at the bottom of the hook, add:

```ts
    groupedView,
    setGroupedView,
```

- [ ] **Step 5: Add `groupedView` to `readPreferences` return type**

In the `readPreferences` function's return type annotation, add `groupedView?: boolean`:

```ts
function readPreferences(): Partial<{
  pageSize: number;
  statusFilter: DocStatus;
  sortField: SortField;
  sortDirection: SortDirection;
  groupedView: boolean;
}> {
```

- [ ] **Step 6: Type-check**

```bash
cd edgequake_webui && pnpm exec tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add edgequake_webui/src/hooks/use-document-preferences.ts
git commit -m "feat(ui): add groupedView preference to useDocumentPreferences"
```

---

## Task 4: Create `DocumentGroupAccordion` component

**Files:**
- Create: `edgequake_webui/src/components/documents/document-group-accordion.tsx`

This component renders one collapsible group. It uses shadcn/ui `Collapsible` and reuses `DocumentTableRow` unchanged.

- [ ] **Step 1: Check if `Collapsible` is available**

```bash
grep -r "Collapsible" edgequake_webui/src/components/ui/ 2>/dev/null | head -3
```

If no output, install it:
```bash
cd edgequake_webui && pnpm dlx shadcn@latest add collapsible
```

If already present, skip install.

- [ ] **Step 2: Create the component**

Create `edgequake_webui/src/components/documents/document-group-accordion.tsx`:

```tsx
'use client';

import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import { cn } from '@/lib/utils';
import type { Document } from '@/types';
import { ChevronDown, ChevronRight, Folder } from 'lucide-react';
import { memo, useState } from 'react';
import { DocumentTableRow } from './document-table-row';
import type { DocumentTableRowProps } from './document-table-row';

/**
 * Handlers passed through to each DocumentTableRow — all row actions.
 * Matches the handler props of DocumentTableSectionProps minus
 * selection/id-specific props (those are per-row).
 */
export interface DocumentGroupRowHandlers
  extends Pick<
    DocumentTableRowProps,
    | 'onSelect'
    | 'onClick'
    | 'onDoubleClick'
    | 'onViewDetails'
    | 'onViewInGraph'
    | 'onViewPdf'
    | 'onRetry'
    | 'onCancel'
    | 'onDelete'
    | 'isRetrying'
    | 'isCancelling'
  > {}

export interface DocumentGroupAccordionProps extends DocumentGroupRowHandlers {
  /** Topic label for this group */
  topic: string;
  /** Documents in this group */
  documents: Document[];
  /** Currently selected document IDs */
  selectedIds: Set<string>;
  /** Currently active/previewed document */
  selectedDocument: Document | null;
  /** Current search query (for row highlight) */
  searchQuery: string;
}

/**
 * Collapsible accordion group for documents sharing the same topic.
 * Collapsed by default. "Tanpa Topic" label rendered muted.
 */
export const DocumentGroupAccordion = memo(function DocumentGroupAccordion({
  topic,
  documents,
  selectedIds,
  selectedDocument,
  searchQuery,
  onSelect,
  onClick,
  onDoubleClick,
  onViewDetails,
  onViewInGraph,
  onViewPdf,
  onRetry,
  onCancel,
  onDelete,
  isRetrying,
  isCancelling,
}: DocumentGroupAccordionProps) {
  const [open, setOpen] = useState(false);
  const isNoTopic = topic === 'Tanpa Topic';

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          className="w-full flex items-center gap-2 px-4 py-2 bg-muted/30 hover:bg-muted/50 border-b text-left transition-colors"
          aria-expanded={open}
        >
          {open ? (
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          )}
          <Folder className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
          <span
            className={cn(
              'text-sm font-medium',
              isNoTopic ? 'text-muted-foreground italic' : 'text-foreground',
            )}
          >
            {topic}
          </span>
          <span className="ml-1.5 text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded-full">
            {documents.length}
          </span>
        </button>
      </CollapsibleTrigger>

      <CollapsibleContent>
        {documents.map((doc, index) => (
          <DocumentTableRow
            key={doc.id}
            doc={doc}
            index={index}
            isSelected={selectedIds.has(doc.id)}
            isActive={selectedDocument?.id === doc.id}
            searchQuery={searchQuery}
            onSelect={onSelect}
            onClick={onClick}
            onDoubleClick={onDoubleClick}
            onViewDetails={onViewDetails}
            onViewInGraph={onViewInGraph}
            onViewPdf={onViewPdf}
            onRetry={onRetry}
            onCancel={onCancel}
            onDelete={onDelete}
            isRetrying={isRetrying}
            isCancelling={isCancelling}
          />
        ))}
      </CollapsibleContent>
    </Collapsible>
  );
});
```

- [ ] **Step 3: Export from the documents index**

In `edgequake_webui/src/components/documents/index.ts`, add:

```ts
export { DocumentGroupAccordion } from './document-group-accordion';
```

- [ ] **Step 4: Type-check**

```bash
cd edgequake_webui && pnpm exec tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add edgequake_webui/src/components/documents/document-group-accordion.tsx \
        edgequake_webui/src/components/documents/index.ts
git commit -m "feat(ui): add DocumentGroupAccordion component"
```

---

## Task 5: Add List/Grouped toggle to `DocumentToolbarSection`

**Files:**
- Modify: `edgequake_webui/src/components/documents/document-toolbar-section.tsx`

- [ ] **Step 1: Add props to `DocumentToolbarSectionProps`**

After the existing `onSortDirectionChange` prop, add:

```ts
  /** Whether documents are currently grouped by topic */
  groupedView: boolean;
  /** Toggle grouped view on/off */
  onGroupedViewChange: (value: boolean) => void;
```

- [ ] **Step 2: Destructure the new props in the function signature**

Add `groupedView` and `onGroupedViewChange` to the destructured parameters.

- [ ] **Step 3: Add the toggle UI**

Add the following import at the top of the file:

```tsx
import { LayoutList, LayoutGrid } from 'lucide-react';
import { Button } from '@/components/ui/button';
```

Then in the JSX, inside the `div.flex.flex-col` that wraps search and filters, add the toggle to the right of `<DocumentFilters>`:

```tsx
        <DocumentFilters
          status={statusFilter}
          onStatusChange={onStatusFilterChange}
          sortField={sortField}
          onSortFieldChange={onSortFieldChange}
          sortDirection={sortDirection}
          onSortDirectionChange={onSortDirectionChange}
          statusCounts={statusCounts}
        />
        {/* List / Grouped toggle */}
        <div className="flex items-center border rounded-md overflow-hidden">
          <Button
            variant={!groupedView ? 'secondary' : 'ghost'}
            size="sm"
            className="rounded-none h-8 px-2.5 gap-1.5"
            onClick={() => onGroupedViewChange(false)}
            title="List view"
          >
            <LayoutList className="h-3.5 w-3.5" />
            <span className="text-xs hidden sm:inline">List</span>
          </Button>
          <Button
            variant={groupedView ? 'secondary' : 'ghost'}
            size="sm"
            className="rounded-none h-8 px-2.5 gap-1.5 border-l"
            onClick={() => onGroupedViewChange(true)}
            title="Grouped by topic"
          >
            <LayoutGrid className="h-3.5 w-3.5" />
            <span className="text-xs hidden sm:inline">Grouped</span>
          </Button>
        </div>
```

- [ ] **Step 4: Type-check**

```bash
cd edgequake_webui && pnpm exec tsc --noEmit 2>&1 | tail -10
```

Expected: errors in `DocumentManager` (missing new props) — that's expected, fixed in Task 7.

- [ ] **Step 5: Commit**

```bash
git add edgequake_webui/src/components/documents/document-toolbar-section.tsx
git commit -m "feat(ui): add List/Grouped toggle button to document toolbar"
```

---

## Task 6: Add grouped render to `DocumentTableSection`

**Files:**
- Modify: `edgequake_webui/src/components/documents/document-table-section.tsx`

- [ ] **Step 1: Add new props to `DocumentTableSectionProps`**

After `onClearFilter`, add:

```ts
  /** Whether to render documents grouped by topic */
  groupedView?: boolean;
```

- [ ] **Step 2: Import new dependencies**

Add at the top of the file:

```tsx
import { useDocumentGrouping } from '@/hooks/use-document-grouping';
import { DocumentGroupAccordion } from './document-group-accordion';
```

- [ ] **Step 3: Destructure `groupedView` in the function signature**

Add `groupedView = false` to the destructured props (with default `false` so existing callers work without changes).

- [ ] **Step 4: Add grouped render branch**

Inside the `{!isLoading && documents.length > 0 && (` block, wrap the existing `<div className="border rounded-lg ...">` in a conditional. Replace:

```tsx
          {!isLoading && documents.length > 0 && (
            <div className="border rounded-lg overflow-hidden shadow-sm">
              ...existing table...
            </div>
          )}
```

With:

```tsx
          {!isLoading && documents.length > 0 && (
            groupedView
              ? <GroupedView
                  documents={documents}
                  selectedIds={selectedIds}
                  selectedDocument={selectedDocument}
                  searchQuery={searchQuery}
                  onSelectOne={onSelectOne}
                  onRowClick={onRowClick}
                  onRowDoubleClick={onRowDoubleClick}
                  onViewDetails={onViewDetails}
                  onViewInGraph={onViewInGraph}
                  onViewPdf={onViewPdf}
                  onRetry={onRetry}
                  onCancel={onCancel}
                  onDelete={onDelete}
                  isRetrying={isRetrying}
                  isCancelling={isCancelling}
                />
              : <div className="border rounded-lg overflow-hidden shadow-sm">
                  {/* existing table — unchanged */}
                  ...
                </div>
          )}
```

- [ ] **Step 5: Add `GroupedView` helper component**

Add this private component inside the same file, above `DocumentTableSection`:

```tsx
/**
 * Private helper: renders all topic groups as accordions.
 * Keeps the grouped rendering logic out of DocumentTableSection's main render.
 */
function GroupedView({
  documents,
  selectedIds,
  selectedDocument,
  searchQuery,
  onSelectOne,
  onRowClick,
  onRowDoubleClick,
  onViewDetails,
  onViewInGraph,
  onViewPdf,
  onRetry,
  onCancel,
  onDelete,
  isRetrying,
  isCancelling,
}: {
  documents: Document[];
  selectedIds: Set<string>;
  selectedDocument: Document | null;
  searchQuery: string;
  onSelectOne: (id: string, checked: boolean) => void;
  onRowClick: (doc: Document) => void;
  onRowDoubleClick: (doc: Document) => void;
  onViewDetails: (doc: Document) => void;
  onViewInGraph: (doc: Document) => void;
  onViewPdf: (doc: Document) => void;
  onRetry: (id: string) => void;
  onCancel: (trackId: string) => void;
  onDelete: (id: string) => void;
  isRetrying: boolean;
  isCancelling: boolean;
}) {
  const groups = useDocumentGrouping(documents);

  return (
    <div className="border rounded-lg overflow-hidden shadow-sm divide-y">
      {Array.from(groups.entries()).map(([topic, docs]) => (
        <DocumentGroupAccordion
          key={topic}
          topic={topic}
          documents={docs}
          selectedIds={selectedIds}
          selectedDocument={selectedDocument}
          searchQuery={searchQuery}
          onSelect={onSelectOne}
          onClick={onRowClick}
          onDoubleClick={onRowDoubleClick}
          onViewDetails={onViewDetails}
          onViewInGraph={onViewInGraph}
          onViewPdf={onViewPdf}
          onRetry={onRetry}
          onCancel={onCancel}
          onDelete={onDelete}
          isRetrying={isRetrying}
          isCancelling={isCancelling}
        />
      ))}
    </div>
  );
}
```

- [ ] **Step 6: Type-check**

```bash
cd edgequake_webui && pnpm exec tsc --noEmit 2>&1 | tail -10
```

Expected: errors only in `DocumentManager` (missing `groupedView` prop and toggle props).

- [ ] **Step 7: Commit**

```bash
git add edgequake_webui/src/components/documents/document-table-section.tsx
git commit -m "feat(ui): add grouped render branch to DocumentTableSection"
```

---

## Task 7: Wire everything up in `DocumentManager`

**Files:**
- Modify: `edgequake_webui/src/components/documents/document-manager.tsx`

- [ ] **Step 1: Destructure `groupedView` and `setGroupedView` from preferences**

Find the line where `useDocumentPreferences` is called. It currently returns:

```ts
const {
  pageSize, setPageSize,
  statusFilter, setStatusFilter,
  sortField, setSortField,
  sortDirection, setSortDirection,
} = useDocumentPreferences();
```

Add `groupedView` and `setGroupedView`:

```ts
const {
  pageSize, setPageSize,
  statusFilter, setStatusFilter,
  sortField, setSortField,
  sortDirection, setSortDirection,
  groupedView, setGroupedView,
} = useDocumentPreferences();
```

- [ ] **Step 2: Pass props to `DocumentToolbarSection`**

Find the `<DocumentToolbarSection` JSX block. Add the two new props anywhere after the existing props:

```tsx
            groupedView={groupedView}
            onGroupedViewChange={setGroupedView}
```

- [ ] **Step 3: Pass `groupedView` to `DocumentTableSection`**

Find the `<DocumentTableSection` JSX block. Add:

```tsx
        groupedView={groupedView}
```

- [ ] **Step 4: Full type-check**

```bash
cd edgequake_webui && pnpm exec tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 5: Run all tests**

```bash
cd edgequake_webui && pnpm exec vitest run 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add edgequake_webui/src/components/documents/document-manager.tsx
git commit -m "feat(ui): wire groupedView toggle through DocumentManager"
```

---

## Verification Checklist

```bash
# 1. All tests pass
cd edgequake_webui && pnpm exec vitest run 2>&1 | grep -E "passed|failed"

# 2. TypeScript clean
pnpm exec tsc --noEmit 2>&1 | tail -5

# 3. Dev server builds
pnpm dev 2>&1 | head -20
```

Manual test:
1. Open `/` in browser
2. Upload a PDF — `enrichment_status: "pending"` appears in metadata
3. Click **Grouped** toggle → documents appear under "Tanpa Topic" (no topic yet)
4. Refresh — Grouped mode persists (localStorage)
5. Click topic header → accordion expands showing doc rows
6. Click **List** → flat table restored
