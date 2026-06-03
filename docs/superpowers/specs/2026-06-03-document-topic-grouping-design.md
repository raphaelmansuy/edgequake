# Document Topic Grouping — Design Spec

**Date:** 2026-06-03
**Status:** Approved

---

## Overview

Add a **Grouped by Topic** view to the document list that groups documents into collapsible accordion sections based on `enrichment_topic` — the topic field written by the PDF Metadata Enrichment Pipeline. A toggle button in the toolbar switches between the existing flat list and the new grouped view. All accordion groups start collapsed.

---

## Goals

- Users can visually organise documents by automatically-extracted topic
- Switching between flat list and grouped is instant (no API call)
- No backend changes required
- Documents without a topic (enrichment pending/skipped/failed) land in a "Tanpa Topic" group at the bottom

---

## Architecture & Data Flow

```
Document[] (already fetched by useDocumentQueries)
        │
        ▼
useDocumentGrouping(documents, isGrouped)
        │  groups by enrichment_topic, "Tanpa Topic" last
        ▼
Map<string, Document[]>   (ordered: alphabetical, "Tanpa Topic" last)
        │
        ▼  when isGrouped=true
DocumentGroupAccordion × N    (collapsed by default)
        └── DocumentTableRow  (same component as flat list)
```

Toggle state is persisted via the existing `useDocumentPreferences` hook (localStorage), so the mode survives a page refresh.

---

## Files

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/types/index.ts` | Add `enrichment_topic`, `enrichment_status`, `enrichment_summary`, `enrichment_language`, `enrichment_keywords` to `Document` interface |
| Create | `src/hooks/use-document-grouping.ts` | Pure grouping logic: takes `Document[]`, returns ordered `Map<string, Document[]>` |
| Modify | `src/hooks/use-document-preferences.ts` | Add `groupedView: boolean` + `setGroupedView` |
| Modify | `src/components/documents/document-toolbar-section.tsx` | Add List / Grouped toggle button pair |
| Modify | `src/components/documents/document-table-section.tsx` | Conditionally render flat table or `DocumentGroupAccordion` list |
| Create | `src/components/documents/document-group-accordion.tsx` | Accordion component for one topic group |

---

## Component Design

### `useDocumentGrouping(documents)`

```ts
// Returns ordered Map: alphabetical topics first, "Tanpa Topic" last
function useDocumentGrouping(documents: Document[]): Map<string, Document[]>
```

- Key `"Tanpa Topic"` for docs where `enrichment_topic` is `undefined`, `null`, or empty string
- Sorts groups alphabetically; `"Tanpa Topic"` always last
- Pure/memoized — no side effects

### `useDocumentPreferences` addition

```ts
groupedView: boolean          // default false
setGroupedView: (v: boolean) => void
```

Stored in localStorage under existing key pattern of this hook.

### Toggle button (in `DocumentToolbarSection`)

```
[ ☰ List ]  [ ⊞ Grouped ]
```

- Two adjacent buttons, active one highlighted (same style as existing filter buttons)
- Placed at the right side of the toolbar, before the existing action buttons

### `DocumentGroupAccordion`

Props:
```ts
{
  topic: string           // group header label
  documents: Document[]   // docs in this group
  // all DocumentTableRow handlers passed through
  selectedIds, onSelectOne, onRowClick, onRowDoubleClick, ...
}
```

Behaviour:
- Collapsed by default (`open` state starts `false`)
- Header shows: topic name, doc count badge, chevron icon
- "Tanpa Topic" header rendered in muted color (`text-muted-foreground`)
- When expanded: renders existing `DocumentTableRow` for each doc (no duplication of row logic)
- No select-all per group (keep it simple)

### `DocumentTableSection` changes

```tsx
if (groupedView) {
  // render DocumentGroupAccordion × N
} else {
  // existing flat table (unchanged)
}
```

Pagination controls remain visible in both modes (flat list paginates as today; grouped view shows all fetched docs grouped).

---

## `Document` type additions

```ts
enrichment_status?: "pending" | "processing" | "completed" | "failed" | "skipped"
enrichment_topic?: string
enrichment_summary?: string
enrichment_language?: string
enrichment_keywords?: string[]
enrichment_completed_at?: string
enrichment_error?: string
```

---

## Error / Edge Cases

| Case | Behaviour |
|---|---|
| All docs have no topic | Single "Tanpa Topic" group |
| Only 1 topic | Single accordion group |
| `enrichment_status` = `"processing"` | Doc goes to "Tanpa Topic" (no topic yet) |
| Search active in grouped mode | Filter applies before grouping — empty groups hidden |
| Status filter active in grouped mode | Same: filter first, group after; empty groups hidden |

---

## Out of Scope

- Editing or renaming topics from the UI
- Server-side grouping / pagination per group
- Select-all within a group
- Drag-and-drop between groups
