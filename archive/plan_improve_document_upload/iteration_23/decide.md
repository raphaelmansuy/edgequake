# Iteration 23: Processing Status Summary Card - Decide

## Decision

### Implement Compact Status Bar

Add conditional status bar between filters and upload zone.

### Visual Design

- Blue theme (bg-blue-50 dark:bg-blue-950/30)
- Blue border for definition
- Rounded corners (rounded-lg)
- Compact padding (px-3 py-2)

### Content Layout

```
[Spinner] Processing X document(s)    [Clock] Y queued  [Check] Z done  Click for details →
```

### Accessibility

- `role="button"` for screen readers
- `tabIndex={0}` for keyboard focus
- `onKeyDown` handler for Enter key

### Code Location

After search/filters section, before upload zone.
