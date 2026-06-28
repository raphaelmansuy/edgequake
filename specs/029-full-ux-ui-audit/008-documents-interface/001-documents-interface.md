# 001 — Documents Interface Audit

**First Principle: Flow** — The upload-to-query workflow should be frictionless.

---

## Page Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│  HEADER (DocumentHeader)                                            │
│  "Documents"  [⬆ Upload] [⟳ Scan] [PDF Mode ▼] [Filter ▼] [⋮]    │
├──────────────────────────────────────────────────────────────────────┤
│  TOOLBAR (DocumentToolbarSection)                                   │
│  [🔍 Search...]  {when items selected → batch bar slides in}       │
├──────────────────────────────────────────────────────────────────────┤
│  TABLE (DocumentTableSection)                                       │
│  [☐] Name                 Status     Chunks  Cost    Date    [⋯]   │
│  ─────────────────────────────────────────────────────────────────  │
│  [☐] 📄 research.pdf      ✅ Done     42     $0.02   2h ago  [⋯]  │
│  [☐] 📄 report.pdf        🧠 Extract  -       -      1m ago  [⋯]  │
│  [☐] 📝 notes.md          ⏳ Queue    -       -      5m ago  [⋯]  │
├──────────────────────────────────────────────────────────────────────┤
│  PAGINATION                                                         │
│  Rows/page: [20 ▼]   (47 total)    ← 1/3 →  »                    │
└──────────────────────────────────────────────────────────────────────┘
```

---

## Issues

### DI-01 · Toolbar Control Count

The document header has **5 controls** before counting the search bar:

1. Upload button
2. Scan button
3. PDF Mode selector
4. Filter dropdown
5. More menu (⋮)

Plus the search bar in the toolbar section = **6 controls total** in the top area.

**Frequency analysis:**

| Control | Frequency | Visibility Priority |
|---------|-----------|-------------------|
| Upload | High | P1 — always visible, prominent |
| Search | High | P1 — always visible |
| Filter | Medium | P2 — visible but compact |
| Scan | Low | P3 — in overflow menu |
| PDF Mode | Low (power user) | P3 — in overflow menu |
| Sort | Medium | P2 — combined with filter |

**Fix:** Consolidate into:

```
PRIMARY: [⬆ Upload]  [🔍 Search...]  [Filter: All ▼]  [⋮]
OVERFLOW (⋮): Scan, PDF Mode, Export, Import
```

### DI-02 · Dual Toolbar Problem

When items are selected, a batch action bar appears below the main toolbar. This creates two toolbar rows simultaneously, which is visually cluttered.

```
CURRENT (selection active):
─────────────────────────────────────────────────────────
[⬆ Upload] [⟳ Scan] [PDF ▼] [Filter ▼] [⋮]
[🔍 Search...]
─────────────────────────────────────────────────────────
↓ selection bar slides in:
[2 selected] [Reprocess] [Delete] [✕ Deselect]
─────────────────────────────────────────────────────────
```

**Better pattern: Replace the header with selection context**

```
SELECTION MODE (replaces header, animated transition):
─────────────────────────────────────────────────────────
[✕] 2 documents selected    [Reprocess] [Delete]
─────────────────────────────────────────────────────────
```

This is the pattern used by Gmail, Notion, and Linear — the selection bar replaces (rather than supplements) the toolbar.

### DI-03 · Table Column Width Distribution

The current table has columns: Checkbox | Name | Status | Chunks | Cost | Date | Actions

The `Name` column should get dominant width (flex-grow), with secondary columns being compact.

**Proposed width allocation:**

```
Column         Width       Priority
────────────────────────────────────────────
☐ Checkbox     32px        Required
Name           flex-grow   Primary (most important)
Status         120px       Primary (always show)
Date           100px       Secondary (relative time)
Actions        48px        Required
────────────────────────────────────────────
Chunks         72px        Hidden by default (column toggle)
Cost           80px        Hidden by default (column toggle)
```

Add a column visibility toggle (⊞ columns) in the toolbar overflow menu.

### DI-04 · Status Badge: 15 States is Too Many

```
Current statuses with distinct visual treatment:
uploading, queued, converting, preprocessing, chunking, extracting,
gleaning, merging, summarizing, embedding, storing, completed, failed,
partial_failure, partial_success, cancelled
```

From a UX perspective, users care about:
1. **Is it done?** → completed / failed / partial
2. **Is it working?** → any in-progress state
3. **Is it stuck?** → queued > threshold

The 10 in-progress states are meaningful for debugging but overwhelming for monitoring.

**Proposed: Two-tier status display**

```typescript
// User-facing: 4 macro states
type MacroStatus = 'waiting' | 'processing' | 'done' | 'failed';

// Developer detail: expandable
// Click status badge → see detailed stage in tooltip or side panel
```

```
TABLE VIEW:          EXPANDED (tooltip/panel):
⏳ Processing    →   Extracting entities... (stage 6/11)
```

### DI-05 · Pagination: Missing Total Items Context

```
CURRENT:
Rows/page: [20 ▼]   (47 total)    Page 1 of 3   ← 1/3 →  »

PROBLEM: "47 total" appears in parentheses, visually weak
```

**Fix:** Make total count more prominent and provide context:

```
Showing 1–20 of 47 documents    [← Prev]  1/3  [Next →]
```

This follows Gmail/Google Sheets pagination pattern where "1–20 of 47" immediately communicates scope.

### DI-06 · Row Double-Click Interaction

The table rows support:
- Single click → opens preview panel  
- Double click → navigates to document detail

Double-click is a **non-discoverable interaction pattern** — users won't know it exists. This functionality needs to be surfaced:

1. Add tooltip to row: "Click to preview · Double-click to open"
2. OR: Add a visible "Open" action in the actions menu (⋯) — which may already exist

### DI-07 · Upload Progress Feedback

When uploading, `uploadingFiles` adds temporary rows to the table with progress indicators. Issues:

- Upload progress is a percentage bar in a table row — rows are not designed for this
- Multiple files uploading simultaneously creates a visually noisy table

**Better pattern:** Use a **persistent upload progress panel** that slides in from the bottom-right (like VS Code's progress indicator or Figma's export progress):

```
┌─────────────────────────────────────┐
│  Uploading 3 files...               │
│  ─────────────────────────────────  │
│  📄 research.pdf     ████████░░ 78% │
│  📄 report.pdf       ██░░░░░░░░ 22% │
│  📝 notes.md         ✓ Complete     │
└─────────────────────────────────────┘
```

### DI-08 · Search Bar: No Debounce Indication

The `document-search-bar.tsx` presumably fires on every keystroke. If it queries the backend (not just client-side filtering), there should be a debounce and a loading indicator in the search input.

```
[🔍 Loading... ○]  ← spinner in search input during debounced query
```

---

## Positive Findings

```
✅ Skeleton loading matches table structure
✅ Filter-aware empty state (distinguishes "no docs" from "filtered")
✅ Search highlight in document names (highlightMatches function)
✅ File type icons with color coding
✅ Relative timestamps (formatDistanceToNow)
✅ Batch selection with keyboard support (useDocumentKeyboard hook)
✅ Duplicate upload detection dialog
✅ Reprocess choice dialog (full re-extract vs. entities only)
✅ Stuck detection hook (useStuckDetection)
✅ WebSocket real-time status updates
✅ Multiple sort fields supported
```

---

## Upload Flow UX Analysis

### Current Upload Flow

```
1. User clicks [Upload] button
2. File input opens (or drag-drop on table)
3. File(s) selected → handleFilesUpload()
4. If duplicate → DuplicateUploadDialog
5. Upload starts → uploadingFiles rows appear in table
6. Processing begins → status badge updates via WebSocket
7. Completion → row moves from "processing" to "completed"
```

**Issues in the flow:**

- Step 3→4: Duplicate dialog is modal — if uploading 10 files with 3 duplicates, user gets 3 sequential dialogs (each blocks the next)
- Step 5: The uploading row appears in the documents table as a first-class row — this is technically correct but visually jarring (a progress bar in a table cell)
- Step 6→7: Status updates may lag (WebSocket reconnection issues) — users may see stale "processing" states

### Recommended: Unified Upload Drop Zone

Consider a persistent drop zone at the top of the documents page (collapsed by default):

```
┌──────────────────────────────────────────────────────────────────┐
│  [Drag files here or click to upload]                (collapsed) │
└──────────────────────────────────────────────────────────────────┘
```

This replaces the header [Upload] button and makes the drop affordance immediately visible.

---

## External References

- [Data Table Design Patterns — NNGroup](https://www.nngroup.com/articles/data-tables/)
- [File Upload UX Best Practices — Smashing Magazine](https://www.smashingmagazine.com/2018/09/ux-design-file-upload/)
- [Batch Actions in UX — UX Planet](https://uxplanet.org/batch-actions-ux-92cdafad5a65)
- [Linear Table Component](https://linear.app/) — reference for clean data tables
