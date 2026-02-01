# OODA-43: Document Detail Page PDF Side-by-Side View

**Date**: 2026-02-01
**Focus**: PDF and Markdown Side-by-Side Display in Document Detail Page

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- PDF Viewing: Display PDF version using best-in-class JS viewer
- Side-by-Side View: Allow comparing PDF and Markdown simultaneously
- Scrolling/Layout: Handle borders, margins, and scrolling optimally
- Responsive Design: Minimalist style, fast, no bloat

### Current State Analysis
Document detail page (`/documents/[id]`) only showed:
- ContentRenderer (markdown/text)
- MetadataSidebar

No PDF viewing capability existed in the detail page.

### Existing Components Available
- `SideBySideViewer`: Resizable split panel layout
- `PDFViewer`: react-pdf based PDF rendering
- Both components well-tested and working

## ORIENT

### First Principle: Complete Document Experience
- Users need to see original PDF alongside extracted markdown
- Comparison view enables quality verification
- Mobile users need tab-based fallback

### Implementation Approach
1. Detect PDF documents via `pdf_id` or `source_type`
2. Desktop: Use SideBySideViewer with PDF left, Markdown right
3. Mobile: Add PDF tab to existing tabbed layout
4. Add Download PDF button in header for convenience

## DECIDE

**Decision**: Integrate SideBySideViewer for PDF documents

### Implementation Plan
1. Import PDFViewer, SideBySideViewer, and getPdfDownloadUrl
2. Add isPdfDocument detection flag
3. Conditionally render SideBySideViewer for PDFs
4. Add PDF tab for mobile layout
5. Add Download PDF button in header

## ACT

### Changes Made

**File**: `edgequake_webui/src/app/(dashboard)/documents/[id]/page.tsx`

1. **Added imports** (lines 5-11):
```typescript
import { PDFViewer } from '@/components/documents/pdf-viewer';
import { SideBySideViewer } from '@/components/documents/side-by-side-viewer';
import { getDocument, getPdfDownloadUrl } from '@/lib/api/edgequake';
import { Download } from 'lucide-react';
```

2. **Added isPdfDocument detection** (line 128):
```typescript
// OODA-43: Detect if document is a PDF for side-by-side viewer
const isPdfDocument = Boolean(document.pdf_id) || document.source_type === 'pdf';
```

3. **Added Download PDF button in header** (lines 167-174):
```typescript
{/* OODA-43: Download PDF button for PDF documents */}
{isPdfDocument && document.pdf_id && (
  <Button variant="outline" size="sm" asChild>
    <a href={getPdfDownloadUrl(document.pdf_id)} target="_blank">
      <Download className="h-4 w-4 mr-2" />
      Download PDF
    </a>
  </Button>
)}
```

4. **Desktop: Conditional SideBySideViewer** (lines 200-221):
```typescript
{isPdfDocument ? (
  <SideBySideViewer
    leftTitle="PDF Document"
    rightTitle="Extracted Markdown"
    leftPanel={<PDFViewer file={getPdfDownloadUrl(document.pdf_id!)} />}
    rightPanel={<ContentRenderer document={document} />}
  />
) : (
  <ContentRenderer document={document} />
)}
```

5. **Mobile: PDF tab added** (lines 244-254):
```typescript
{isPdfDocument && <TabsTrigger value="pdf">PDF</TabsTrigger>}
{isPdfDocument && (
  <TabsContent value="pdf">
    <PDFViewer file={getPdfDownloadUrl(document.pdf_id!)} />
  </TabsContent>
)}
```

### Behavior After Change
```
Document Detail Page (/documents/[id])

PDF Documents:
┌─────────────────────────────────────────────┐
│ ← Back   Document.pdf    [Download] [Graph] │
├─────────────────────────────────────────────┤
│            │                                │
│    PDF     │     Extracted                  │
│   Viewer   │     Markdown                   │
│            │                                │
│   [Zoom]   │   # Heading                    │
│   [Page]   │   Content here...              │
│            │                                │
└─────────────────────────────────────────────┘

Non-PDF Documents (unchanged):
┌─────────────────────────────────────────────┐
│                    │                        │
│    Markdown        │   Metadata             │
│    Content         │   Sidebar              │
│                    │                        │
└─────────────────────────────────────────────┘
```

### Evidence
- TypeScript compilation: ✅ No errors
- PDF documents show side-by-side viewer
- Download button visible for PDFs
- Mobile layout has PDF tab

**Status**: ✅ COMPLETE - PDF side-by-side view implemented
