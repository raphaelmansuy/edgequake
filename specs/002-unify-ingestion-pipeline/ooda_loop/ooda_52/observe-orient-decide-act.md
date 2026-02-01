# OODA-52: Error Boundary Implementation

**Date**: 2026-02-01
**Focus**: React Error Boundaries for PDF Rendering

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Graceful error handling
- User-friendly error messages
- No blank screens on failure

### Current Error Handling

**PDFViewer Error State:**
```typescript
const [error, setError] = useState<Error | null>(null);

<Document
  onLoadError={(error) => setError(error)}
  error={<PDFError error={error} />}
>
```

**Missing: React Error Boundary**
If rendering crashes, entire app may fail.

## ORIENT

### Error Scenarios

| Scenario | Current Behavior | Expected |
|----------|------------------|----------|
| PDF load fail | Shows error message | ✅ Good |
| PDF corrupt | Shows error message | ✅ Good |
| React render crash | White screen | ❌ Bad |
| Network timeout | Shows loading forever | ⚠️ Needs timeout |

### Error Boundary Placement
```
<ErrorBoundary fallback={<PDFErrorFallback />}>
  <PDFViewer file={url} />
</ErrorBoundary>
```

## DECIDE

**Decision**: Add React Error Boundary wrapper

Implementation:
1. Create reusable ErrorBoundary component
2. Wrap PDFViewer in boundary
3. Provide retry mechanism

## ACT

### ErrorBoundary Component

**File:** `edgequake_webui/src/components/error-boundary.tsx`

```typescript
class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('ErrorBoundary caught:', error, errorInfo);
    // Could report to error tracking service
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback || (
        <div className="p-4 text-red-500">
          <p>Something went wrong.</p>
          <button onClick={() => this.setState({ hasError: false })}>
            Retry
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
```

### Usage Pattern
```typescript
<ErrorBoundary fallback={<PDFLoadError onRetry={reload} />}>
  <PDFViewer file={pdfUrl} />
</ErrorBoundary>
```

**Status**: 📋 DOCUMENTED - Implementation pattern ready
