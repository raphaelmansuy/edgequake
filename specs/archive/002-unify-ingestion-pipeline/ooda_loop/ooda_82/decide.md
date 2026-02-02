# OODA-82: Decide

**Date**: 2026-02-01
**Mission Re-read**: ✅

## Decision: Implement PDF + Markdown Split View

### Implementation Steps

1. **Create `pdf-markdown-split-view.tsx`**
   - View mode toggle (PDF / Markdown / Split)
   - Grid layout for split view
   - Responsive stacking on mobile
   - Uses existing PDFViewer and MarkdownViewer

2. **Modify `document-detail-dialog.tsx`**
   - Add "Source" tab (visible only when `document.pdf_id` exists)
   - Import and use PDFMarkdownSplitView component
   - Construct PDF URL from pdf_id

### File Locations

| Action | File                                                                   |
| ------ | ---------------------------------------------------------------------- |
| CREATE | `edgequake_webui/src/components/documents/pdf-markdown-split-view.tsx` |
| MODIFY | `edgequake_webui/src/components/documents/document-detail-dialog.tsx`  |

---

## Component Specification

### PDFMarkdownSplitView

```tsx
interface PDFMarkdownSplitViewProps {
  pdfUrl: string;
  markdown: string | null;
  className?: string;
  height?: number;
}

// View modes
type ViewMode = "pdf" | "markdown" | "split";

// Default: 'split' for maximum utility
```

### Styling

- Grid layout: `grid grid-cols-2` for split
- Divider: `border-l` between panels
- Responsive: `flex flex-col lg:grid lg:grid-cols-2`
- Panel height: Match parent or fixed height prop

---

## Acceptance Criteria

- [ ] Toggle buttons work for all three modes
- [ ] PDF viewer loads and displays correctly
- [ ] Markdown viewer renders content correctly
- [ ] Split view shows both side-by-side
- [ ] Mobile view stacks vertically
- [ ] "Source" tab only appears for PDF documents

---

## Next Action

Proceed to **Act** phase to implement the components.
