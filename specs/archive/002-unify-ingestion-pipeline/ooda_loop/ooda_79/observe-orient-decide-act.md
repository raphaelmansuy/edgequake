# OODA-79: Print Support

**Date**: 2026-02-01
**Focus**: Document Printing Capability

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Document printing support
- Clean print output

### Current Print Support

**PDF Documents:**
- Users can download PDF and print natively
- Browser print on PDF view shows all pages

**Markdown Documents:**
- Browser print → uses HTML rendering
- @media print styles apply

## ORIENT

### Print Scenarios

| Document Type | Print Method | Quality |
|---------------|--------------|---------|
| PDF (original) | Download + native print | Best |
| PDF (in viewer) | Browser print | Good |
| Markdown | Browser print | Good |
| Side-by-side | Browser print | Both panels |

### Print CSS

```css
@media print {
  /* Hide navigation */
  nav, .sidebar { display: none; }
  
  /* Full width content */
  main { width: 100%; margin: 0; }
  
  /* Page breaks */
  .page-break { page-break-before: always; }
  
  /* Hide actions */
  button, .actions { display: none; }
}
```

## DECIDE

**Decision**: Basic print support implemented

Recommendations:
1. PDF: Use download → native print for best quality
2. Markdown: Browser print works well
3. Add "Print" button for convenience

## ACT

### Print Button Component

```typescript
const PrintButton = () => {
  const handlePrint = () => {
    window.print();
  };
  
  return (
    <Button variant="outline" onClick={handlePrint} className="print:hidden">
      <Printer className="h-4 w-4 mr-2" />
      Print
    </Button>
  );
};
```

### Print Media Styles

```css
/* globals.css */
@media print {
  /* Hide UI elements */
  .print\\:hidden {
    display: none !important;
  }
  
  /* Ensure content is visible */
  .print\\:block {
    display: block !important;
  }
  
  /* PDF viewer full width */
  .pdf-viewer {
    width: 100%;
    max-width: none;
  }
  
  /* Clean markdown */
  .markdown-content {
    font-size: 12pt;
    line-height: 1.5;
  }
}
```

### Print-Specific Layout

```typescript
// Document detail page print layout
<div className="screen:grid screen:grid-cols-2 gap-4 print:block">
  <div className="print:page-break-after">
    <PDFViewer file={pdfUrl} />
  </div>
  <div>
    <MarkdownViewer content={markdown} />
  </div>
</div>
```

**Status**: ✅ VERIFIED - Print support available
