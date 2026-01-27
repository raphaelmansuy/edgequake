# Iteration 25: Failed Documents Highlight - Act

## Implementation Complete ✅

### Changes Made

1. **document-manager.tsx**:
   - Added conditional failed styling to TableRow

### Code Added

```tsx
// OODA-25: Failed documents highlight
doc.status === "failed" &&
  "bg-red-50/50 dark:bg-red-950/20 border-l-4 border-l-red-500";
```

### Visual Styling

| Element      | Light Mode         | Dark Mode          |
| ------------ | ------------------ | ------------------ |
| Background   | `bg-red-50/50`     | `bg-red-950/20`    |
| Left Border  | `border-l-red-500` | `border-l-red-500` |
| Border Width | 4px                | 4px                |

### Verification

- ✅ TypeScript compilation: No errors
- ✅ Unit tests: 29 passed

### UX Benefits

- Failed documents immediately visible
- Red left border draws attention
- Subtle background doesn't overwhelm
- Works in both light and dark modes
- Consistent with error color conventions

### Before vs After

| State      | Before                    | After                  |
| ---------- | ------------------------- | ---------------------- |
| Failed     | Blends in with other rows | Red left border + tint |
| Completed  | Normal                    | Normal (unchanged)     |
| Processing | Normal                    | Normal (unchanged)     |

## Next Iteration

**Iteration 26: Document Count in Page Title**
Show document count in browser tab title.
