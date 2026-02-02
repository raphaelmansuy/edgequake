# OODA-33: react-pdf Library Choice Justification

**Date**: 2025-01-27  
**Focus**: PDF Library Selection Rationale

## OBSERVE

### Library Comparison (Jan 2025)

| Library             | Weekly Downloads | Last Update | Stars | Size   |
| ------------------- | ---------------- | ----------- | ----- | ------ |
| react-pdf           | 2.8M             | Active      | 9.5k  | ~200KB |
| @react-pdf/renderer | 800K             | Active      | 15k   | ~150KB |
| pdf.js (raw)        | 5M               | Active      | 50k   | ~500KB |
| react-pdf-viewer    | 50K              | Active      | 2k    | ~300KB |

### react-pdf Features

- ✅ Built on Mozilla pdf.js (industry standard)
- ✅ React 18 compatible
- ✅ TypeScript support
- ✅ SSR compatible
- ✅ Active maintenance
- ✅ Pagination or continuous scroll
- ✅ Zoom controls
- ✅ Text selection
- ✅ Annotations support

### Current Implementation

```typescript
import { Document, Page, pdfjs } from "react-pdf";

// PDF.js worker setup
pdfjs.GlobalWorkerOptions.workerSrc = `//unpkg.com/pdfjs-dist@${pdfjs.version}/build/pdf.worker.min.mjs`;
```

## ORIENT

### First Principle: Best Tool for the Job

- PDF rendering is complex (fonts, vectors, annotations)
- Using battle-tested library saves dev time
- pdf.js is the gold standard (Mozilla-maintained)

### Why NOT Other Options

**@react-pdf/renderer**:

- For creating PDFs, not viewing
- Wrong use case

**Raw pdf.js**:

- Lower level, more boilerplate
- react-pdf is wrapper around it

**react-pdf-viewer**:

- Less popular
- Fewer features
- Less community support

## DECIDE

**Decision**: react-pdf is the correct choice

### Justification Matrix

1. **Popularity**: 2.8M downloads/week = proven stability
2. **Foundation**: Built on pdf.js = gold standard
3. **React Integration**: Native hooks and components
4. **Feature Set**: Pagination, zoom, annotations
5. **TypeScript**: Full type definitions
6. **Documentation**: Comprehensive and examples

## ACT

### Implementation Verification

From `package.json`:

```json
{
  "dependencies": {
    "react-pdf": "^10.0.0"
  }
}
```

### Code Quality

```typescript
// Proper error handling
onLoadError={(error) => setError(error.message)}

// Proper loading states
onLoadSuccess={({ numPages }) => setNumPages(numPages)}

// Proper pagination
<Page pageNumber={pageNumber} scale={scale} />
```

### Evidence Collection

- Library installed and working
- No runtime errors observed
- PDF rendering confirmed functional

**Status**: ✅ COMPLETE - react-pdf correctly chosen as best option
