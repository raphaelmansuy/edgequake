# Iteration 25: Failed Documents Highlight - Decide

## Decision

### Add Visual Emphasis to Failed Rows
Use red left border and subtle background tint.

### Code Change
```tsx
<TableRow 
  className={cn(
    "cursor-pointer transition-colors duration-150",
    "hover:bg-primary/5 dark:hover:bg-primary/10",
    selectedDocument?.id === doc.id && "...",
    index % 2 === 0 ? "bg-background" : "bg-muted/20",
    // OODA-25: Failed documents highlight
    doc.status === 'failed' && "bg-red-50/50 dark:bg-red-950/20 border-l-4 border-l-red-500"
  )}
>
```

### Visual Effect
| Theme | Background | Border |
|-------|------------|--------|
| Light | Subtle red tint | 4px solid red |
| Dark | Subtle red tint | 4px solid red |

### Rationale
- Immediately identifies failed documents
- Non-distracting for completed documents
- Consistent with red = error convention
