# OODA-82: Observe

**Date**: 2026-02-01
**Mission Re-read**: ✅ Read ./specs/002-unify-ingestion-pipeline.md

## Focus: PDF + Markdown Split View Component

### Current Document Detail Dialog State

**File**: `edgequake_webui/src/components/documents/document-detail-dialog.tsx`

**Tabs available**:

1. Overview - Metadata display
2. Content - Plain text content (pre-formatted)
3. Entities - Link to graph view

**Missing**: "Source" tab for PDF documents showing side-by-side view.

---

## Existing Components Analysis

### 1. PDFViewer Component

**File**: `edgequake_webui/src/components/documents/pdf-viewer.tsx`
**Lines**: 277
**Status**: ✅ Working

Features:

- react-pdf based rendering
- Page navigation (1 / N)
- Zoom controls (50% - 300%)
- Full width toggle
- Loading skeleton
- Error handling with retry

### 2. Document Type Interface

**File**: `edgequake_webui/src/types/index.ts:80-155`

Relevant fields:

- `pdf_id?: string` - Links to PDF document for viewing
- `source_type?: "pdf" | "markdown" | ...` - Determines if PDF view applicable
- `content?: string` - Extracted markdown text

---

## Required Implementation

### New Component: PDFMarkdownSplitView

```
┌─────────────────────────────────────────────────────────────┐
│  View: [PDF Only] [Markdown Only] [Side-by-Side]            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────┐ ║ ┌─────────────────────────────┐  │
│  │                     │ ║ │                             │  │
│  │    PDF Viewer       │ ║ │    Markdown Viewer          │  │
│  │   (react-pdf)       │ ║ │   (syntax highlighted)      │  │
│  │                     │ ║ │                             │  │
│  │   ◄ 1/10 ►         │ ║ │   # Document Title          │  │
│  │   [🔍+] [🔍-]      │ ║ │                             │  │
│  │                     │ ║ │   Lorem ipsum dolor sit...  │  │
│  │                     │ ║ │                             │  │
│  └─────────────────────┘ ║ └─────────────────────────────┘  │
│                          ║                                   │
│          ◄──── Draggable Divider ────►                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Integration Points

1. **DocumentDetailDialog** - Add "Source" tab when `document.pdf_id` exists
2. **API Endpoint** - `/api/v1/documents/pdf/{pdf_id}/download` already exists
3. **Content Display** - Use existing `document.content` for markdown

---

## Library Options for Split View

### Option A: @radix-ui/react-resizable (already using Radix)

- Pros: Consistent with existing Radix primitives
- Cons: Need to install, more setup

### Option B: react-resizable-panels (popular)

- Pros: Simple API, good defaults
- Cons: New dependency

### Option C: Custom CSS Grid with resize handle

- Pros: No new dependencies
- Cons: More code to maintain

**Decision**: Use CSS Grid with toggle buttons for view modes (simpler, no new deps)

---

## Summary

| Item                          | Status           |
| ----------------------------- | ---------------- |
| PDFViewer exists              | ✅               |
| pdf_id field in Document      | ✅               |
| API endpoint for PDF download | ✅               |
| Split view component          | ❌ Missing       |
| Source tab in dialog          | ❌ Missing       |
| Markdown viewer component     | ⚠️ Need to check |

---

## Next Action

Proceed to **Orient** phase to design the component architecture.
