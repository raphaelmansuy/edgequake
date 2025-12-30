# Documents Screen Audit

**Route:** `/documents`  
**Viewport(s) Tested:** 320px, 428px, 768px, 1280px, 1536px  
**UI Regions:** Header, Sidebar, Breadcrumb, Search/Filter Bar, Upload Area, Document List, Preview Panel  
**States Captured:** Default (Empty), With Search, Preview Collapsed  
**Screenshots:** `screenshots/screens/documents/`  
**Relevant Files:** `src/app/(dashboard)/documents/page.tsx`, `src/components/documents/document-manager.tsx`

---

## What I Reviewed

### Layout Structure

```
┌─────────────────────────────────────────────────────────┐
│ Header (fixed, h: 64px)                    │ API │ 🌐 ☀️ 👤│
├────────────┬────────────────────────────────────────────┤
│ Sidebar    │ Breadcrumb: EdgeQuake > Documents          │
│ w: 256px   ├────────────────────────────────────────────┤
│            │ ┌──────────────────────────────────────┐  │
│            │ │ Page Title         │ Scan │ Refresh  │  │
│            │ ├──────────────────────────────────────┤  │
│            │ │ 🔍 Search... │ Status ▼│ Sort: Created│  │
│            │ ├──────────────────────────────────────┤  │
│            │ │ ┌────────────────────────────────┐   │  │
│            │ │ │ ↑ Drag & drop files or browse  │   │ ▸│
│            │ │ │   TXT, MD, JSON (max 10MB)     │   │ P│
│            │ │ └────────────────────────────────┘   │ r│
│            │ ├──────────────────────────────────────┤ e│
│            │ │ 📄 Documents (0)                     │ v│
│            │ │ ┌────────────────────────────────┐   │ i│
│            │ │ │ 📄 No documents yet            │   │ e│
│            │ │ │    Upload documents to build   │   │ w│
│            │ │ │    your knowledge graph        │   │  │
│            │ │ └────────────────────────────────┘   │  │
│            │ └──────────────────────────────────────┘  │
└────────────┴────────────────────────────────────────────┘
```

---

## Slickness Score

| Criterion           | Score (1–5) | Notes                                     |
| ------------------- | ----------- | ----------------------------------------- |
| Visual refinement   | 4.2         | Clean layout, good use of cards           |
| Modern styling      | 4.0         | Upload area is clear and inviting         |
| Smooth interactions | 3.5         | Preview panel needs animation             |
| Professional polish | 4.0         | Good empty state, clear actions           |
| **Overall**         | **3.9**     | Good foundation, preview panel needs work |

---

## Issues

### � Critical (P0)

#### Preview Panel NOT SCROLLABLE (P0 - CRITICAL)

- **Priority:** P0 - CRITICAL - Discovered December 30, 2025 (scroll audit)
- **Severity:** 🔴 Critical
- **Location:** Document Preview Panel (right side)
- **Viewport(s) affected:** All desktop viewports
- **Root Cause:** Container has `overflow: visible` (no scroll capability)
- **Evidence:**
  - Content height: 1149px
  - Visible height: 619px
  - **530px of content hidden (46% inaccessible)**
- **Impact:**
  - Cannot scroll to see full content preview
  - Action buttons (View Details, Graph, Reprocess, Delete) may be cut off
  - Processing cost breakdown may be hidden
- **Fix Required:**
```tsx
// CURRENT (BUG)
<aside className="w-[400px] border-l bg-card flex flex-col ...">
  <div className="header">...</div>
  <div className="relative flex-1">  // ← overflow: visible
  
// SHOULD BE
<aside className="w-[400px] border-l bg-card flex flex-col ...">
  <div className="header shrink-0">...</div>
  <div className="flex-1 overflow-y-auto">  // ← Add scroll
```

---

### �🟠 Major

#### Preview Panel Collapsed State Not Obvious

- **Severity:** 🟠 Major
- **Location:** Right edge of screen
- **Viewport(s) affected:** Desktop
- **Current behavior:** Thin vertical bar with "Preview" text rotated
- **Expected behavior:** More visible collapsed indicator with hover expansion hint
- **Recommendation:** Add subtle pulse or tooltip on hover

#### No Loading Skeleton for Document List

- **Severity:** 🟠 Major
- **Location:** Documents list section
- **Viewport(s) affected:** All
- **Current behavior:** Shows "Documents (0)" immediately
- **Expected behavior:** Show skeleton rows while fetching

---

### 🟡 Minor

#### Upload Area Border Could Be More Prominent

- **Severity:** 🟡 Minor
- **Location:** Upload dropzone
- **Viewport(s) affected:** All
- **Current behavior:** Dashed border, subtle
- **Expected behavior:** Slightly more prominent border or background

#### Search Input Lacks Clear Button

- **Severity:** 🟡 Minor
- **Location:** Search input
- **Viewport(s) affected:** All
- **Current behavior:** No way to clear search except manual delete
- **Expected behavior:** Show "×" button when search has content

#### "Browse Files" Button Redundant

- **Severity:** 🟡 Minor
- **Location:** Upload area
- **Viewport(s) affected:** All
- **Current behavior:** Both the area is clickable AND there's a button
- **Expected behavior:** Either make it clearer or remove redundancy

#### Sort Buttons Could Have Active State

- **Severity:** 🟡 Minor
- **Location:** Sort controls
- **Viewport(s) affected:** Desktop
- **Current behavior:** "Créé" has arrow, "Mis à jour" does not
- **Expected behavior:** Both should show sort direction when active

---

## Recommendations

### 1. Enhance Preview Panel Collapsed State

**Change:** Make collapsed panel more discoverable

**Specifications:**

```tsx
// When collapsed, show:
// - Expand icon (ChevronLeft)
// - Subtle hover animation
// - Tooltip: "Click to expand preview"
<div
  className={cn(
    "w-10 border-l bg-card/50 flex flex-col items-center py-4",
    "cursor-pointer hover:bg-muted transition-colors",
    "hover:w-12" // Slight expand on hover
  )}
/>
```

**Animation:**

- Width transition: 40px → 48px on hover
- Duration: 150ms

**Acceptance Criteria:**

- [ ] Collapsed bar widens on hover
- [ ] Clear expand icon visible
- [ ] Tooltip appears after 500ms hover

---

### 2. Add Document List Skeleton

**Change:** Show loading skeleton while fetching documents

**Specifications:**

```tsx
{isLoading ? (
  <div className="space-y-2">
    {[1, 2, 3].map((i) => (
      <div key={i} className="flex items-center gap-3 p-3 animate-pulse">
        <div className="h-10 w-10 bg-muted rounded" />
        <div className="flex-1 space-y-2">
          <div className="h-4 w-3/4 bg-muted rounded" />
          <div className="h-3 w-1/2 bg-muted rounded" />
        </div>
      </div>
    ))}
  </div>
) : (
  // ... actual content
)}
```

**Applies to:** Document list area

**Acceptance Criteria:**

- [ ] Skeleton shows during load
- [ ] 3 placeholder rows
- [ ] Smooth transition to content

---

### 3. Add Search Clear Button

**Change:** Add clear button to search input

**Specifications:**

```tsx
<div className="relative">
  <input ... />
  {searchQuery && (
    <button
      onClick={clearSearch}
      className="absolute right-3 top-1/2 -translate-y-1/2"
      aria-label="Clear search"
    >
      <X className="h-4 w-4 text-muted-foreground" />
    </button>
  )}
</div>
```

**Acceptance Criteria:**

- [ ] Clear button appears when search has value
- [ ] Clicking clears input and results
- [ ] Keyboard accessible (focus ring)

---

### 4. Improve Upload Area Visual Hierarchy

**Change:** Make upload area more prominent

**Specifications:**

```tsx
<div className={cn(
  "border-2 border-dashed rounded-xl p-8",
  "border-muted-foreground/25",  // Slightly more visible
  "bg-muted/30",  // Add subtle background
  "hover:border-primary/50 hover:bg-muted/50",  // Hover feedback
  "transition-all duration-200"
)}>
```

**Acceptance Criteria:**

- [ ] Upload area more visually distinct
- [ ] Clear hover feedback
- [ ] Drop state is obvious (border color change)

---

## Measurements

| Element              | Current        | Recommended                         |
| -------------------- | -------------- | ----------------------------------- |
| Page padding         | 24px           | ✅ Good                             |
| Upload area height   | Auto (~100px)  | ✅ Good                             |
| Search input height  | 40px           | ✅ Good                             |
| Document row height  | ~64px          | ✅ Good                             |
| Preview panel width  | 48px collapsed | Consider 56px for better tap target |
| Gap between sections | 16px           | ✅ Good                             |

---

## Responsive Behavior

### Mobile (320-428px)

- ✅ Layout stacks vertically
- ✅ Upload area full width
- ⚠️ Preview panel should be hidden on mobile
- ⚠️ Filter controls could be in expandable section

### Tablet (768px)

- ✅ Good layout
- ✅ Search and filters side by side

### Desktop (1280px+)

- ✅ Preview panel visible
- ✅ Ample space for document list
- ⚠️ Consider max-width for content area

---

## Accessibility

| Check                    | Status        | Notes                 |
| ------------------------ | ------------- | --------------------- |
| File input accessible    | ✅ Good       | Has label association |
| Drag and drop ARIA       | ⚠️ Needs work | Add aria-dropeffect   |
| Search results announced | ⚠️ Needs work | Use aria-live region  |
| Empty state              | ✅ Good       | Clear messaging       |
| Keyboard navigation      | ✅ Good       | Can tab through       |

---

## Screenshots Reference

| State           | Breakpoint       | File                         |
| --------------- | ---------------- | ---------------------------- |
| Default (Empty) | Desktop 1280px   | `02-documents-desktop.png`   |
| Default         | Desktop L 1536px | `02-documents-desktop-l.png` |
| Default         | Tablet 768px     | `02-documents-tablet.png`    |
| Default         | Mobile L 428px   | `02-documents-mobile-l.png`  |
| Default         | Mobile S 320px   | `02-documents-mobile-s.png`  |
| Upload Area     | Desktop          | `02-documents-upload.png`    |
| Table           | Desktop          | `02-documents-table.png`     |

---

---

## December 30, 2025 - Live Testing Update

### Testing with Real Data

Tested with 2 indexed documents (AI safety research papers):

- miss_2512_21110v2.md (794 entities, $0.042 cost, 92.3K tokens)
- miss_2512.21110v2.md (5 entities, $0.00034 cost)

#### ✅ Verified Features

1. **Document List with Data**

   - Table displays correctly with columns: Title, Status, Entities, Cost, Created
   - Status badges ("Completed") render with green checkmark
   - Costs display in formatted currency ($0.042, $0.0003)
   - Row selection works with visual highlight
   - Pagination controls present at bottom

2. **Preview Panel**

   - Opens smoothly when document selected
   - Shows document metadata: Title, Status, ID, Size, Created date, Entity count
   - Processing Cost breakdown: Total cost, tokens, model info
   - Content Preview section with expandable text
   - Action buttons: View Details, Graph, Reprocess, Delete
   - Collapse toggle works correctly

3. **Document Detail Page (/documents/[id])**

   - Clean layout with header showing document info
   - Stats cards: Chunks (32), Entities (794), Relations (576), Processed time
   - Extraction Lineage section shows pipeline: Upload → Content Extraction → Entity Extraction → Relationship Mapping → Graph Indexing
   - Knowledge Graph section with entity/relation counts
   - Source Details with metadata (MIME type, content length, timestamps)
   - Processing Info with model details and entity types extracted
   - **Large document scroll works** - 66,290px scrollable content renders correctly
   - Fixed header remains in place during scroll

4. **Scroll Behavior (VERY IMPORTANT)**
   - Document detail page: Main content area scrolls independently
   - Sidebar: Fixed, does not scroll with content
   - Right metadata panel: Has internal scroll for long content
   - **Scrollable element identified:** `div.flex-1.overflow-auto` with proper overflow handling

#### 🟡 Observations

1. **Mobile Layout (320px)**

   - Layout adapts correctly
   - Preview panel hidden as expected
   - Table columns responsive (some hidden on mobile)
   - Upload area remains functional

2. **Tablet Layout (768px)**

   - Sidebar visible
   - Preview panel shows as collapsible button
   - Document table shows all critical columns

3. **Document Detail Page - Academic Paper Rendering**
   - Long academic paper (134K+ characters) renders correctly
   - Markdown formatting preserved (headers, lists, bold, links)
   - References section scrollable
   - Tables in content render properly

#### 🔴 Issues Found

1. **Status Column Not Showing Filter Count**
   - Status dropdown shows "All Status (0)" even with 2 completed documents
   - Expected: "All Status (2)" or proper count by status

2. **Preview Panel NOT SCROLLABLE (P0 - CRITICAL)** - Discovered December 30, 2025 (scroll audit)
   - Content height: 1149px, Visible: 619px → **530px hidden**
   - Root cause: `overflow: visible` on content container
   - Impact: Action buttons may be cut off, content preview truncated
   - See Issues section above for fix details

#### Fixed Zone Verification

| Zone            | Behavior                           | Status                           |
| --------------- | ---------------------------------- | -------------------------------- |
| Header          | Fixed at top                       | ✅ Correct                       |
| Sidebar         | Fixed on left                      | ✅ Correct                       |
| Breadcrumb      | Fixed below header                 | ✅ Correct                       |
| Document List   | Scrollable (not needed for 2 docs) | ✅ Correct                       |
| Preview Panel   | Fixed on right, internal scroll    | ❌ **BROKEN** - overflow:visible |
| Document Detail | Main area scrolls, header fixed    | ✅ Correct                       |

### Updated Screenshots (Dec 30)

| State           | Breakpoint     | File                                              |
| --------------- | -------------- | ------------------------------------------------- |
| List with Data  | Desktop 1280px | `documents-desktop-empty.png` (actually has data) |
| Preview Open    | Desktop 1280px | `documents-desktop-with-preview.png`              |
| Detail Page     | Desktop 1280px | `documents-detail-desktop-1280.png`               |
| Detail Scrolled | Desktop        | `documents-detail-scrolled-middle.png`            |
| Mobile          | 320px          | `documents-mobile-320.png`                        |
| Tablet          | 768px          | `documents-tablet-768.png`                        |

### Updated Score (Revised after scroll bug discovery)

| Criterion           | Previous | Current  | Change    |
| ------------------- | -------- | -------- | --------- |
| Visual refinement   | 4.2      | 4.4      | +0.2      |
| Modern styling      | 4.0      | 4.3      | +0.3      |
| Smooth interactions | 3.5      | 4.0      | +0.5      |
| Professional polish | 4.0      | 4.3      | +0.3      |
| **Scroll/Overflow** | **N/A**  | **2.5**  | **NEW**   |
| **Overall**         | **3.9**  | **3.9**  | **0.0**   |

**Score Revision Note:** Initial testing showed 4.25, but scroll audit revealed preview panel has `overflow: visible` preventing scroll when content exceeds viewport. This is a P0 bug that must be fixed.

---

_Last updated: December 30, 2025 (Revised with scroll bug findings)_
