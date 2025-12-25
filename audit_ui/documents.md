# Documents/Upload Screen Audit

## Screen: Documents Management (`/documents`)

**Screenshot References:**
- [`02-documents-full.png`](../audit_ui/screenshots/02-documents-full.png)
- [`02-documents-list.png`](../audit_ui/screenshots/02-documents-list.png)
- [`02-documents-upload-area.png`](../audit_ui/screenshots/02-documents-upload-area.png)

**Component Files:**
- Page: [`src/app/(dashboard)/documents/page.tsx`](../edgequake_webui/src/app/(dashboard)/documents/page.tsx)
- Main Component: [`src/components/documents/document-manager.tsx`](../edgequake_webui/src/components/documents/document-manager.tsx)
- Related Components: [`src/components/documents/`](../edgequake_webui/src/components/documents/)

---

## What I Reviewed

### UI Regions Analyzed:
1. **Upload Area** - Drag-and-drop zone with file input
2. **Filters & Actions Bar** - Status filter, sort controls, bulk actions
3. **Document Table** - List view with columns: Title, Status, Size, Date, Actions
4. **Pagination Controls** - Page navigation and items per page selector
5. **Upload Progress Cards** - Real-time upload feedback (when uploading)
6. **Dialogs/Modals:**
   - Document detail view
   - Pipeline status
   - Clear documents confirmation
   - Reprocess failed documents

### Measurements:
- Table takes full content width (~1664px on desktop)
- Upload area height: varies based on content/empty state
- No right panel for document preview (missed opportunity)

---

## Issues

### 🔴 Critical

**C1. No Right Panel for Document Preview**
- **Location:** Document manager layout
- **Issue:** When clicking on a document or selecting it, there's no quick preview panel
- **Current:** Opens a modal dialog ([`document-detail-dialog.tsx`](../edgequake_webui/src/components/documents/document-detail-dialog.tsx))
- **Impact:** 
  - Modal blocks the entire view, requiring close to see list
  - Can't compare multiple documents side-by-side
  - Modal UI is modal for a non-critical action
- **Better UX:** Right sliding panel (like Gmail) with document preview, metadata, and actions

**C2. Upload Area Not Prominent Enough**
- **Location:** Top of document manager
- **Issue:** Upload dropzone is small and doesn't stand out visually
- **Evidence:** Screenshot shows minimal upload UI
- **Impact:** Users might not realize they can drag-and-drop files
- **Expected:** Large, inviting drop zone with clear visual cues (especially when empty)

**C3. No Visual Feedback for Empty State**
- **Location:** Document table when no documents exist
- **Issue:** Based on code review, empty state exists but lacks visual impact
- **Expected:** Large illustration, helpful onboarding text, primary "Upload" CTA

### 🟡 Major

**M1. Table Column Width Not Optimized**
- **Location:** Document table
- **Issue:**
  - Filename column takes excessive space
  - Status column too narrow for badge
  - Actions column wastes space with dropdown
- **Recommendation:** Fixed column widths with proper proportions

**M2. Status Filter UX Weak**
- **Location:** Filter bar above table
- **Issue:** 
  - Filter is a dropdown, requires click to see options
  - No visual indication of current filter (except dropdown label)
  - No "active filter" badges or clear affordance
- **Better UX:** Tab-based filter (All | Pending | Processing | Completed | Failed) with counts

**M3. Bulk Actions Not Implemented**
- **Location:** Table rows
- **Issue:** Code comment says "TODO: Implement bulk selection in future"
- **Impact:** Must delete/reprocess documents one by one
- **Expected:** 
  - Checkbox column for selection
  - Bulk action bar appears when items selected
  - Actions: Delete Selected, Reprocess Selected, Export Selected

**M4. Pagination Feels Detached**
- **Location:** Bottom of page
- **Issue:** Pagination controls are visually separated from table
- **Evidence:** No visual connection (border, background) between table and pagination
- **Better UX:** Integrate pagination into table footer with shared border/background

**M5. Upload Progress Cards Lack Context**
- **Location:** Batch progress area during upload
- **Issue:** Progress cards show individual file status but:
  - No overall batch progress indicator
  - Can't cancel individual files mid-upload
  - Cards take up significant vertical space
- **Recommendation:** Compact accordion-style progress with overall batch summary

**M6. Document Actions Buried in Dropdown**
- **Location:** Actions column (MoreVertical icon)
- **Issue:**
  - Common actions (View, Delete) require dropdown click
  - No quick action buttons visible
  - Dropdown feels like an anti-pattern for 2-3 actions
- **Better UX:** Show Delete and View icons directly, use dropdown only for 3+ actions

### 🟢 Minor

**m1. No Search/Filter by Filename**
- **Location:** Filter bar
- **Issue:** Can filter by status but not search by filename
- **Expected:** Search input at top: "Search documents..."

**m2. Date Format Not User-Friendly**
- **Location:** Created/Updated date columns
- **Issue:** Uses `formatDistanceToNow` (e.g., "2 hours ago") which is good, but no tooltip with exact timestamp
- **Recommendation:** Show relative time, add tooltip with full timestamp

**m3. File Size Display Could Be Better**
- **Location:** Size column
- **Issue:** May show bytes/KB without proper formatting
- **Recommendation:** Format as "1.2 MB", "456 KB", etc.

**m4. No Document Type/Format Indication**
- **Location:** Table rows
- **Issue:** No icon or badge showing file type (PDF, TXT, MD, etc.)
- **Recommendation:** Add file type icon before filename

**m5. Status Badge Colors Not Accessible**
- **Location:** Status column badges
- **Issue:** Color alone conveys information (fails WCAG)
- **Recommendation:** Icons already present (good!), ensure sufficient contrast

**m6. Pipeline Status Dialog Hidden**
- **Location:** Header or action bar
- **Issue:** [`pipeline-status-dialog.tsx`](../edgequake_webui/src/components/documents/pipeline-status-dialog.tsx) exists but not clear how to access it
- **Recommendation:** Add "View Pipeline Status" button or link in header

**m7. Filters Don't Show Active State Clearly**
- **Location:** Filter dropdowns
- **Issue:** When a filter is applied, hard to tell at a glance
- **Recommendation:** Show active filters as dismissible pills below filter bar

**m8. No Keyboard Shortcuts**
- **Location:** Document manager
- **Issue:** No keyboard shortcuts for common actions (Upload: U, Delete: Del, etc.)
- **Recommendation:** Add keyboard shortcuts and show them in tooltips

---

## Recommendations

### For Right Panel (Document Preview)

**R1. Add Collapsible Right Panel for Document Details** ⭐ **PRIORITY**
```
┌──────────┬────────────────────────────────┬─────────────────┐
│ Sidebar  │     Document Table             │  Document       │
│          │                                │  Preview        │
│          │  [Upload Area]                 │                 │
│          │  [Filters & Actions]           │  Title          │
│          │  ┌─────────────────────────┐  │  Status: ✓      │
│          │  │ Row 1 (selected) ◄──────┼──┤  Size: 1.2 MB   │
│          │  │ Row 2                   │  │  Created: ...   │
│          │  │ Row 3                   │  │                 │
│          │  └─────────────────────────┘  │  [Preview]      │
│          │  [Pagination]                  │  Content...     │
│          │                                │                 │
│          │                                │  [Actions]      │
└──────────┴────────────────────────────────┴─────────────────┘
```

**Panel Width:** 400px (wider than dashboard right panel for content preview)
**Default State:** Collapsed (opens when document selected)
**Content:**
- Document metadata (title, size, type, dates)
- Status with progress indicator
- Content preview (first 500 chars with "View Full" button)
- Quick actions (Delete, Reprocess, Download)
- Related entities (if extracted)
- Processing log/errors (if any)

**Interaction:**
- Click row → Open right panel with that document
- Click again or click X → Close panel
- Arrow keys → Navigate between documents while panel open
- ESC → Close panel

**R2. Keep Modal for Full Document View**
- Right panel shows **preview** (metadata + snippet)
- Modal shows **full document content** (when "View Full" clicked)
- Modal can also show full processing logs, entity graph, etc.

### For Upload Area

**R3. Dramatically Improve Upload Zone (Empty State)** ⭐ **PRIORITY**

**When No Documents:**
```tsx
<Card className="border-2 border-dashed border-muted-foreground/20 bg-muted/5">
  <CardContent className="flex flex-col items-center justify-center py-16 text-center">
    <div className="rounded-full bg-primary/10 p-6 mb-4">
      <Upload className="h-12 w-12 text-primary" />
    </div>
    <h3 className="text-xl font-semibold mb-2">
      Upload Your First Document
    </h3>
    <p className="text-muted-foreground mb-6 max-w-md">
      Drag and drop files here, or click to browse. 
      Supported formats: TXT, PDF, MD, DOCX.
    </p>
    <div className="flex gap-3">
      <Button size="lg" className="gap-2">
        <Upload className="h-4 w-4" />
        Browse Files
      </Button>
      <Button size="lg" variant="outline" className="gap-2">
        <FileText className="h-4 w-4" />
        Paste Text
      </Button>
    </div>
    <p className="text-xs text-muted-foreground mt-6">
      Maximum file size: 10MB per file
    </p>
  </CardContent>
</Card>
```

**When Documents Exist:**
```tsx
<Card className="border-dashed hover:border-primary/50 hover:bg-primary/5 transition-colors cursor-pointer">
  <CardContent className="flex items-center gap-4 py-6">
    <div className="rounded-lg bg-primary/10 p-3">
      <Upload className="h-6 w-6 text-primary" />
    </div>
    <div className="flex-1">
      <p className="font-medium">Upload Documents</p>
      <p className="text-sm text-muted-foreground">
        Drag files here or click to browse
      </p>
    </div>
    <Button variant="outline" size="sm">Browse</Button>
  </CardContent>
</Card>
```

**Drag-Over State:**
- Border becomes solid primary color
- Background: `bg-primary/10`
- Show large upload icon overlay
- Prevent dropping on table rows (only in drop zone)

### For Table & Layout

**R4. Implement Tab-Based Status Filter** ⭐ **PRIORITY**
```tsx
<Tabs value={statusFilter} onValueChange={setStatusFilter}>
  <TabsList>
    <TabsTrigger value="all">
      All <Badge variant="secondary" className="ml-2">{totalCount}</Badge>
    </TabsTrigger>
    <TabsTrigger value="processing">
      Processing <Badge variant="secondary" className="ml-2">{processingCount}</Badge>
    </TabsTrigger>
    <TabsTrigger value="completed">
      Completed <Badge variant="secondary" className="ml-2">{completedCount}</Badge>
    </TabsTrigger>
    <TabsTrigger value="failed">
      Failed <Badge variant="destructive" className="ml-2">{failedCount}</Badge>
    </TabsTrigger>
  </TabsList>
</Tabs>
```

**Benefits:**
- Visual at-a-glance status overview
- Shows counts for each status
- Clearer active state
- More space-efficient than dropdown

**R5. Optimize Table Column Widths**
```tsx
<Table>
  <TableHeader>
    <TableRow>
      <TableHead className="w-[40px]">
        {/* Future: Checkbox for bulk selection */}
      </TableHead>
      <TableHead className="w-[40%]">Title</TableHead>
      <TableHead className="w-[120px]">Status</TableHead>
      <TableHead className="w-[100px]">Size</TableHead>
      <TableHead className="w-[140px]">Uploaded</TableHead>
      <TableHead className="w-[100px] text-right">Actions</TableHead>
    </TableRow>
  </TableHeader>
</Table>
```

**R6. Add Search Input**
```tsx
<div className="flex items-center gap-4 mb-4">
  <div className="flex-1 max-w-md">
    <div className="relative">
      <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
      <Input 
        placeholder="Search documents..." 
        className="pl-9"
        value={searchQuery}
        onChange={(e) => setSearchQuery(e.target.value)}
      />
    </div>
  </div>
  
  <Tabs value={statusFilter} onValueChange={setStatusFilter}>
    {/* Status tabs */}
  </Tabs>
  
  <DropdownMenu>
    <DropdownMenuTrigger asChild>
      <Button variant="outline" size="sm" className="gap-2">
        <SortAsc className="h-4 w-4" />
        Sort: {sortLabel}
      </Button>
    </DropdownMenuTrigger>
    {/* Sort options */}
  </DropdownMenu>
</div>
```

**R7. Improve Table Row Actions**
```tsx
<TableCell className="text-right">
  <div className="flex items-center justify-end gap-1">
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button 
            variant="ghost" 
            size="sm" 
            onClick={() => handleViewDocument(doc)}
          >
            <Eye className="h-4 w-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>View document</TooltipContent>
      </Tooltip>
    </TooltipProvider>
    
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button 
            variant="ghost" 
            size="sm" 
            onClick={() => handleDeleteDocument(doc.id)}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Delete document (Shift+Del)</TooltipContent>
      </Tooltip>
    </TooltipProvider>
    
    {doc.status === 'failed' && (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button 
              variant="ghost" 
              size="sm" 
              onClick={() => handleReprocess(doc.id)}
            >
              <RefreshCw className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Reprocess document</TooltipContent>
        </Tooltip>
      </TooltipProvider>
    )}
    
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm">
          <MoreVertical className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem>Download</DropdownMenuItem>
        <DropdownMenuItem>View in Graph</DropdownMenuItem>
        <DropdownMenuItem>Copy ID</DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem className="text-destructive">
          Delete Permanently
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  </div>
</TableCell>
```

**Key Changes:**
- Show View and Delete icons directly (most common actions)
- Show Reprocess icon for failed documents
- Move less common actions to dropdown
- Add tooltips with keyboard shortcuts
- Visual hierarchy: primary actions more prominent

**R8. Add Bulk Selection**
```tsx
// Checkbox column in header
<TableHead className="w-[40px]">
  <Checkbox 
    checked={selectedIds.size === documents.length && documents.length > 0}
    onCheckedChange={handleSelectAll}
    aria-label="Select all documents"
  />
</TableHead>

// Checkbox column in each row
<TableCell className="w-[40px]">
  <Checkbox 
    checked={selectedIds.has(doc.id)}
    onCheckedChange={() => handleToggleSelection(doc.id)}
    aria-label={`Select ${doc.title}`}
  />
</TableCell>

// Bulk action bar (appears when items selected)
{selectedIds.size > 0 && (
  <Card className="fixed bottom-4 left-1/2 -translate-x-1/2 shadow-lg z-50">
    <CardContent className="flex items-center gap-4 py-3 px-6">
      <p className="text-sm font-medium">
        {selectedIds.size} document(s) selected
      </p>
      <div className="flex gap-2">
        <Button variant="outline" size="sm" onClick={handleBulkDelete}>
          <Trash2 className="h-4 w-4 mr-2" />
          Delete
        </Button>
        <Button variant="outline" size="sm" onClick={handleBulkReprocess}>
          <RefreshCw className="h-4 w-4 mr-2" />
          Reprocess
        </Button>
        <Button variant="ghost" size="sm" onClick={handleClearSelection}>
          <X className="h-4 w-4 mr-2" />
          Clear
        </Button>
      </div>
    </CardContent>
  </Card>
)}
```

### For Upload Experience

**R9. Improve Batch Upload Progress**
```tsx
<Card>
  <CardHeader className="pb-3">
    <div className="flex items-center justify-between">
      <CardTitle className="text-base">
        Uploading {uploadingFiles.length} file(s)
      </CardTitle>
      <Button variant="ghost" size="sm" onClick={handleCancelBatch}>
        <X className="h-4 w-4" />
      </Button>
    </div>
    <Progress value={overallProgress} className="h-2 mt-2" />
    <p className="text-xs text-muted-foreground mt-1">
      {completedFiles}/{uploadingFiles.length} completed • {Math.round(overallProgress)}%
    </p>
  </CardHeader>
  <Collapsible>
    <CollapsibleTrigger asChild>
      <Button variant="ghost" size="sm" className="w-full">
        <ChevronDown className="h-4 w-4 mr-2" />
        View individual files
      </Button>
    </CollapsibleTrigger>
    <CollapsibleContent>
      <ScrollArea className="h-[200px]">
        <CardContent className="space-y-2 pt-0">
          {uploadingFiles.map((file, index) => (
            <div key={index} className="flex items-center gap-3">
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate">{file.file.name}</p>
                <Progress value={file.progress} className="h-1 mt-1" />
                <p className="text-xs text-muted-foreground">{file.phase}</p>
              </div>
              {file.status === 'error' && (
                <AlertCircle className="h-4 w-4 text-destructive" />
              )}
              {file.status === 'success' && (
                <CheckCircle className="h-4 w-4 text-green-500" />
              )}
            </div>
          ))}
        </CardContent>
      </ScrollArea>
    </CollapsibleContent>
  </Collapsible>
</Card>
```

**Key Improvements:**
- Overall batch progress bar at top
- Individual files collapsed by default (saves space)
- Can expand to see details
- Shows phase for each file
- Option to cancel entire batch

---

## Rationale

### Why Right Panel for Preview
- **Faster workflow:** No modal opening/closing
- **Comparison:** Can see list and details simultaneously
- **Modern pattern:** Used by Gmail, Slack, GitHub
- **Context preservation:** Don't lose place in list

### Why Tab-Based Filters
- **Visual clarity:** See all options at once
- **Status overview:** Counts show data distribution
- **Faster interaction:** Single click vs. two (open dropdown + select)
- **Better affordance:** Tabs clearly communicate "one active at a time"

### Why Bulk Selection Matters
- **Efficiency:** Managing 100+ documents one-by-one is painful
- **Common task:** Deleting test documents, reprocessing failed batches
- **Expected feature:** Users expect bulk actions in any list view
- **Power user friendly:** Keyboard selection (Shift+Click, Ctrl+A)

### Why Improved Upload Zone
- **Discoverability:** Large, clear CTA helps first-time users
- **Confidence:** Clear messaging (file types, size limits) reduces errors
- **Feedback:** Drag-over state confirms drop will work
- **Multi-path:** Support drag-drop, browse, and paste

---

## Acceptance Criteria

### AC1: Right Panel for Document Preview
- [ ] Right panel (400px) slides in from right when document clicked
- [ ] Shows document metadata (title, status, size, dates)
- [ ] Shows content preview (first 500 chars)
- [ ] Has "View Full" button to open modal with complete content
- [ ] Shows quick actions (Delete, Reprocess, Download)
- [ ] Can navigate between documents with arrow keys while panel open
- [ ] Panel closes on ESC or when clicking outside
- [ ] Panel state persists on page navigation (if document still selected)
- [ ] Smooth transition animation (200ms)

### AC2: Enhanced Upload Zone
- [ ] **Empty state:** Large card with upload icon, title, description, primary CTA
- [ ] **With documents:** Compact card with clear drop affordance
- [ ] **Drag-over state:** Border and background change to indicate drop target
- [ ] Shows supported file types and size limits
- [ ] Has both "Browse" button and "Paste Text" option
- [ ] Browse opens native file picker with correct filters
- [ ] Paste opens dialog to paste plain text content

### AC3: Tab-Based Status Filter
- [ ] Tabs show: All, Processing, Completed, Failed
- [ ] Each tab shows count badge
- [ ] Failed tab uses destructive variant (red)
- [ ] Active tab has clear visual indication
- [ ] Clicking tab filters table immediately
- [ ] URL updates with filter param (for bookmarking)

### AC4: Search Functionality
- [ ] Search input at top of page
- [ ] Placeholder: "Search documents..."
- [ ] Searches document titles (case-insensitive)
- [ ] Search updates as user types (debounced 300ms)
- [ ] Works in combination with status filter
- [ ] Clear button appears when search has text
- [ ] Focus on CMD/CTRL+K (or F)

### AC5: Optimized Table
- [ ] Fixed column widths as specified
- [ ] Title column truncates long names with tooltip
- [ ] File type icon before title
- [ ] Status badge with icon and color
- [ ] File size formatted (KB, MB, GB)
- [ ] Date shows relative time ("2 hours ago") with tooltip for exact timestamp
- [ ] Row hover state visible
- [ ] Selected row has distinct background (for right panel)

### AC6: Improved Row Actions
- [ ] View (Eye) icon button always visible
- [ ] Delete (Trash) icon button always visible
- [ ] Reprocess (RefreshCw) icon button visible only for failed documents
- [ ] Dropdown (MoreVertical) for less common actions
- [ ] All buttons have tooltips
- [ ] Tooltips show keyboard shortcuts where applicable
- [ ] Icon buttons use ghost variant
- [ ] Actions right-aligned in column

### AC7: Bulk Selection
- [ ] Checkbox column added (40px width)
- [ ] Header checkbox selects/deselects all visible documents
- [ ] Row checkboxes toggle individual selection
- [ ] Shift+Click selects range
- [ ] CMD/CTRL+A selects all (when table focused)
- [ ] Selected count shown in floating action bar
- [ ] Action bar appears at bottom-center when items selected
- [ ] Bulk actions: Delete, Reprocess, Clear Selection
- [ ] Confirmation dialog for destructive bulk actions

### AC8: Enhanced Upload Progress
- [ ] Overall batch progress bar
- [ ] Shows: "Uploading X file(s)" with percentage
- [ ] Individual file list collapsible (collapsed by default)
- [ ] Expanded view shows per-file progress and phase
- [ ] Success/error icons for each file
- [ ] Option to cancel entire batch
- [ ] Toast notification on completion (success/error summary)
- [ ] Failed files shown with error message

### AC9: Keyboard Shortcuts
- [ ] `U` - Open upload dialog
- [ ] `Del` or `Backspace` - Delete selected document(s)
- [ ] `R` - Reprocess selected failed document
- [ ] `CMD/CTRL+K` or `F` - Focus search
- [ ] `ESC` - Close right panel or clear selection
- [ ] `↑`/`↓` - Navigate documents (when right panel open)
- [ ] `Space` - Toggle document selection (checkbox)
- [ ] `CMD/CTRL+A` - Select all visible documents

---

## ASCII Layout Diagram

### With Right Panel Open
```
┌────────────┬───────────────────────────────────────┬──────────────────┐
│  Sidebar   │           Documents                   │   Document       │
│            ├───────────────────────────────────────┤   Preview        │
│            │ Upload Area (compact)                 │                  │
│            ├───────────────────────────────────────┤   Title          │
│            │ [Search] [All|Processing|...]         │   "example.txt"  │
│            ├───────────────────────────────────────┤                  │
│            │ ┌───┬──────────┬────────┬─────────┐  │   Status: ✓      │
│            │ │ ☐ │ Title    │ Status │ Actions │  │   Size: 1.2 MB   │
│            │ ├───┼──────────┼────────┼─────────┤  │   Created: ...   │
│            │ │ ☐ │ Doc 1    │   ✓    │ 👁 🗑   │  │                  │
│            │ │ ☑ │ Doc 2 ◄──┼────────┼─────────┼──┤   Content:       │
│            │ │ ☐ │ Doc 3    │   ⏳   │ 👁 🗑   │  │   Lorem ipsum... │
│            │ └───┴──────────┴────────┴─────────┘  │   [View Full]    │
│            │                                       │                  │
│            │ ← 1 2 3 ... 10 →  [20 per page ▾]   │   Actions:       │
│            │                                       │   [Delete]       │
│            │                                       │   [Reprocess]    │
│            │                                       │   [Download]     │
└────────────┴───────────────────────────────────────┴──────────────────┘
                                                          400px
```

### Empty State
```
┌────────────┬─────────────────────────────────────────────────────────┐
│  Sidebar   │                    Documents                            │
│            ├─────────────────────────────────────────────────────────┤
│            │                                                         │
│            │    ┌────────────────────────────────────────────┐      │
│            │    │                                            │      │
│            │    │           📤  (Upload Icon)                │      │
│            │    │                                            │      │
│            │    │      Upload Your First Document            │      │
│            │    │                                            │      │
│            │    │   Drag and drop files here, or click to    │      │
│            │    │   browse. Supported: TXT, PDF, MD, DOCX.   │      │
│            │    │                                            │      │
│            │    │      [📤 Browse Files] [📄 Paste Text]     │      │
│            │    │                                            │      │
│            │    │      Maximum file size: 10MB per file      │      │
│            │    │                                            │      │
│            │    └────────────────────────────────────────────┘      │
│            │                                                         │
└────────────┴─────────────────────────────────────────────────────────┘
```

### With Bulk Selection
```
┌────────────┬───────────────────────────────────────────────────────┐
│  Sidebar   │           Documents                                   │
│            ├───────────────────────────────────────────────────────┤
│            │ [Search] [All|Processing|Completed|Failed]            │
│            ├───────────────────────────────────────────────────────┤
│            │ ┌───┬──────────────────┬────────┬─────────────────┐  │
│            │ │ ☑ │ Title            │ Status │ Actions         │  │
│            │ ├───┼──────────────────┼────────┼─────────────────┤  │
│            │ │ ☑ │ Selected Doc 1   │   ✓    │ 👁 🗑 ⋮         │  │
│            │ │ ☑ │ Selected Doc 2   │   ✓    │ 👁 🗑 ⋮         │  │
│            │ │ ☐ │ Unselected Doc   │   ✓    │ 👁 🗑 ⋮         │  │
│            │ └───┴──────────────────┴────────┴─────────────────┘  │
│            │                                                       │
│            │             ╔═══════════════════════════╗             │
│            │             ║ 2 documents selected      ║             │
│            │             ║ [🗑 Delete] [🔄 Reprocess]║             │
│            │             ║          [✕ Clear]        ║             │
│            │             ╚═══════════════════════════╝             │
│            │                  ↑ Floating action bar                │
└────────────┴───────────────────────────────────────────────────────┘
```

---

## Related Files & Components

### Components to Modify:
- ✏️ [`src/components/documents/document-manager.tsx`](../edgequake_webui/src/components/documents/document-manager.tsx) - Major refactor
- ✏️ [`src/components/documents/document-filters.tsx`](../edgequake_webui/src/components/documents/document-filters.tsx) - Convert to tabs + search
- ✏️ [`src/components/documents/batch-progress-card.tsx`](../edgequake_webui/src/components/documents/batch-progress-card.tsx) - Collapsible progress

### New Components to Create:
- 🆕 `src/components/documents/document-preview-panel.tsx` - Right panel
- 🆕 `src/components/documents/bulk-action-bar.tsx` - Floating action bar
- 🆕 `src/components/documents/upload-zone.tsx` - Enhanced upload area
- 🆕 `src/components/documents/document-table.tsx` - Extract table to separate component
- 🆕 `src/components/documents/status-tabs.tsx` - Tab-based filter

### API Changes Required:
- ✏️ Add search parameter to `getDocuments` API call
- ✏️ Add bulk delete endpoint support
- ✏️ Add bulk reprocess endpoint support

---

## Priority Summary

**🔥 Must Do (Quick Wins):**
1. ✅ Add tab-based status filter (R4) - 2 hours
2. ✅ Optimize table column widths and row actions (R5, R7) - 2 hours
3. ✅ Improve upload zone empty state (R3) - 1-2 hours
4. ✅ Add search functionality (R6) - 1 hour

**📌 Should Do (Next Sprint):**
5. Add right panel for document preview (R1) - 4-6 hours
6. Implement bulk selection (R8) - 3-4 hours
7. Improve batch upload progress (R9) - 2-3 hours
8. Add keyboard shortcuts - 2 hours

**💡 Nice to Have (Later):**
9. File type icons
10. Enhanced empty state illustrations
11. Pipeline status button in header
12. Active filter pills
