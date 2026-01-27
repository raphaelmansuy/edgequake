# Iteration 21: Document Preview Error Enhancement - Orient

## Analysis

### Error Categorization Integration

Reuse the error-categories.ts utility created in OODA-09:

- categorizeError(message) → category info
- getCategoryIcon(category) → appropriate icon
- getCategoryColor(category) → Tailwind color classes
- getCategorySuggestions(category) → helpful hints

### Enhancement Plan

1. Import categorization utilities
2. Replace raw error display with categorized view
3. Show category-specific icon and color
4. Display suggestions when available
5. Add prominent retry button for retryable errors

### Code Structure

```tsx
// Get error categorization
const errorInfo = useMemo(
  () =>
    document.error_message ? categorizeError(document.error_message) : null,
  [document.error_message],
);

// Render enhanced error section
{
  isFailed && errorInfo && (
    <div className="space-y-2">
      <h4
        className={`text-sm font-medium flex items-center gap-1.5 ${getCategoryColor(errorInfo.category)}`}
      >
        {getCategoryIcon(errorInfo.category)} {/* Icon based on category */}
        {errorInfo.category.replace("_", " ")} Error
      </h4>
      <p>{document.error_message}</p>
      {errorInfo.suggestions?.length > 0 && (
        <ul>
          {errorInfo.suggestions.map((s) => (
            <li>{s}</li>
          ))}
        </ul>
      )}
      {errorInfo.retryable && onReprocess && (
        <Button onClick={onReprocess}>Retry</Button>
      )}
    </div>
  );
}
```

## Risk Assessment

- Low risk: Visual enhancement only
- Reuses tested utility
- Easy to verify visually
