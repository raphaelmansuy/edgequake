# Audit: Documents Page

**Component:** `src/app/(dashboard)/documents/`, `src/components/documents/`  
**Route:** `/documents`  
**Screenshot:** `e2e/screenshots/03-documents.png`

---

## Current Layout (ASCII)

```
┌─ Documents (56 ●) ──────────────────────────────────────────────────┐
│  Upload and manage documents for knowledge graph extraction          │
│                                 [ Refresh ]  [ 🗑 Clear All (RED) ] │
│  🔍 Search docs...   [ Completed (56) ▾ ]  Sort: Created ▾ Updated  │
│  ──────────────────────────────────────────────────────────────────  │
│  ┌ Drop zone: Drag & drop or click to upload ... ───────────────┐   │
│  │  Parser for this upload: [ Workspace Default ▾ ]             │   │
│  └───────────────────────────────────────────────────────────── ┘   │
│  56 of 56                                                             │
│  ┌── Title ─────────────────────────────┬ Status ─┬ Ent ┬ Cost ─┐  │
│  │  residual-risk-matrix.pdf            │✅ Done  │  98 │$0.005 │  │
│  │  math_2605.22763v1.pdf               │✅ Done  │2104 │$0.142 │  │
│  │  ...54 more rows, no pagination      │         │     │       │  │
│  └──────────────────────────────────────┴─────────┴─────┴───────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Findings

### F-DOC-01 · "Clear All" button is dangerously prominent and poorly placed · HIGH
**Problem:** The red "🗑 Clear All" button is next to "Refresh" in the top-right, with no contextual distance from the primary actions. One misclick deletes all 56 documents.  
**Principle:** Nielsen #5 — Error prevention: destructive actions must have confirmation and friction.  
**Fix:** Move to a secondary overflow menu (…) or inside a "Danger Zone" in Settings. If kept on this page, require a typed confirmation.

### F-DOC-02 · No pagination — all 56 rows shown · MED
**Problem:** "56 of 56" text shows all rows at once. At 100+ documents, the list becomes unwieldy. No pagination controls visible.  
**Fix:** Implement virtual scrolling or server-side pagination with a page size selector (25/50/100 per page).

### F-DOC-03 · "NEW" badge on timestamps adds noise · LOW
**Problem:** "NEW 31 minutes ago" in the Created column. The "NEW" label is redundant — recency is already communicated by the timestamp.

### F-DOC-04 · Cost column is a secondary concern on the primary view · LOW
**Problem:** The `$ 0.0045` cost value is shown in the main table. For most users, cost is an occasional concern, not a per-row concern.  
**Fix:** Move to a collapsible column or hide by default, show in document detail view.

### F-DOC-05 · File icon is always red PDF icon regardless of file type · LOW
**Problem:** All documents show the same red PDF file icon. TXT, MD, JSON uploads would look identical.  
**Fix:** Use type-appropriate icons (e.g., blue for TXT/MD, purple for JSON, red for PDF).

### F-DOC-06 · Action icons (external link, eye, sparkle, ellipsis) lack visual hierarchy · MED
**Problem:** Four action icons per row — `↗ 👁 ✦ ⋮` — are the same size with no grouping. Primary action is unclear.  
**Fix:** Show only the ellipsis `⋮` by default; reveal other actions on row hover.

### F-DOC-07 · Upload zone placement separates context from action · MED
**Problem:** The upload zone is placed between the filter bar and the document list. This creates a visual break in the scanning flow.  
**Fix:** Move upload zone to a persistent sticky area or a modal/drawer triggered by a prominent "Upload" button.

---

## Summary Score

| Dimension             | Score | Notes                    |
| --------------------- | ----- | ------------------------ |
| Error Prevention      | 4/10  | Clear All too accessible |
| Information Hierarchy | 6/10  | OK table structure       |
| Pagination            | 4/10  | All rows shown           |
| Action Hierarchy      | 5/10  | Too many row actions     |
| Visual Noise          | 5/10  | NEW badge, cost column   |
