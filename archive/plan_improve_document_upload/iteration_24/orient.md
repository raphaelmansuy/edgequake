# Iteration 24: Document Sort Persistence - Orient

## Analysis

### Storage Key

`edgequake:documents:sort`

### Data Structure

```typescript
interface SortPreference {
  field: SortField;
  direction: SortDirection;
}
```

### Implementation Options

1. **Direct localStorage** - Simple, inline
2. **Custom hook** - Reusable pattern
3. **Zustand persist** - If we had Zustand for this

### Selected Approach: Direct localStorage

- Minimal overhead
- Single use case
- Easy to understand

### Code Structure

```tsx
// Initialize from localStorage
const [sortField, setSortField] = useState<SortField>(() => {
  if (typeof window === "undefined") return "created_at";
  const stored = localStorage.getItem("edgequake:documents:sort");
  if (stored) {
    try {
      return JSON.parse(stored).field || "created_at";
    } catch {
      return "created_at";
    }
  }
  return "created_at";
});

// Effect to persist changes
useEffect(() => {
  localStorage.setItem(
    "edgequake:documents:sort",
    JSON.stringify({
      field: sortField,
      direction: sortDirection,
    }),
  );
}, [sortField, sortDirection]);
```

## Risk Assessment

- Low risk: localStorage is well-supported
- Graceful fallback to defaults
- No backend changes needed
