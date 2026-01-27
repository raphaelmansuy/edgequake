# Iteration 25: Failed Documents Highlight - Orient

## Analysis

### Visual Treatment Selected
- Red left border (4px solid)
- Subtle red background tint
- Works with existing styling

### Tailwind Classes
```
border-l-4 border-l-red-500  // Left border
bg-red-50/50                  // Light mode background
dark:bg-red-950/20            // Dark mode background
```

### Integration with Existing Styles
Added to `cn()` call with conditional:
```tsx
doc.status === 'failed' && "bg-red-50/50 dark:bg-red-950/20 border-l-4 border-l-red-500"
```

### Visual Priority
- Overrides zebra striping for failed rows
- Maintains selection highlight priority
- Hover state still works

## Risk Assessment
- Low risk: CSS-only change
- Works with dark mode
- Non-disruptive to existing styles
