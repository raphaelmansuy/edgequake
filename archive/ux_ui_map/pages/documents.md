# Page: Documents

## Overview

- **Route**: `/documents`
- **Title**: "Documents"
- **Layout**: Main content with collapsible preview panel on right
- **Source File**: [src/app/(dashboard)/documents/page.tsx](../../edgequake_webui/src/app/(dashboard)/documents/page.tsx)

## Layout Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│ ┌───────────────┬─────────────────────────────────────────────────┐ │
│ │               │ Header (64px)                                   │ │
│ │               ├─────────────────────────────────────────────────┤ │
│ │   Sidebar     │ Breadcrumb: EdgeQuake > Documents               │ │
│ │   (64px)      ├─────────────────────────────────────┬───────────┤ │
│ │               │                                     │           │ │
│ │               │ Page Header                         │ Preview   │ │
│ │               │ "Documents" + action buttons        │ Panel     │ │
│ │               │                                     │ (collaps) │ │
│ │               │ ┌─────────────────────────────────┐ │           │ │
│ │               │ │ Search & Filters Bar            │ │ Document  │ │
│ │               │ └─────────────────────────────────┘ │ Details   │ │
│ │               │                                     │           │ │
│ │               │ ┌─────────────────────────────────┐ │ Content   │ │
│ │               │ │ Dropzone (Drag & Drop)          │ │ Preview   │ │
│ │               │ └─────────────────────────────────┘ │           │ │
│ │               │                                     │ Actions   │ │
│ │               │ ┌─────────────────────────────────┐ │           │ │
│ │               │ │ Documents List/Table            │ │           │ │
│ │               │ │ (or Empty State)                │ │           │ │
│ │               │ │                                 │ │           │ │
│ │               │ └─────────────────────────────────┘ │           │ │
│ │               │                                     │           │ │
│ │               │ ┌─────────────────────────────────┐ │           │ │
│ │               │ │ Pagination Controls             │ │           │ │
│ │               │ └─────────────────────────────────┘ │           │ │
│ └───────────────┴─────────────────────────────────────┴───────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

## Screenshots

| Viewport | Screenshot |
|----------|------------|
| Desktop (1440px) | [documents-desktop.png](../screenshots/documents/documents-desktop.png) |
| Tablet (768px) | [documents-tablet.png](../screenshots/documents/documents-tablet.png) |
| Mobile (375px) | [documents-mobile.png](../screenshots/documents/documents-mobile.png) |

---

## Region: Page Header

- **Position**: Top of main content
- **Layout**: Flex row, space-between
- **Source File**: [src/components/documents/document-manager.tsx](../../edgequake_webui/src/components/documents/document-manager.tsx)

### Container: Title Block

- **Content**:
  - H1: "Documents" (30px, bold)
  - Subtitle: "Téléchargez et gérez les documents pour l'extraction du graphe de connaissances"

### Container: Actions Bar

- **Layout**: Flex row with gap

#### Component: Scan Directory Button

- **Type**: Button, outline variant
- **Icon**: FolderSearch icon (left)
- **Text**: "Scan Directory"
- **Source File**: [src/components/documents/scan-documents-button.tsx](../../edgequake_webui/src/components/documents/scan-documents-button.tsx)

#### Component: Refresh Button

- **Type**: Button, outline variant
- **Icon**: RefreshCw icon (left)
- **Text**: "Rafraîchir"
- **Function**: Refetches document list

---

## Region: Filters Bar

- **Position**: Below header
- **Layout**: Flex row with gap
- **Source File**: [src/components/documents/document-filters.tsx](../../edgequake_webui/src/components/documents/document-filters.tsx)

### Component: Search Input

- **Type**: Input with search icon
- **Placeholder**: "Search documents..."
- **Icon**: Search icon (left, 16px)
- **Height**: 36px
- **Border Radius**: 8px

### Component: Status Filter

- **Type**: Select/Combobox dropdown
- **Default**: "Tous les statuts (0)"
- **Options**: All, Pending, Processing, Completed, Failed

### Component: Sort Controls

- **Type**: Button group
- **Label**: "Trier par :"
- **Options**: "Créé" (created_at), "Mis à jour" (updated_at)
- **States**: Selected button has pressed/active state

---

## Region: Upload Dropzone

- **Position**: Below filters
- **Type**: Drag-and-drop file upload area
- **Border**: 1px dashed border, rounded-xl
- **Padding**: 24px
- **Background**: Transparent (hover: muted/10)

### Container: Dropzone Content

- **Layout**: Flex row, space-between
- **Content**:
  - Upload icon (24px, muted-foreground)
  - Text: "Drag & drop files or click to browse"
  - Subtext: "TXT, MD, JSON (max 10MB per file)"
  - "Browse Files" button

#### Component: Browse Files Button

- **Type**: Button, outline variant
- **Text**: "Browse Files"
- **Function**: Opens file picker

---

## Region: Documents List

- **Position**: Main content area
- **Type**: Card with table or empty state

### Container: List Header

- **Layout**: Flex row with icon + count
- **Content**: FileText icon + "Documents (n)"

### Container: Empty State

- **Type**: Centered content block
- **Visibility**: Shown when document count is 0
- **Content**:
  - Large file icon (48px, muted)
  - Text: "No documents yet"
  - Subtext: "Upload documents to build your knowledge graph"

### Container: Documents Table

- **Type**: Data table
- **Visibility**: Shown when documents exist
- **Source File**: Uses Table component from [src/components/ui/table.tsx](../../edgequake_webui/src/components/ui/table.tsx)

#### Table Columns

| Column | Width | Content |
|--------|-------|---------|
| Checkbox | 40px | Selection checkbox |
| Name | flex | Document filename with icon |
| Status | 100px | Status badge (Pending/Processing/Completed/Failed) |
| Size | 80px | File size in KB/MB |
| Created | 120px | Relative time (e.g., "2 hours ago") |
| Actions | 40px | More options menu |

#### Component: Status Badge

- **Type**: Badge with icon
- **Variants**:
  - pending: Yellow, Clock icon
  - processing: Blue, Loader2 icon (animated spin)
  - completed: Green, CheckCircle icon
  - indexed: Green, CheckCircle icon
  - failed: Red, XCircle icon
- **Border Radius**: rounded-full

#### Component: Row Actions Menu

- **Type**: Dropdown menu
- **Trigger**: MoreVertical icon button
- **Options**:
  - View/Preview
  - Reprocess
  - Delete

---

## Region: Preview Panel

- **Position**: Right side, collapsible
- **Dimensions**: 400px width when expanded
- **Border**: 1px solid border on left
- **Background**: `var(--card)`
- **Source File**: [src/components/documents/document-preview-panel.tsx](../../edgequake_webui/src/components/documents/document-preview-panel.tsx)

### Container: Panel Toggle

- **Position**: Right edge of main content
- **Type**: Vertical button tab
- **Content**: Eye icon + "Preview" text (rotated 90°)

### Container: Panel Content

- **Visibility**: When document selected
- **Sections**:
  - Document metadata (name, status, dates)
  - Content preview (scrollable)
  - Entity/relationship counts
  - Action buttons (Reprocess, Delete)

---

## Component: Batch Progress Card

- **Type**: Progress indicator card
- **Visibility**: Shown during batch upload
- **Source File**: [src/components/documents/batch-progress-card.tsx](../../edgequake_webui/src/components/documents/batch-progress-card.tsx)
- **Content**:
  - Progress bar
  - File list with individual status
  - Phase indicator (Reading, Uploading, Extracting)

---

## Component: Pipeline Status Dialog

- **Type**: Modal dialog
- **Source File**: [src/components/documents/pipeline-status-dialog.tsx](../../edgequake_webui/src/components/documents/pipeline-status-dialog.tsx)
- **Content**: Real-time pipeline processing status

---

## Responsive Behavior

| Breakpoint | Layout | Preview Panel |
|------------|--------|---------------|
| Mobile (<768px) | Stacked, single column | Hidden or bottom sheet |
| Tablet (768-1024px) | Full width, collapsed panel | Collapsed by default |
| Desktop (>1024px) | Main + preview split | Visible when selected |

---

## Component Cross-References

- [Button](../components/buttons.md) — Action buttons, browse files
- [Input](../components/inputs.md) — Search input
- [Table](../components/tables.md) — Documents list
- [Card](../components/cards.md) — List container, preview panel
- [Badge](../components/buttons.md) — Status badges
- [Dialog](../components/dialogs.md) — Pipeline status, confirm delete
- [Dropdown Menu](../components/dialogs.md) — Row actions, status filter
