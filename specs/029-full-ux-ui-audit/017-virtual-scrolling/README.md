# 017 — Virtual Scrolling Audit

**First Principle: Flow** — Remove friction from browsing large lists.

## Problem

The current document list uses server-side pagination with:
- 10/20/50/100 rows per page
- Navigation buttons (first/prev/page/next/last)
- A count bar showing "1–20 of 20"

## Issues

### VS-01 · Pagination Is Cognitive Overhead

Users must decide: which page size? Which page? Must click to navigate.
For a knowledge tool where users scan documents by name and status, **natural scroll** is superior.

### VS-02 · Pagination Controls Waste 48px

The pagination bar at the bottom takes 48px of vertical space that could show another row.

### VS-03 · Client-Side Filter + Server Pagination Creates Inconsistency

When filtering/searching, the document count updates (e.g., "5 of 20 matching") but users must still think about pages. Natural scroll handles this transparently.

## Proposed Solution

**Large-page-size fetch + @tanstack/react-virtual**

1. Fetch all documents at once (default page_size: 500)
2. Use `useVirtualizer` to render only visible rows in the DOM
3. Remove pagination controls entirely
4. The table container gets `overflow-y: auto` and scrolls naturally

**Why not infinite scroll?**
- Knowledge graph users work with relatively small doc sets (10–500)
- Fetching all at once enables instant client-side search/filter
- @tanstack/react-virtual handles 10K+ rows with no performance cost

## Reference
- [`@tanstack/react-virtual` docs](https://tanstack.com/virtual/latest)
- Already used in: `conversation-history-panel-v2.tsx` (virtualized conversation list)
