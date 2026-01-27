# Iteration 25: Failed Documents Highlight - Observe

## Current State Analysis

### Current Row Styling
```tsx
<TableRow 
  className={cn(
    "cursor-pointer transition-colors duration-150",
    "hover:bg-primary/5 dark:hover:bg-primary/10",
    selectedDocument?.id === doc.id && "bg-primary/10 dark:bg-primary/15 ring-1 ring-primary/20",
    index % 2 === 0 ? "bg-background" : "bg-muted/20"
  )}
>
```

### User Pain Point
- Failed documents blend in with others
- Hard to spot failures at a glance
- No visual urgency for documents needing attention

### Enhancement Opportunity
Add visual emphasis to failed documents:
- Red/orange left border
- Subtle background tint
- Error icon in row

### Design Options
1. **Left border**: 3px solid red border
2. **Background tint**: Very subtle red background
3. **Row icon**: Small error indicator

### Selected Approach
Use left border + subtle background for failed rows.
