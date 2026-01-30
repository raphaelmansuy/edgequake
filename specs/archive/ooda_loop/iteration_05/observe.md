# Iteration 05 - OBSERVE Phase

## Objective

Enhance error display with copy-to-clipboard and expandable details

## Current Error Display Analysis

### document-manager.tsx (lines 979-986)

Current implementation:

```tsx
{
  doc.status === "failed" && doc.error_message && (
    <span className="text-xs text-red-500 dark:text-red-400 flex items-center gap-1">
      <AlertCircle className="h-3 w-3" />
      <span className="truncate max-w-[200px]" title={doc.error_message}>
        {doc.error_message}
      </span>
    </span>
  );
}
```

**Issues**:

1. Error message truncated to 200px - long errors are hidden
2. No way to copy error message for debugging
3. No expansion to see full error
4. Title attribute shows full error on hover, but not mobile-friendly
5. No structured error parsing (stack traces, error codes)

### Error Message Types Observed

From backend (processor.rs):

1. **LLM Errors**: "Failed to extract entities: API rate limit exceeded"
2. **Embedding Errors**: "Embedding failed: dimension mismatch (1536 vs 768)"
3. **Storage Errors**: "Failed to store vectors: connection timeout"
4. **Parsing Errors**: "Failed to parse document: unsupported format"

## Enhancement Goals

1. **Copy Button**: One-click copy error to clipboard
2. **Expandable Details**: Click to see full error
3. **Error Categories**: Visual indication of error type
4. **Retry Information**: Show retry count if applicable
5. **Timestamp**: When the error occurred
