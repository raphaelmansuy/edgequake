# Iteration 22: Document List Quick Actions - Decide

## Decision

### Add Three Quick Action Buttons

1. **Preview** (all documents) - Eye icon with tooltip
2. **View in Graph** (completed/indexed) - Sparkles icon with tooltip
3. **Retry** (failed) - RefreshCw icon with tooltip, orange color

### Code Changes

1. Add Tooltip imports from @/components/ui/tooltip
2. Wrap preview button with TooltipProvider and Tooltip
3. Add conditional View in Graph button
4. Add conditional Retry button

### Button Configuration

```tsx
// Preview (always)
<Button variant="ghost" size="icon" className="h-8 w-8">
  <Eye className="h-4 w-4" />
</Button>;

// View in Graph (completed/indexed)
{
  (doc.status === "completed" || doc.status === "indexed") && (
    <Button variant="ghost" size="icon" className="h-8 w-8">
      <Sparkles className="h-4 w-4" />
    </Button>
  );
}

// Retry (failed)
{
  doc.status === "failed" && (
    <Button
      variant="ghost"
      size="icon"
      className="h-8 w-8 text-orange-600 hover:text-orange-700 hover:bg-orange-50"
    >
      <RefreshCw className="h-4 w-4" />
    </Button>
  );
}
```

### Rationale

- Reduces clicks for common operations
- Failed documents get prominent retry option
- Completed documents can quickly jump to graph
