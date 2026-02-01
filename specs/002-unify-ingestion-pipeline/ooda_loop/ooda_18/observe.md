# OODA-18: Observe - Document Viewer Architecture Audit

## Date: 2026-02-01

## Mission Re-Read ✓

Re-read `specs/002-unify-ingestion-pipeline.md` - Confirmed objectives:

1. PDF and Markdown viewer display
2. Side-by-side view capability
3. Scrolling/border/margin UX
4. Multi-tenancy compliance
5. Swagger/OpenAPI documentation
6. Testing evidence

---

## Current State Analysis

### Frontend Components (Already Implemented)

| Component            | File                                                                                              | Status      | Features                                          |
| -------------------- | ------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------- |
| PDFViewer            | [pdf-viewer.tsx](edgequake_webui/src/components/documents/pdf-viewer.tsx)                         | ✅ Complete | react-pdf, pagination, zoom, fullscreen           |
| MarkdownViewer       | [markdown-viewer.tsx](edgequake_webui/src/components/documents/markdown-viewer.tsx)               | ✅ Complete | StreamingMarkdownRenderer, copy, syntax highlight |
| SideBySideViewer     | [side-by-side-viewer.tsx](edgequake_webui/src/components/documents/side-by-side-viewer.tsx)       | ✅ Complete | Resizable panels, view modes                      |
| DocumentViewerDialog | [document-viewer-dialog.tsx](edgequake_webui/src/components/documents/document-viewer-dialog.tsx) | ✅ Complete | Full-screen modal, download                       |

### PDF Library Choice

**Selected: `react-pdf` (wojtekmaj)**

- Version: 10.x
- Weekly Downloads: 2.8M+
- Based on: Mozilla pdf.js
- Last Updated: Active maintenance
- Features:
  - Text layer support
  - Annotation layer support
  - Thumbnail generation
  - Outline/TOC support
  - Responsive layout

**Alternative Considered: `@react-pdf-viewer/core`**

- Last publish: 3 years ago ⚠️
- Would require license purchase
- Not suitable for active development

**Decision: Keep react-pdf** (already implemented, well-maintained)

### Backend API Endpoints

| Endpoint                       | Method | Handler           | Multi-Tenant | OpenAPI |
| ------------------------------ | ------ | ----------------- | ------------ | ------- |
| `/documents/pdf/{id}/download` | GET    | `download_pdf`    | ✅           | ✅      |
| `/documents/pdf/{id}/content`  | GET    | `get_pdf_content` | ✅           | ✅      |

### Multi-Tenancy Verification

Both PDF endpoints enforce workspace isolation:

```rust
// Verify workspace access
let workspace_id = context.workspace_id_uuid()?;
if pdf.workspace_id != workspace_id {
    return Err(ApiError::Forbidden);
}
```

**Status: ✅ COMPLIANT**

### UX Observations

1. **PDF Viewer UX**:
   - ✅ Pagination controls
   - ✅ Zoom in/out (0.5x to 3.0x)
   - ✅ Full-width toggle
   - ✅ Loading skeleton
   - ✅ Error state with retry

2. **Side-by-Side Viewer UX**:
   - ✅ Resizable divider (25%-75% range)
   - ✅ View mode toggle (PDF only, Markdown only, side-by-side)
   - ✅ Tooltips for controls
   - ⚠️ Scrolling not synchronized between panels

3. **Markdown Viewer UX**:
   - ✅ Copy to clipboard
   - ✅ Syntax highlighting
   - ✅ Dark/light mode
   - ✅ Prose typography

### Gaps Identified

1. **Scroll Synchronization**: PDF and Markdown panels scroll independently
2. **Page-to-Section Mapping**: No linking between PDF pages and Markdown sections
3. **Thumbnail Navigation**: No page thumbnail sidebar
4. **Search in PDF**: No text search feature
5. **Print Mode**: No print-optimized view

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        DOCUMENT VIEWER STACK                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                    DocumentViewerDialog                            │ │
│  │  ┌──────────────────────────────────────────────────────────────┐  │ │
│  │  │                  SideBySideViewer                            │  │ │
│  │  │  ┌─────────────────────┐ │ ┌─────────────────────────────┐  │  │ │
│  │  │  │    PDFViewer        │ │ │     MarkdownViewer          │  │  │ │
│  │  │  │    (react-pdf)      │ │ │  (StreamingMarkdown)        │  │  │ │
│  │  │  │                     │ │ │                             │  │  │ │
│  │  │  │  • Document         │ │ │  • prose styling            │  │  │ │
│  │  │  │  • Page             │ │ │  • code blocks              │  │  │ │
│  │  │  │  • pdfjs-dist       │ │ │  • tables                   │  │  │ │
│  │  │  └─────────────────────┘ │ └─────────────────────────────┘  │  │ │
│  │  │         ▲ Scroll         │         ▲ Scroll (independent)   │  │ │
│  │  └──────────────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  API Layer:                                                              │
│  ┌────────────────────┐    ┌─────────────────────────────────┐          │
│  │ getPdfDownloadUrl │─►  │ /api/v1/documents/pdf/:id/download │        │
│  └────────────────────┘    └─────────────────────────────────┘          │
│  ┌────────────────────┐    ┌─────────────────────────────────┐          │
│  │ getPdfContent     │─►  │ /api/v1/documents/pdf/:id/content  │        │
│  └────────────────────┘    └─────────────────────────────────┘          │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Dependencies Verified

```json
{
  "react-pdf": "^10.0.1",
  "pdfjs-dist": "^4.10.38"
}
```

---

## Summary

| Area            | Status       | Notes                        |
| --------------- | ------------ | ---------------------------- |
| PDF Viewer      | ✅ Complete  | react-pdf well integrated    |
| Markdown Viewer | ✅ Complete  | StreamingMarkdownRenderer    |
| Side-by-Side    | ✅ Complete  | Resizable panels             |
| Multi-Tenancy   | ✅ Compliant | Workspace isolation enforced |
| OpenAPI Docs    | ✅ Present   | utoipa annotations           |
| Scroll Sync     | ⚠️ Missing   | Potential enhancement        |
| Page Mapping    | ⚠️ Missing   | Potential enhancement        |
