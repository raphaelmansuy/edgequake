# OODA-77: Bundle Size Optimization

**Date**: 2026-02-01
**Focus**: Frontend Performance

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Fast page loads
- Optimized bundle size

### Current Bundle Analysis

**Major Dependencies:**
| Package | Size | Purpose |
|---------|------|---------|
| react-pdf | ~300KB | PDF rendering |
| @radix-ui/* | ~50KB | UI primitives |
| @tanstack/react-query | ~40KB | Data fetching |
| next | ~200KB | Framework |

**PDF.js Worker:**
- pdfjs-dist/build/pdf.worker.min.js: ~800KB
- Loaded separately via web worker

## ORIENT

### Optimization Strategies

| Strategy | Impact | Implementation |
|----------|--------|----------------|
| Code splitting | High | Dynamic imports |
| Tree shaking | Medium | ES modules |
| Worker externalization | High | CDN worker |
| Image optimization | Medium | next/image |

### Current Code Splitting

```typescript
// PDF viewer lazy loaded
const PDFViewer = dynamic(
  () => import('@/components/documents/pdf-viewer'),
  { 
    loading: () => <PDFLoadingSkeleton />,
    ssr: false 
  }
);
```

## DECIDE

**Decision**: Bundle optimization correctly applied

Key optimizations:
1. PDF viewer dynamically imported
2. PDF worker from CDN
3. Only needed React components loaded

## ACT

### PDF Worker CDN Configuration

```typescript
// In PDFViewer component
import { pdfjs } from 'react-pdf';

// Use CDN to avoid bundling worker
pdfjs.GlobalWorkerOptions.workerSrc = 
  `//cdnjs.cloudflare.com/ajax/libs/pdf.js/${pdfjs.version}/pdf.worker.min.js`;
```

### Dynamic Component Import Pattern

```typescript
// Only load PDF viewer when needed
const SideBySideViewer = dynamic(
  () => import('@/components/documents/side-by-side-viewer'),
  { 
    loading: () => <ViewerSkeleton />,
    ssr: false 
  }
);

// Usage in page
{isPdfDocument && (
  <SideBySideViewer pdfFile={pdfUrl} markdown={content} />
)}
```

### Bundle Analysis Command

```bash
# Analyze bundle size
pnpm add -D @next/bundle-analyzer

# next.config.ts
const withBundleAnalyzer = require('@next/bundle-analyzer')({
  enabled: process.env.ANALYZE === 'true',
});

module.exports = withBundleAnalyzer(nextConfig);

# Run analysis
ANALYZE=true pnpm build
```

**Status**: ✅ VERIFIED - Bundle optimization applied
