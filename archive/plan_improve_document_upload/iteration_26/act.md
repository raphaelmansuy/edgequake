# Iteration 26: Document Count in Page Title - Act

## Implementation Complete ✅

### Changes Made

Added useEffect to update document.title with document count and processing status.

### Code Added

```tsx
useEffect(() => {
  const baseTitle = "Documents - EdgeQuake";
  const count = totalCount || 0;
  const processing = pipelineStatus?.running_tasks || 0;

  if (processing > 0) {
    document.title = `⏳ Processing (${processing}) | Documents (${count}) - EdgeQuake`;
  } else if (count > 0) {
    document.title = `Documents (${count}) - EdgeQuake`;
  } else {
    document.title = baseTitle;
  }

  return () => {
    document.title = baseTitle;
  };
}, [totalCount, pipelineStatus?.running_tasks]);
```

### Title Formats

| State      | Title Example                                   |
| ---------- | ----------------------------------------------- |
| Empty      | Documents - EdgeQuake                           |
| With docs  | Documents (42) - EdgeQuake                      |
| Processing | ⏳ Processing (3) \| Documents (42) - EdgeQuake |

### Verification

- ✅ TypeScript compilation: No errors

### UX Benefits

- See document count without switching tabs
- Processing indicator in tab title
- Know system status at a glance
