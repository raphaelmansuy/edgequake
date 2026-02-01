# OODA-82: Act

**Date**: 2026-02-01
**Mission Re-read**: ✅

## Changes Implemented

### 1. New Component: PDFMarkdownSplitView

**File**: `edgequake_webui/src/components/documents/pdf-markdown-split-view.tsx` (NEW)

Features:

- Three view modes: PDF Only, Split, Markdown Only
- Toggle buttons in toolbar
- Responsive layout (stacked on mobile, side-by-side on desktop)
- Uses existing PDFViewer and MarkdownViewer components
- Independent scrolling for each panel
- Clear visual separation with borders

```tsx
interface PDFMarkdownSplitViewProps {
  pdfUrl: string;
  markdown: string | null;
  className?: string;
  height?: number;
  initialMode?: ViewMode; // 'pdf' | 'markdown' | 'split'
}
```

### 2. Updated DocumentDetailDialog

**File**: `edgequake_webui/src/components/documents/document-detail-dialog.tsx`

Changes:

- Added import for PDFMarkdownSplitView
- Added `hasPdfSource` check for `document.pdf_id`
- Constructed `pdfUrl` from pdf_id
- Made dialog wider for PDF documents (4xl vs 2xl)
- Added dynamic TabsList columns (4 for PDF, 3 for non-PDF)
- Added "Source" tab with PDFMarkdownSplitView component

```tsx
// OODA-82: Check if this is a PDF-origin document
const hasPdfSource = Boolean(document.pdf_id);
const pdfUrl = hasPdfSource ? `/api/v1/documents/pdf/${document.pdf_id}/download` : '';

// Tab grid adapts: 4 columns for PDF docs, 3 for others
<TabsList className={`grid w-full ${hasPdfSource ? 'grid-cols-4' : 'grid-cols-3'}`}>
```

---

## Verification

```bash
$ pnpm exec tsc --noEmit
# No errors

$ pnpm exec eslint src/components/documents/pdf-markdown-split-view.tsx src/components/documents/document-detail-dialog.tsx
# No errors
```

✅ TypeScript compilation successful
✅ ESLint passed

---

## UI Preview

```
┌─────────────────────────────────────────────────────────────┐
│  Document Detail: AgenticPlatformReference.pdf              │
│  [✓ Indexed] · 2 hours ago                                  │
├─────────────────────────────────────────────────────────────┤
│  [Overview] [Source] [Content] [Entities]                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  View: [PDF] [Split ✓] [Markdown]                          │
│  ┌────────────────────┬─────────────────────────────────┐  │
│  │                    │                                 │  │
│  │   PDF Viewer       │   Markdown Viewer               │  │
│  │   ◄ 1/40 ►        │   # Agentic Platform            │  │
│  │   [🔍+] [🔍-]     │   Reference Architecture        │  │
│  │                    │                                 │  │
│  └────────────────────┴─────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Files Changed

| Action | File                          | Lines     |
| ------ | ----------------------------- | --------- |
| CREATE | `pdf-markdown-split-view.tsx` | 162       |
| MODIFY | `document-detail-dialog.tsx`  | +20 lines |

---

## Next Steps

- OODA-83: SRP/DRY audit of ingestion pipeline
- OODA-84+: E2E testing with Playwright MCP

---

## Commit

Ready for commit:

```
OODA-82: Add PDF+Markdown split view in document detail

WHY: For PDF-origin documents, users need to see both the original
PDF and extracted markdown side-by-side to verify extraction quality.

- Create PDFMarkdownSplitView component with 3 view modes
- Add "Source" tab to DocumentDetailDialog for PDF documents
- Wider dialog for PDF documents (4xl vs 2xl)
- Responsive layout (stacked mobile, side-by-side desktop)
```
