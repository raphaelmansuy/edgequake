# Observation - Iteration 43

## Focus: Documents Page Accessibility Audit

## Files Examined

- `edgequake_webui/src/components/documents/quick-action-buttons.tsx` (150 lines) — Row action buttons
- `edgequake_webui/src/components/documents/document-actions-menu.tsx` (134 lines) — Dropdown actions menu
- `edgequake_webui/src/components/documents/document-search-bar.tsx` (~65 lines) — Search input
- `edgequake_webui/src/components/documents/pagination-controls.tsx` (~100 lines) — Page navigation
- `edgequake_webui/src/components/documents/document-table-section.tsx` (219 lines) — Document table

## Automated Accessibility Audit Results

Ran comprehensive JS evaluation on `http://localhost:3000/documents` checking:
images alt, button names, table semantics, input labels, live regions, headings, landmarks.

### Findings (Pre-Fix)

| Category | Issue | Count |
|----------|-------|-------|
| Buttons without accessible name | Icon-only buttons with no `aria-label` | **52** |
| Table semantics | `<table>` missing `aria-label` | **1** |
| Table headers | `<th>` missing `scope="col"` | **7** |
| Search input | `<input>` missing `aria-label` | **1** |
| Actions column header | Empty `<th>` with no screen-reader text | **1** |

### Breakdown of 52 Unnamed Buttons

| Component | Buttons per row | × Rows | = Total |
|-----------|----------------|--------|---------|
| ActionButton (View Details, Preview, View in Graph) | 3 | × 12 | = 36 |
| ActionButton (Retry for failed docs) | 1 | × 1 | = 1 |
| MoreVertical menu trigger | 1 | × 12 | = 12 |
| Pagination (First, Prev, Next, Last) | 4 | × 1 | = 4 |
| **Total** | | | **53** |

Note: 1 fewer than expected due to pagination button having some text content.

## Root Cause

- `ActionButton` component in `quick-action-buttons.tsx` renders `<Button variant="ghost" size="icon">` with only an icon child — the `label` prop is used for tooltip text but not passed as `aria-label`
- DropdownMenuTrigger in `document-actions-menu.tsx` renders `<Button>` with only `<MoreVertical />` icon
- Search `<Input>` has `placeholder` but no `aria-label` (placeholder is not a reliable accessible name)
- Pagination buttons use `<ChevronLeft />` etc. with no label
