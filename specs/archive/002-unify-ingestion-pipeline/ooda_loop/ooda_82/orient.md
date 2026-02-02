# OODA-82: Orient

**Date**: 2026-02-01
**Mission Re-read**: ✅

## Analysis: PDF + Markdown Split View

### Available Components

| Component            | File                                              | Status             |
| -------------------- | ------------------------------------------------- | ------------------ |
| PDFViewer            | `components/documents/pdf-viewer.tsx`             | ✅ Ready           |
| MarkdownViewer       | `components/documents/markdown-viewer.tsx`        | ✅ Ready           |
| DocumentDetailDialog | `components/documents/document-detail-dialog.tsx` | Needs modification |

### Architecture Decision

**Option A: Dedicated Split View Component**

- Create `pdf-markdown-split-view.tsx` with toggle modes
- Embed in new "Source" tab
- Pros: Reusable, testable, clean separation
- Cons: More files

**Option B: Inline in Dialog**

- Add split view logic directly in DocumentDetailDialog
- Pros: Fewer files
- Cons: Bloats dialog, less reusable

**Decision**: Option A - Create dedicated component

---

## Component Design

### PDFMarkdownSplitView

```typescript
interface PDFMarkdownSplitViewProps {
  pdfUrl: string; // URL to PDF file
  markdown: string | null; // Extracted markdown content
  className?: string;
  height?: number;
}

type ViewMode = "pdf" | "markdown" | "split";
```

### View Mode Behavior

| Mode          | Left Panel       | Right Panel           | Divider |
| ------------- | ---------------- | --------------------- | ------- |
| PDF Only      | PDFViewer (100%) | Hidden                | Hidden  |
| Markdown Only | Hidden           | MarkdownViewer (100%) | Hidden  |
| Side-by-Side  | PDFViewer (50%)  | MarkdownViewer (50%)  | Visible |

### UX Considerations

1. **Toggle buttons** in toolbar for view mode selection
2. **Default mode**: Side-by-side (most useful for comparison)
3. **Responsive**: Stack vertically on mobile
4. **Visual separation**: Border/shadow between panels
5. **Independent scrolling**: Each panel scrolls independently
6. **Persistence**: Remember user's preferred view mode (localStorage)

---

## Integration Plan

1. Create `pdf-markdown-split-view.tsx` component
2. Add "Source" tab to DocumentDetailDialog when `pdf_id` exists
3. Construct PDF URL from `pdf_id`
4. Pass document content as markdown

---

## API URL Construction

From existing code analysis:

```typescript
const pdfUrl = `/api/v1/documents/pdf/${document.pdf_id}/download`;
```

---

## Next Action

Proceed to **Decide** phase to finalize implementation steps.
