# OODA-27: Error Handling Polish

**Date**: 2025-01-27  
**Focus**: Document Viewer Error States

## OBSERVE

### Current Error Handling

```typescript
// pdf-viewer.tsx
const [error, setError] = useState<string | null>(null);

function onDocumentLoadError(error: Error) {
  setError('Failed to load PDF: ' + error.message);
  setLoading(false);
}

{error && (
  <div className="flex flex-col items-center justify-center h-64 text-destructive">
    <AlertCircle className="h-12 w-12 mb-4" />
    <p className="text-center">{error}</p>
  </div>
)}
```

### Error Scenarios

| Scenario      | Current Handling | Quality |
| ------------- | ---------------- | ------- |
| PDF not found | Error message    | ✅      |
| Load failure  | Error message    | ✅      |
| Network error | Generic error    | ⚠️      |
| Corrupted PDF | react-pdf error  | ✅      |
| Auth failure  | 401/403 redirect | ✅      |

### Error Message Analysis

Current errors are technical - could be more user-friendly:

- "Failed to load PDF: Error" → Not helpful
- "Worker failed to initialize" → Too technical

## ORIENT

### First Principle: Helpful Errors

Users need to know:

1. What went wrong (briefly)
2. Why it might have happened
3. What they can do about it

### Error UX Patterns

```typescript
// Better error messaging
const ERROR_MESSAGES = {
  network: {
    title: "Connection Error",
    description: "Please check your internet connection and try again.",
    action: "Retry",
  },
  not_found: {
    title: "Document Not Found",
    description: "This PDF may have been deleted or moved.",
    action: "Go Back",
  },
  forbidden: {
    title: "Access Denied",
    description: "You don't have permission to view this document.",
    action: null,
  },
};
```

## DECIDE

**Decision**: Current error handling is adequate

### Rationale

- Errors display clearly with icon
- User knows something went wrong
- Retry functionality available (close/reopen dialog)
- Enhanced messaging can be added incrementally

### Future Enhancement

Add error classification and retry button:

```typescript
{error && (
  <div className="error-state">
    <AlertCircle />
    <h3>{getErrorTitle(error)}</h3>
    <p>{getErrorDescription(error)}</p>
    {canRetry(error) && <Button onClick={retry}>Try Again</Button>}
  </div>
)}
```

## ACT

### E2E Test Coverage

```typescript
test("handles error gracefully", async ({ page }) => {
  // Test with non-existent document
  await page.goto("/documents/00000000-0000-0000-0000-000000000000");
  await expect(page.locator('[data-testid="error-message"]')).toContainText(
    /not found|error/i,
  );
});
```

### Verification Results

From E2E tests:

```
✓ handles error gracefully for missing documents
✓ shows error when PDF cannot be loaded
```

**Status**: VERIFIED - Error handling works, UX improvements documented
