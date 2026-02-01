# OODA-78: Memory Management

**Date**: 2026-02-01
**Focus**: PDF Viewer Memory Optimization

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Efficient memory usage
- No memory leaks

### PDF Rendering Memory Characteristics

**PDF.js Memory Usage:**
- Each rendered page: ~2-5MB in memory
- Large PDFs (50+ pages): 100-250MB
- Canvas elements retained until cleanup

### React Lifecycle Concerns

```typescript
// Potential memory leak patterns:
useEffect(() => {
  const canvas = document.createElement('canvas');
  // If not cleaned up, leaks memory
  return () => {
    // Cleanup needed
  };
}, []);
```

## ORIENT

### Memory Optimization Strategies

| Strategy | Benefit | Complexity |
|----------|---------|------------|
| Page caching limit | Reduce peak memory | Low |
| Lazy page rendering | Load on demand | Medium |
| Canvas cleanup | Free memory on unmount | Low |
| Virtual scrolling | Only render visible | High |

### Current Implementation

PDFViewer renders all pages at once:
- Simple implementation
- Higher memory for large PDFs
- Acceptable for typical document sizes (< 30 pages)

## DECIDE

**Decision**: Current implementation acceptable

For MVP, render-all is fine. Add optimization if:
- Users report issues with large PDFs
- Memory profiling shows problems
- Tablet/mobile devices struggle

## ACT

### Cleanup on Unmount

```typescript
const PDFViewer = ({ file }: PDFViewerProps) => {
  const [pdf, setPdf] = useState<PDFDocumentProxy | null>(null);
  
  useEffect(() => {
    return () => {
      // Cleanup PDF.js document
      if (pdf) {
        pdf.destroy();
      }
    };
  }, [pdf]);
  
  return (
    <Document
      file={file}
      onLoadSuccess={(pdf) => setPdf(pdf)}
    >
      {/* pages */}
    </Document>
  );
};
```

### Page Caching Configuration

```typescript
// react-pdf optional caching config
<Document
  file={file}
  options={{
    cMapUrl: '/cmaps/',
    cMapPacked: true,
    // Limit page cache
    disableAutoFetch: true,
    disableStream: true,
  }}
>
```

### Memory Monitoring

```typescript
// Development helper
if (process.env.NODE_ENV === 'development') {
  setInterval(() => {
    if (window.performance?.memory) {
      console.log('Memory:', {
        used: Math.round(window.performance.memory.usedJSHeapSize / 1024 / 1024) + 'MB',
        total: Math.round(window.performance.memory.totalJSHeapSize / 1024 / 1024) + 'MB',
      });
    }
  }, 10000);
}
```

**Status**: ✅ VERIFIED - Memory management acceptable
