# UX Improvements Plan

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** UX shortcomings and improvement strategies

---

## Table of Contents

1. [UX Heuristics Assessment](#ux-heuristics-assessment)
2. [Current State Analysis](#current-state-analysis)
3. [Improvement Recommendations](#improvement-recommendations)
4. [Interaction Design](#interaction-design)
5. [Accessibility Improvements](#accessibility-improvements)
6. [Cross-References](#cross-references)

---

## UX Heuristics Assessment

Based on Nielsen's 10 Usability Heuristics:

| Heuristic                    | Current Score | Target | Gap    |
| ---------------------------- | ------------- | ------ | ------ |
| Visibility of system status  | 3/5           | 5/5    | Medium |
| Match with real world        | 4/5           | 5/5    | Low    |
| User control and freedom     | 3/5           | 5/5    | Medium |
| Consistency and standards    | 4/5           | 5/5    | Low    |
| Error prevention             | 3/5           | 5/5    | Medium |
| Recognition over recall      | 3/5           | 5/5    | Medium |
| Flexibility and efficiency   | 2/5           | 5/5    | High   |
| Aesthetic and minimal design | 4/5           | 5/5    | Low    |
| Error recovery               | 3/5           | 4/5    | Medium |
| Help and documentation       | 2/5           | 4/5    | High   |

---

## Current State Analysis

### Graph Viewer UX

**Strengths:**

- ✅ Clean visual presentation
- ✅ Node selection with details panel
- ✅ Context menu on right-click
- ✅ Zoom controls
- ✅ Loading skeleton

**Weaknesses:**

- ❌ No visual feedback during node operations
- ❌ Limited discoverability of features
- ❌ No keyboard navigation
- ❌ Static node positions (no drag)
- ❌ No search (must scroll to find nodes)
- ❌ No legend for color meaning

**User Pain Points:**

1. "I can't find the entity I'm looking for"
2. "What do the colors mean?"
3. "I want to move this node to see connections better"
4. "I need to present this in full screen"

---

### Document Manager UX

**Strengths:**

- ✅ Drag-and-drop upload
- ✅ Status badges with icons
- ✅ Auto-refresh for status updates
- ✅ Confirmation dialogs for destructive actions

**Weaknesses:**

- ❌ No bulk selection
- ❌ Pagination not visible without scrolling
- ❌ No progress indication during upload
- ❌ Pipeline status not prominent
- ❌ No document preview

**User Pain Points:**

1. "I uploaded 10 files but can't see the progress"
2. "I want to delete multiple failed documents at once"
3. "Where can I see what's currently processing?"

---

### Query Interface UX

**Strengths:**

- ✅ Clean chat-like interface
- ✅ Mode selector visible
- ✅ Streaming response display
- ✅ Source citations expandable
- ✅ History sidebar

**Weaknesses:**

- ❌ No thinking time indication
- ❌ No keyboard shortcuts for modes
- ❌ No copy button on responses
- ❌ No prompt templates/history
- ❌ Settings require opening panel

**User Pain Points:**

1. "I want to quickly switch to local mode"
2. "How long did the AI think before answering?"
3. "I want to copy this response to share"

---

### Navigation UX

**Strengths:**

- ✅ Clear sidebar with icons
- ✅ Mobile-responsive
- ✅ Breadcrumb navigation
- ✅ Keyboard shortcuts (?)

**Weaknesses:**

- ❌ No quick-jump shortcuts
- ❌ No recent pages/history
- ❌ Theme toggle not prominently placed
- ❌ Connection status small

---

## Improvement Recommendations

### 1. Visibility of System Status

#### Problem

Users don't know what's happening during long operations.

#### Solutions

**a) Enhanced Loading States**

```tsx
// Add to graph loading
<div className="absolute inset-0 flex items-center justify-center bg-background/80">
  <div className="flex flex-col items-center gap-2">
    <Loader2 className="h-8 w-8 animate-spin" />
    <span className="text-sm text-muted-foreground">
      Loading {nodeCount} nodes...
    </span>
    <Progress value={loadProgress} className="w-48" />
  </div>
</div>
```

**b) Pipeline Status Badge**

- Add pulsing badge when pipeline is active
- Show count of processing/queued documents
- Click to open detailed status dialog

**c) Upload Progress**

```tsx
// Show individual file progress
{
  uploadingFiles.map((file) => (
    <div key={file.name} className="flex items-center gap-2">
      <FileIcon className="h-4 w-4" />
      <span className="text-sm">{file.name}</span>
      <Progress value={file.progress} className="flex-1" />
    </div>
  ));
}
```

---

### 2. Flexibility and Efficiency

#### Problem

Power users have no shortcuts or advanced features.

#### Solutions

**a) Keyboard Shortcuts Expansion**

| Shortcut | Action                |
| -------- | --------------------- |
| `Ctrl+K` | Open command palette  |
| `Ctrl+G` | Go to graph           |
| `Ctrl+D` | Go to documents       |
| `Ctrl+Q` | Go to query           |
| `Ctrl+/` | Toggle help           |
| `F11`    | Toggle fullscreen     |
| `Escape` | Close dialog/deselect |
| `/mode`  | Query mode prefix     |

**b) Command Palette**

```tsx
// Implement cmdk command palette
<CommandDialog open={open} onOpenChange={setOpen}>
  <CommandInput placeholder="Type a command or search..." />
  <CommandList>
    <CommandGroup heading="Navigation">
      <CommandItem onSelect={() => router.push("/graph")}>
        <Network className="mr-2 h-4 w-4" />
        Go to Graph
      </CommandItem>
      {/* More commands */}
    </CommandGroup>
  </CommandList>
</CommandDialog>
```

**c) Quick Actions Menu**

- Add floating action button (FAB) on mobile
- Context-aware actions based on current page

---

### 3. Recognition Over Recall

#### Problem

Users must remember how to use features.

#### Solutions

**a) Graph Legend**

```tsx
<div className="absolute bottom-4 left-4 bg-card p-2 rounded-lg shadow">
  <h4 className="text-xs font-medium mb-1">Entity Types</h4>
  {entityTypes.map((type) => (
    <div key={type.name} className="flex items-center gap-2 text-xs">
      <div
        className="w-3 h-3 rounded-full"
        style={{ backgroundColor: type.color }}
      />
      <span>{type.name}</span>
    </div>
  ))}
</div>
```

**b) Tooltips on All Actions**

- Add tooltips to every icon button
- Include keyboard shortcut in tooltip
- Use consistent tooltip placement

**c) Onboarding Hints**

```tsx
// First-time user hints
{
  !hasSeenHint("graph-drag") && (
    <div className="absolute top-4 right-4 bg-primary text-primary-foreground p-3 rounded-lg">
      <p className="text-sm">💡 Tip: Drag nodes to rearrange the graph!</p>
      <Button
        size="sm"
        variant="ghost"
        onClick={() => dismissHint("graph-drag")}
      >
        Got it
      </Button>
    </div>
  );
}
```

---

### 4. Help and Documentation

#### Problem

Users don't know all available features.

#### Solutions

**a) In-App Help Panel**

```tsx
// Add help button that opens a panel with:
- Feature walkthroughs
- Keyboard shortcuts list
- FAQ
- Link to documentation
```

**b) Contextual Help**

- Add `?` icon next to complex features
- Hover for explanation
- Click for detailed help

**c) Empty State Guidance**

```tsx
// When no results
<div className="text-center py-8">
  <Search className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
  <h3 className="font-medium">No entities found</h3>
  <p className="text-sm text-muted-foreground mt-1">
    Try a different search term or{" "}
    <button className="underline">view all entities</button>
  </p>
</div>
```

---

## Interaction Design

### Graph Interactions

| Action       | Trigger              | Feedback                       |
| ------------ | -------------------- | ------------------------------ |
| Select node  | Click                | Highlight + show details panel |
| Drag node    | Click + drag         | Move node, update connections  |
| Pan graph    | Click + drag canvas  | Move viewport                  |
| Zoom         | Scroll / buttons     | Scale graph with cursor focus  |
| Search       | Ctrl+F or click icon | Open search popover            |
| Context menu | Right-click node     | Show action menu               |
| Multi-select | Ctrl+Click           | Add to selection (future)      |

### Document Interactions

| Action     | Trigger             | Feedback                 |
| ---------- | ------------------- | ------------------------ |
| Upload     | Drop files or click | Progress bar per file    |
| Delete     | Click trash icon    | Confirmation dialog      |
| Delete all | Click button        | Confirmation with count  |
| Filter     | Dropdown selection  | Update table immediately |
| Sort       | Click column header | Arrow indicator          |
| Refresh    | Click button        | Spinner + toast          |

### Query Interactions

| Action          | Trigger             | Feedback                    |
| --------------- | ------------------- | --------------------------- |
| Submit          | Enter or click      | Disable input, show spinner |
| Stream response | Automatic           | Typing indicator + scroll   |
| Toggle sources  | Click chevron       | Expand/collapse animation   |
| Copy response   | Click copy icon     | Toast confirmation          |
| Switch mode     | Dropdown or /prefix | Badge update                |

---

## Accessibility Improvements

### Current Issues

1. **Color Contrast:** Some muted text below 4.5:1 ratio
2. **Keyboard Navigation:** Graph not keyboard accessible
3. **Screen Readers:** Missing ARIA labels on icons
4. **Focus Indicators:** Inconsistent focus rings
5. **Motion:** No reduced motion support

### Fixes

**a) ARIA Labels**

```tsx
// Add to all icon buttons
<Button aria-label={t("graph.zoomIn")}>
  <ZoomIn className="h-4 w-4" aria-hidden="true" />
</Button>
```

**b) Skip Links**

```tsx
// Add at top of layout
<a
  href="#main-content"
  className="sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-50 focus:bg-primary focus:text-primary-foreground focus:p-2 focus:rounded"
>
  Skip to main content
</a>
```

**c) Reduced Motion**

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

**d) Focus Management**

```tsx
// Trap focus in dialogs
// Return focus on close
// Visible focus rings on all interactive elements
```

---

## Responsive Design

### Breakpoints

| Breakpoint | Width    | Layout Changes                   |
| ---------- | -------- | -------------------------------- |
| Mobile     | < 640px  | Collapsed sidebar, stacked cards |
| Tablet     | 640-1024 | Sidebar overlay, compact toolbar |
| Desktop    | > 1024px | Full sidebar, all panels visible |

### Mobile-Specific Improvements

1. **Bottom Navigation Bar**

   - Replace sidebar with bottom tabs on mobile
   - Prominent action buttons

2. **Touch-Friendly Targets**

   - Minimum 44x44px touch targets
   - Adequate spacing between buttons

3. **Swipe Gestures**
   - Swipe to delete documents
   - Swipe to navigate between tabs

---

## Error Handling UX

### Error States

**API Errors:**

```tsx
<Alert variant="destructive">
  <AlertCircle className="h-4 w-4" />
  <AlertTitle>Connection Failed</AlertTitle>
  <AlertDescription>
    Unable to connect to EdgeQuake API.
    <Button variant="link" onClick={retry}>
      Try again
    </Button>
  </AlertDescription>
</Alert>
```

**Empty States:**

```tsx
<div className="text-center py-12">
  <FileText className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
  <h3 className="font-medium">No documents yet</h3>
  <p className="text-sm text-muted-foreground mt-1">
    Upload your first document to get started
  </p>
  <Button className="mt-4">
    <Upload className="mr-2 h-4 w-4" />
    Upload Document
  </Button>
</div>
```

**Validation Errors:**

```tsx
// Inline validation with helpful messages
<div>
  <Input
    className={errors.query ? "border-destructive" : ""}
    aria-invalid={!!errors.query}
    aria-describedby="query-error"
  />
  {errors.query && (
    <p id="query-error" className="text-sm text-destructive mt-1">
      {errors.query}
    </p>
  )}
</div>
```

---

## Cross-References

- **Gap Analysis:** [001-gap-analysis.md](./001-gap-analysis.md)
- **Proposed Solutions:** [002-proposed-solutions.md](./002-proposed-solutions.md)
- **Performance:** [006-performance-strategy.md](./006-performance-strategy.md)
- **Success Criteria:** [008-success-criteria.md](./008-success-criteria.md)
